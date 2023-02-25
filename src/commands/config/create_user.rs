use arc_swap::{ArcSwapAny, Guard};
use binance::account::Account;
use serenity::{
    client::Context,
    model::prelude::{
        command::CommandOptionType,
        interaction::application_command::{CommandDataOption, CommandDataOptionValue},
    },
};
use std::{option, sync::Arc};
use tracing::debug;

use serenity::{
    async_trait,
    builder::CreateApplicationCommand,
    model::prelude::interaction::{
        application_command::ApplicationCommandInteraction, InteractionResponseType,
    },
};

use crate::{config::Config, models::NewUser, ops::user_ops};

use crate::commands::{CommandError, SlashCommand};
pub(crate) const COMMAND_NAME: &'static str = "create_user";
pub(crate) fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name(COMMAND_NAME)
        .description("add a user to the database")
        .create_option(|option| {
            option
                .kind(CommandOptionType::User)
                .name("user")
                .description("The user to add to the database")
                .required(true)
        })
}

pub struct CreateUserCommand {}
impl CreateUserCommand {
    pub fn new() -> Self {
        CreateUserCommand {}
    }
}
#[async_trait]
impl SlashCommand for CreateUserCommand {
    fn config(&self) -> crate::commands::CommandConfig {
        crate::commands::CommandConfig {
            accessLevel: crate::commands::AccessLevels::TRADER,
            ..Default::default()
        }
    }

    async fn run(
        &self,
        interaction: ApplicationCommandInteraction,
        ctx: Context,
        config: Arc<ArcSwapAny<Arc<Config>>>,
    ) -> Result<(), CommandError> {
        debug!("Executing Create User Command");
        let Some(option) = interaction.data.options.get(0) else{
            return Err(CommandError::ParsingDataError("Did not recieve a user".into()));
        };
        let Some(resolved) = option.resolved.as_ref() else{
            return Err(CommandError::ParsingDataError("Did not recieve a user".into()));
        };
        if let CommandDataOptionValue::User(user, _) = resolved {
            let tag = &user.tag();
            user_ops::handle(user_ops::Operations::CreateUser(NewUser {
                id: user.id.0 as i64,
                tag,
            }))?;

            interaction
                .edit_original_interaction_response(&ctx.http, |response| {
                    response.content(format!("{tag} User Created"))
                })
                .await?;
        }

        Ok(())
    }
}
