use arc_swap::ArcSwap;
use binance::account::Account;
use binance::api::Binance;
use std::env;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;

use binance::market::Market;
use dotenv::dotenv;
use serenity::async_trait;
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::model::gateway::Ready;
use serenity::model::id::GuildId;
use serenity::model::prelude::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::prelude::interaction::autocomplete::AutocompleteInteraction;
use serenity::prelude::*;
use tokio::time::timeout;
use tracing::{debug, error, info, instrument, span, trace, warn, Level};
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};

use crate::binance_wrapped::BinanceWrapped;
use crate::commands::config::account::AccountCommand;
use crate::commands::config::create_user::CreateUserCommand;
use crate::commands::config::list_config::ListConfigCommand;
use crate::commands::config::set_config::SetConfigCommand;
use crate::commands::config::status::StatusCommand;
use crate::commands::schedule::clock::ClockCommand;
use crate::commands::schedule::reserve::ReserveCommand;
use crate::commands::trading::balance::BalanceCommand;
use crate::commands::trading::buy::BuyCommand;
use crate::commands::trading::cancel::CancelCommand;
use crate::commands::trading::orders::OrdersCommand;
use crate::commands::trading::price::PriceCommand;
use crate::commands::trading::sell::SellCommand;
use crate::config::{Config, ValueType};
use crate::db::establish_connection;
use crate::utils::message::send_status;
use crate::{commands, interval_handler};

pub struct Handler {
    binance: Arc<RwLock<BinanceWrapped>>,
    config: Arc<ArcSwap<Config>>,
    market: Market,
    is_loop_running: AtomicBool,
}
impl Handler {
    pub fn new(
        binance: Arc<RwLock<BinanceWrapped>>,
        config: Arc<ArcSwap<Config>>,
        market: Market,
    ) -> Self {
        Self {
            binance,
            config,
            market,
            is_loop_running: AtomicBool::new(false),
        }
    }

    #[instrument(skip_all, name = "AutoComplete", level = "trace")]
    async fn auto_complete(&self, ctx: Context, command: AutocompleteInteraction) {
        let config = self.config.load();

        debug!("Recieved Autocomplete for {}", command.data.name);
        
        trace!("Finding command for autocomplete");
        let command_runner: Box<dyn commands::AutoComplete> = match command.data.name.as_str() {
            commands::config::set_config::COMMAND_NAME => Box::from(SetConfigCommand::new()),
            commands::schedule::reserve::COMMAND_NAME => Box::from(ReserveCommand::new()),
            commands::config::account::COMMAND_NAME => Box::from(AccountCommand::new(self.binance.clone())),

            _ => {
                error!("Autocomplete did not exist");
                return;
            } //TODO MAKE HELP COMMAND
        };
        debug!("Running AutoComplete");
        if let Err(err) = command_runner
            .auto_complete(command.clone(), ctx.clone(), config.clone())
            .await
        {
            error!("error executing autocomplete {err:?}");
        }
    }

