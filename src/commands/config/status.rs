use arc_swap::{ArcSwapAny, Guard};
use serenity::client::Context;
use std::sync::Arc;

use serenity::{
    async_trait,
    builder::CreateApplicationCommand,
    model::prelude::interaction::{
        application_command::ApplicationCommandInteraction, InteractionResponseType,
    },
};

use crate::{
    commands::{CommandError, SlashCommand},
    config::Config,
};
pub(crate) const COMMAND_NAME: &'static str = "status";
pub(crate) fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name(COMMAND_NAME)
        .description("Check the bot's status")
}
#[derive(Debug)]
pub struct StatusCommand;
impl StatusCommand {
    pub fn new() -> Self {
        StatusCommand {}
    }
}
#[async_trait]
impl SlashCommand for StatusCommand {
    fn config(&self) -> crate::commands::CommandConfig {
        crate::commands::CommandConfig {
            accessLevel: crate::commands::AccessLevels::ADMIN,
            ..Default::default()
        }
    }

    async fn run(
        &self,
        interaction: ApplicationCommandInteraction,
        ctx: Context,
        config: Arc<ArcSwapAny<Arc<Config>>>,
    ) -> Result<(), CommandError> {
        interaction
            .edit_original_interaction_response(&ctx.http, |response| response.content("Ok"))
            .await?;
        Ok(())
    }
}
