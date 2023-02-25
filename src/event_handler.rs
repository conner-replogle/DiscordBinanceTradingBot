use arc_swap::ArcSwap;
use binance::account::Account;
use binance::api::Binance;
use std::env;
use std::sync::Arc;
use std::time::Duration;

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

use crate::commands;
use crate::commands::config::create_user::CreateUserCommand;
use crate::commands::config::list_config::ListConfigCommand;
use crate::commands::config::set_config::SetConfigCommand;
use crate::commands::config::status::StatusCommand;
use crate::commands::schedule::reserve::ReserveCommand;
use crate::commands::trading::balance::BalanceCommand;
use crate::commands::trading::buy::BuyCommand;
use crate::commands::trading::price::PriceCommand;
use crate::commands::trading::sell::SellCommand;
use crate::config::{Config, ValueType};
use crate::db::establish_connection;
use crate::utils::message::send_status;

pub struct Handler {
    binance: Account,
    config: Arc<ArcSwap<Config>>,
    market: Market,
}
impl Handler {
    pub fn new(binance: Account, config: Arc<ArcSwap<Config>>, market: Market) -> Self {
        Self {
            binance,
            config,
            market,
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
            commands::config::create_user::COMMAND_NAME => Box::from(CreateUserCommand::new()),
            commands::config::set_config::COMMAND_NAME => Box::from(SetConfigCommand::new()),
            commands::config::list_config::COMMAND_NAME => Box::from(ListConfigCommand::new()),
            commands::schedule::reserve::COMMAND_NAME => Box::from(ReserveCommand::new()),

            commands::trading::price::COMMAND_NAME => {
                Box::from(PriceCommand::new(self.binance.clone(), self.market.clone()))
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

        let command_config = command_runner.config();
        let guild_id = command.guild_id.unwrap();

        match command_config.accessLevel {
            commands::AccessLevels::ADMIN => {
                if let ValueType::BIGINT(Some(role_id)) = config.get("roles", "admin") {
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
                if let ValueType::BIGINT(Some(role_id)) = config.get("roles", "trader") {
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
                    return;
                }
            }
            commands::AccessLevels::ANY => {
                debug!("User authorized")
            }
        }
        let time = match config.get("general", "command_timeout") {
            ValueType::INT(Some(int)) => int,
            val => {
                warn!("config general/command_timeout does not contain a value {val:?}");
                60 * 5
            }
        };

        let config = self.config.clone();
        let command_cl = command.clone();
        let name = command_cl.data.name.clone();
        tokio::spawn(async move {
            if let Err(err) = timeout(Duration::from_secs(time as u64), async move {
                debug!("Running Command");
                if let Err(err) = command_runner.run(command_cl, ctx.clone(), config).await {
                    error!("error executing command {err:?}");
                    if let Err(err) =
                        send_status(&ctx, &command, &format!("command failed {err:?}")).await
                    {
                        error!("Error executing inital command {err:?}")
                    }; //TODO SEND ERROR TO USER
                }
            })
            .await
            {
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