    #[instrument(skip_all, name = "Command", level = "debug")]
    async fn command(&self, ctx: Context, command: ApplicationCommandInteraction) {
        let config = self.config.load();

        if let Err(err) = command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|message| message.content("Checking permissions..."))
            })
            .await
        {
            error!("Error executing inital message {err:?}")
        };
        trace!("Received command: {:#?} ", command.data.name);

        //TODO MAKE ALL OF THIS DYNAMICALLY GENERATE MACRO ->
        let command_runner: Box<dyn commands::SlashCommand> = match command.data.name.as_str() {
            commands::config::status::COMMAND_NAME => Box::from(StatusCommand::new()),
            commands::trading::balance::COMMAND_NAME => {
                Box::from(BalanceCommand::new(self.binance.clone()))
            }
            commands::trading::buy::COMMAND_NAME => {
                Box::from(BuyCommand::new(self.binance.clone()))
            }
            commands::trading::sell::COMMAND_NAME => {
                Box::from(SellCommand::new(self.binance.clone()))
            }
            commands::config::account::COMMAND_NAME => {
                Box::from(AccountCommand::new(self.binance.clone()))
            } 
            commands::config::create_user::COMMAND_NAME => Box::from(CreateUserCommand::new()),
            commands::config::set_config::COMMAND_NAME => Box::from(SetConfigCommand::new()),
            commands::config::list_config::COMMAND_NAME => Box::from(ListConfigCommand::new()),
            commands::schedule::reserve::COMMAND_NAME => Box::from(ReserveCommand::new()),

            commands::trading::price::COMMAND_NAME => {
                Box::from(PriceCommand::new(self.binance.clone(), self.market.clone()))
            }
            commands::schedule::clock::COMMAND_NAME => {
                Box::from(ClockCommand::new(self.binance.clone()))
            }
            commands::trading::orders::COMMAND_NAME => {
                Box::from(OrdersCommand::new(self.binance.clone()))
            }
            commands::trading::cancel::COMMAND_NAME => {
                Box::from(CancelCommand::new(self.binance.clone()))
            }


            _ => {
                error!("Command did not exist");
                if let Err(err) = send_status(&ctx, &command, "Command did not exist").await {
                    error!("Error executing inital command {err:?}")
                };
                return;
            } //TODO MAKE HELP COMMAND
        };
        // <-
        //TODO CHECK ACCESS LEVEL AND OPTIONS
        trace!("Command Found");
        let command_config = command_runner.config();
        let guild_id = command.guild_id.unwrap();
        trace!("Checking Access Level");

        match command_config.accessLevel {
            commands::AccessLevels::ADMIN => {
                if let Some(role_id) = config.get::<i64>("roles", "admin").unwrap() {
                    if let Ok(true) = command
                        .user
                        .has_role(&ctx.http, guild_id, role_id as u64)
                        .await
                    {
                        debug!("User authorized")
                    } else {
                        if let Err(err) = send_status(
                            &ctx,
                            &command,
                            "Not Authorized for this command: ADMIN ACCESS",
                        )
                        .await
                        {
                            error!("Error executing inital command {err:?}")
                        };
                        debug!("User was not authorized for command");
                        return;
                    }
                } else {
                    if let Err(err) = send_status(
                        &ctx,
                        &command,
                        "Error checkin role make sure the roles are set with /config",
                    )
                    .await
                    {
                        error!("Error executing inital command {err:?}")
                    };
                    warn!("Trading role was not set properly");
                }
            }
            commands::AccessLevels::TRADER => {
                if let Some(role_id) = config.get::<i64>("roles", "trader").unwrap() {
                    if let Ok(true) = command
                        .user
                        .has_role(&ctx.http, guild_id, role_id as u64)
                        .await
                    {
                        debug!("User authorized")
                    } else {
                        if let Err(err) = send_status(
                            &ctx,
                            &command,
                            "Not Authorized for this command: TRADER ACCESS",
                        )
                        .await
                        {
                            error!("Error executing inital command {err:?}")
                        };
                        debug!("User was not authorized for role");
                    }
                } else {
                    if let Err(err) = send_status(
                        &ctx,
                        &command,
                        "Error checkin role make sure the roles are set with /config",
                    )
                    .await
                    {
                        error!("Error executing inital command {err:?}")
                    };
                    warn!("Trading role was not set properly");
                }
            }
            commands::AccessLevels::ANY => {
                debug!("User authorized")
            }
        }
        let time = match config.get("general", "command_timeout").unwrap() {
            Some(int) => int,
            None => 60 * 5,
        };

        let config = self.config.clone();
        let command_cl = command.clone();
        let name = command_cl.data.name.clone();
        trace!("Spinning up command thread and executing thread");

        tokio::spawn(async move {
            let clone_ctx = ctx.clone();
            let clone_cmd = command.clone();

            if let Err(err) = timeout(Duration::from_secs(time as u64), async move {
                debug!("Running Command");
                if let Err(err) = command_runner.run(command_cl, ctx.clone(), config).await {
                    error!("error executing command {err:?}");
                    if let Err(err) =
                        send_status(&ctx.clone(), &clone_cmd.clone(), &format!("command failed {err:?}")).await
                    {
                        error!("Error executing inital command {err:?}")
                    }; //TODO SEND ERROR TO USER
                }
            })
            .await
            {
                if let Err(err) =
                send_status(&(clone_ctx.clone()),&command,"Command Timed Out").await
                {
                    error!("Error executing inital command {err:?}")
                }; 
               
                error!("Command {} Timed out {}", name, err)
            }
        });
    }
}

#[async_trait]
impl EventHandler for Handler {
    #[instrument(skip_all, name = "Interaction", level = "debug")]
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Autocomplete(interaction) = interaction {
            self.auto_complete(ctx, interaction).await;
        } else if let Interaction::ApplicationCommand(command) = interaction {
            self.command(ctx, command).await;
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);
        let ctx = Arc::new(ctx);
        if !self.is_loop_running.load(Ordering::Relaxed) {
            debug!("Starting Interval Thread");
            // We have to clone the Arc, as it gets moved into the new thread.
            let ctx1 = Arc::clone(&ctx);
            let config1 = Arc::clone(&self.config);
            let binance1 = Arc::clone(&self.binance);

            // tokio::spawn creates a new green thread that can run in parallel with the rest of
            // the application.
            tokio::spawn(async move {
                interval_handler::run(Arc::clone(&ctx1), config1,binance1).await;
            });

            // Now that the loop is running, we set the bool to true
            self.is_loop_running.swap(true, Ordering::Relaxed);
        }
        let guild_id = GuildId(
            env::var("GUILD_ID")
                .expect("Expected GUILD_ID in environment")
                .parse()
                .expect("GUILD_ID must be an integer"),
        );

        match GuildId::set_application_commands(&guild_id, &ctx.http, |commands| {
            commands
                .create_application_command(|command| commands::config::status::register(command))
                .create_application_command(|command| commands::trading::balance::register(command))
                .create_application_command(|command| commands::trading::sell::register(command))
                .create_application_command(|command| commands::trading::buy::register(command))
                .create_application_command(|command| {
                    commands::config::create_user::register(command)
                })
                .create_application_command(|command| {
                    commands::config::set_config::register(command)
                })
                .create_application_command(|command| {
                    commands::config::list_config::register(command)
                })
                .create_application_command(|command| commands::trading::price::register(command))
                .create_application_command(|command| {
                    commands::schedule::reserve::register(command)
                })
                .create_application_command(|command| commands::config::account::register(command))
                .create_application_command(|command| commands::schedule::clock::register(command))
                .create_application_command(|command| commands::trading::orders::register(command))
                .create_application_command(|command| commands::trading::cancel::register(command))


        })
        .await
        {
            Ok(commands) => commands.iter().for_each(|command| {
                let options: Vec<String> = command
                    .options
                    .iter()
                    .map(|option| {
                        return format!(
                            "Name: {} Description: {} Type:{:?}",
                            option.name, option.description, option.kind
                        );
                    })
                    .collect();
                debug!(
                    "Registered Command: {} Options: {:?}",
                    command.name, options
                );
            }),
            Err(err) => {
                error!("Error was thrown registering commands {err}")
            }
        }

        // let guild_command = Command::create_global_application_command(&ctx.http, |command| {
        //     commands::wonderful_command::register(command)
        // })
        // .await;

        // println!("I created the following global slash command: {:#?}", guild_command);
    }
}
