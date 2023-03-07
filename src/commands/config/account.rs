use arc_swap::{ArcSwap, ArcSwapAny, Guard};
use binance::account::Account;
use diesel::RunQueryDsl;
use serenity::{client::Context, model::prelude::{component::ButtonStyle, command::CommandOptionType, interaction::application_command::CommandDataOption}};
use std::{sync::{Arc}, thread, time::Duration};
use tokio::sync::RwLock;
use tracing::{debug, warn, trace, error};

use serenity::{
    async_trait, builder::CreateApplicationCommand,
    model::prelude::interaction::application_command::ApplicationCommandInteraction,
};

use crate::{
    binance_wrapped::BinanceWrapped,
    commands::{CommandError, SlashCommand, AutoComplete},
    config::{Config, ValueType}, utils::get_option::get_option, db::establish_connection, ops::config_ops, models::{self, BinanceAccount},
};

pub(crate) const COMMAND_NAME: &'static str = "account";
pub(crate) fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
    .name(COMMAND_NAME)
    .description("Group of Commands to modify Binance Accounts")
    .create_option(|opt|
        opt.kind(CommandOptionType::SubCommand)
        .name("create")
        .description("create a binance account")  
        .create_sub_option(|sub_opt|
            sub_opt.name("account_name")
            .description("The custom name to recognize this account in the db")
            .kind(CommandOptionType::String)
            .required(true)
        )
        .create_sub_option(|sub_opt|
            sub_opt.name("account_api")
            .description("The Binance Account Api Key")
            .kind(CommandOptionType::String)
            .required(true)
        )
        .create_sub_option(|opt|
            opt.name("account_secret")
            .description("The Binance Secret")
            .kind(CommandOptionType::String)
            .required(true)
        )
        .create_sub_option(|opt|
            opt.name("is_paper")
            .description("Is this a paper Account")
            .default_option(false)
            .kind(CommandOptionType::Boolean)
        ) 
    )
    .create_option(|opt|
        opt.kind(CommandOptionType::SubCommand)
        .name("set")
        .description("set the binance account")  
        .create_sub_option(|sub_opt|
            sub_opt.name("account_name")
            .description("The custom name to recognize this account in the db")
            .kind(CommandOptionType::String)
            .required(true)
            .set_autocomplete(true)
        )
        
    )
    .create_option(|opt|
        opt.kind(CommandOptionType::SubCommand)
        .name("delete")
        .description("delete a binance account")  
        .create_sub_option(|sub_opt|
            sub_opt.name("account_name")
            .description("The custom name to recognize this account in the db")
            .kind(CommandOptionType::String)
            .required(true)
            .set_autocomplete(true)
        )
        
    )
}




pub struct AccountCommand {
    binance: Arc<RwLock<BinanceWrapped>>
}

impl AccountCommand {
    pub fn new(binance: Arc<RwLock<BinanceWrapped>>) -> Self {
        AccountCommand {binance }
    }
}
#[async_trait]
impl SlashCommand for AccountCommand {
    fn config(&self) -> crate::commands::CommandConfig {
        crate::commands::CommandConfig {
            accessLevel: crate::commands::AccessLevels::ADMIN,
            ephermal: true,
            ..Default::default()
        }
    }


    async fn run(
        &self,
        interaction: ApplicationCommandInteraction,
        ctx: Context,
        config_swap: Arc<ArcSwapAny<Arc<Config>>>,
    ) -> Result<(), CommandError> {
        let options:&Vec<CommandDataOption> = interaction.data.options.as_ref();

        if let Some(sub_command) = options.iter().find(|opt| opt.name == "create"){ 
            debug!("Running create sub command");
            let options:&Vec<CommandDataOption> = sub_command.options.as_ref();
            let name = get_option::<String>(&mut options.iter(), "account_name")?;
            let api = get_option::<String>(&mut options.iter(), "account_api")?;
            let secret = get_option::<String>(&mut options.iter(), "account_secret")?;
            let is_paper = get_option::<bool>(&mut options.iter(), "is_paper").unwrap_or(false);
            debug!("Executing create_account Command");
            use crate::schema::binance_accounts::dsl;
            let mut connection = establish_connection();
            trace!("Recieved Options {name} {api} {secret} {is_paper}");
            diesel::insert_into(dsl::binance_accounts).values(crate::models::NewBinanceAccount{
                name,
                api_key: api,
                secret,
                is_paper,
            }).execute(&mut connection)?;
            interaction.edit_original_interaction_response(&ctx.http, |i|
                i.content("Account Created succesfully")
            ).await?;
        }else if let Some(sub_command) = options.iter().find(|opt| opt.name == "set"){
            debug!("Running set sub command");
            let mut options = sub_command.options.iter();
            let name = get_option::<String>(&mut options, "account_name")?;
            config_ops::handle(config_ops::Operations::UpdateConfig(models::UpdateConfig{
                section: "trading".into(),
                key: "account_name".into(),
                value: Some(name)
            }))?;

            config_swap.store(Arc::from(Config::load()?));
            {
                let mut binance = self.binance.write().await;
                binance.load_account()?;
            }
            interaction.edit_original_interaction_response(&ctx.http, |i|
                i.content("Account Set succesfully")
            ).await?;
        }else if let Some(delete_command) = options.iter().find(|opt| opt.name == "delete"){
            debug!("Running set delete command");
            let mut options = delete_command.options.iter();
            let mut connection = establish_connection();

            let name = get_option::<String>(&mut options, "account_name")?;
            use diesel::ExpressionMethods;
            use crate::schema::binance_accounts::dsl;
            diesel::delete(dsl::binance_accounts).filter(dsl::name.eq(name)).execute(&mut connection)?;

            interaction.edit_original_interaction_response(&ctx.http, |i|
                i.content("Account Delete succesfully")
            ).await?;
        }else{
            error!("No sub command found");
        }

        Ok(())
    }
}
#[async_trait]
impl AutoComplete for AccountCommand {
    async fn auto_complete(&self,interaction: serenity::model::prelude::interaction::autocomplete::AutocompleteInteraction,
        ctx: Context,
        config: Arc<Config>,) -> Result<(),CommandError>{

        let mut connection = establish_connection();
        use crate::schema::binance_accounts::dsl;
        let accounts = dsl::binance_accounts.load::<BinanceAccount>(&mut connection)?;
        let options:&Vec<CommandDataOption> = interaction.data.options.as_ref();

        let Some(set_option) = options.iter().find(|a| a.name == "set" || a.name =="delete") else {
            trace!("Did not find set sub command for auto_complete");
            return Ok(());
        };
        let mut options = set_option.options.iter();

        let account_name = get_option::<String>(&mut options, "account_name")?;

        let account_names = accounts.iter().filter_map(|a| {
            if a.name.contains(&account_name){
                Some(a.name.clone())
            }else{
                None
            }
        });


        
        interaction
        .create_autocomplete_response(&ctx.http, |a| {
            account_names.take(25).for_each(|str| {
                a.add_string_choice(str.clone(), str);
            });
            a
        })
        .await?;
        
        Ok(())
    }
}
