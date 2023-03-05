use arc_swap::ArcSwapAny;
use diesel::RunQueryDsl;
use serenity::{
    builder::CreateEmbed,
    client::Context,
    model::prelude::{
        command::CommandOptionType, interaction::autocomplete::AutocompleteInteraction, Embed,
    },
};
use std::sync::Arc;
use tracing::trace;

use serenity::{
    async_trait,
    builder::CreateApplicationCommand,
    model::prelude::interaction::{
        application_command::ApplicationCommandInteraction, InteractionResponseType,
    },
};

use crate::{
    commands::{AutoComplete, CommandConfig, CommandError, SlashCommand},
    config::{Config, ValueType},
    models::{self},
    ops::config_ops::{self, Operations},
    utils::get_option::get_option,
};
pub(crate) const COMMAND_NAME: &'static str = "list_config";
pub(crate) fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name(COMMAND_NAME).description("List Configuration")
}
#[derive(Debug)]
pub struct ListConfigCommand;
impl ListConfigCommand {
    pub fn new() -> Self {
        ListConfigCommand {}
    }
}

#[async_trait]
impl SlashCommand for ListConfigCommand {
    fn config(&self) -> CommandConfig {
        CommandConfig {
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
        let config_load = config.load();
        let cache = &config_load.cached;
        let embeds: Vec<CreateEmbed> = cache
            .iter()
            .map(|(key, value)| {
                let mut embed = CreateEmbed::default();
                embed.title(key);
                value.iter().for_each(|(key, value)| {
                    embed.field(key, format!("{value:#?}"), false);
                });
                embed
            })
            .collect();
        interaction
            .edit_original_interaction_response(&ctx.http, |m| m.add_embeds(embeds))
            .await?;

        Ok(())
    }
}
