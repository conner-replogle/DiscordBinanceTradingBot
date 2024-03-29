use arc_swap::ArcSwapAny;
use serenity::{
    client::Context,
    model::prelude::{
        command::CommandOptionType, interaction::autocomplete::AutocompleteInteraction,
    },
};
use std::sync::Arc;
use tracing::{error, trace};

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
    models,
    ops::config_ops::{self, Operations},
    utils::get_option::get_option,
};
pub(crate) const COMMAND_NAME: &'static str = "set_config";
pub(crate) fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name(COMMAND_NAME)
        .description("Set Configuration")
        .create_option(|option| {
            option
                .kind(CommandOptionType::String)
                .name("section")
                .description("Config Section Name SECTION/KEY")
                .required(true)
                .set_autocomplete(true)
        })
        .create_option(|option| {
            option
                .kind(CommandOptionType::String)
                .name("key")
                .description("Config Key Name SECTION/KEY")
                .required(true)
                .set_autocomplete(true)
        })
        .create_option(|option| {
            option
                .kind(CommandOptionType::String)
                .name("value")
                .required(true)
                .description("value that you would like to set")
        })
}
#[derive(Debug)]
pub struct SetConfigCommand;
impl SetConfigCommand {
    pub fn new() -> Self {
        SetConfigCommand {}
    }
}

#[async_trait]
impl SlashCommand for SetConfigCommand {
    fn config(&self) -> CommandConfig {
        CommandConfig {
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
        let config_load = config.load();
        let mut options_iter = interaction.data.options.iter();

        let Ok(section) = get_option::<String>(&mut options_iter,"section") else {
            return Err(CommandError::IncorrectParameters("Section".into()));
        };
        let Ok(key) = get_option::<String>(&mut options_iter,"key") else {
            return Err(CommandError::IncorrectParameters("Section".into()));
        };
        let Ok(value) = get_option::<String>(&mut options_iter,"value") else {
            return Err(CommandError::IncorrectParameters("Value".into()));
        };

        trace!("Recieved Options Section:{section} Key:{key} Value:{value}");
        if value == "None"{
            config_ops::handle(Operations::UpdateConfig(models::UpdateConfig {
                section: section.clone(),
                key: key.clone(),
                value: None,
            }))?;
            interaction.edit_original_interaction_response(&ctx.http, |response| {
                response.content(format!("Updated Config at {section}/{key} to None"))
            }).await?;
        }else{
        match config_load.get_type(&section, &key)? {
            ValueType::STRING => {
                let old_value = config_load.get::<String>(&section, &key)?;

                let Ok(value) = value.parse::<String>() else{
                    return Err(CommandError::IncorrectParameters("Expected String".into()));
                };
                //update config
                config_ops::handle(Operations::UpdateConfig(models::UpdateConfig {
                    section: section.clone(),
                    key: key.clone(),
                    value: Some(value.clone()),
                }))?;
                interaction.edit_original_interaction_response(&ctx.http, |response| {
                    response.content(format!("Updated Config at {section}/{key} previous:{old_value:?} new:{value:?}"))
                }).await?;
            }
            ValueType::INT => {
                let old_value = config_load.get::<i32>(&section, &key)?;

                let Ok(value) = value.parse::<i32>() else{
                    return Err(CommandError::IncorrectParameters("Expected int".into()));
                };
                //update config
                config_ops::handle(Operations::UpdateConfig(models::UpdateConfig {
                    section: section.clone(),
                    key: key.clone(),
                    value: Some(value.to_string()),
                }))?;
                interaction.edit_original_interaction_response(&ctx.http, |response| {
                    response.content(format!("Updated Config at {section}/{key} previous:{old_value:?} new:{value:?}"))
                }).await?;
            }
            ValueType::BIGINT => {
                let old_value = config_load.get::<i64>(&section, &key)?;

                let Ok(value) = value.parse::<i64>() else{
                    return Err(CommandError::IncorrectParameters("Expected bigint".into()));
                };
                //update config
                config_ops::handle(Operations::UpdateConfig(models::UpdateConfig {
                    section: section.clone(),
                    key: key.clone(),
                    value: Some(value.to_string()),
                }))?;
                interaction.edit_original_interaction_response(&ctx.http, |response| {
                    response.content(format!("Updated Config at {section}/{key} previous:{old_value:?} new:{value:?}"))
                }).await?;
            }
            ValueType::BOOL => {
                let old_value = config_load.get::<bool>(&section, &key)?;

                let Ok(value) = value.parse::<bool>() else{
                    return Err(CommandError::IncorrectParameters("Expected bool".into()));
                };
                //update config
                config_ops::handle(Operations::UpdateConfig(models::UpdateConfig {
                    section: section.clone(),
                    key: key.clone(),
                    value: Some(value.to_string()),
                }))?;
                interaction.edit_original_interaction_response(&ctx.http, |response| {
                    response.content(format!("Updated Config at {section}/{key} previous:{old_value:?} new:{value:?}"))
                }).await?;
            }
        }
        }   
        match Config::load() {
            Ok(con) => config.store(Arc::from(con)),
            Err(err) => {
                interaction
                    .edit_original_interaction_response(&ctx.http, |response| {
                        response.content(format!(
                            "Successfully updated value but failed to load config back {err}"
                        ))
                    })
                    .await?;
                error!("Successfully updated value but failed to load config back {err}");
                return Ok(());
            }
        }

        Ok(())
    }
}
#[async_trait]
impl AutoComplete for SetConfigCommand {
    async fn auto_complete(
        &self,
        interaction: AutocompleteInteraction,
        ctx: Context,
        config: Arc<Config>,
    ) -> Result<(), CommandError> {
        //debug!("AutoComplete Interaction {:#?}",interaction.data);

        let Some(focused) = interaction.data.options.iter().find(|o| o.focused) else{
            return  Ok(());
        };
        let options;
        if focused.name == "section" {
            options = config.cached.keys().collect::<Vec<&String>>()
        } else if focused.name == "key" {
            let Some(option) = interaction.data.options.iter().find(|o| o.name == "section") else{
                return Ok(())
            };
            let Some(s_value) = option.value.as_ref() else{
                return Ok(())
            };
            let Some(section) = s_value.as_str() else{
                return Ok(())
            };
            let Some(map) = config.cached.get(section) else {
                return Ok(())
            };
            options = map.keys().collect::<Vec<&String>>()
        } else {
            return Ok(());
        }
        let options_iter = options.iter();
        let filtered_options = options_iter.filter(|o| {
            if let Some(s_value) = focused.value.as_ref() {
                let Some(str) = s_value.as_str() else{
                    return true;
                };
                return o.contains(str);
            }
            return true;
        });

        interaction
            .create_autocomplete_response(&ctx.http, |a| {
                filtered_options.for_each(|o| {
                    a.add_string_choice(o, o);
                });
                a
            })
            .await?;

        Ok(())
    }
}
