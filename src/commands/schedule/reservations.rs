use arc_swap::ArcSwapAny;
use chrono::{NaiveDateTime, Utc};
use diesel::sql_types::Time;
use serenity::{client::Context, model::prelude::command::CommandOptionType};
use std::sync::Arc;
use tracing::instrument;

use serenity::{
    async_trait,
    builder::CreateApplicationCommand,
    model::prelude::interaction::{
        application_command::ApplicationCommandInteraction, InteractionResponseType,
    },
};

use crate::{
    commands::{AutoComplete, CommandError, SlashCommand},
    config::Config,
    models::NewReservation,
    ops::user_ops,
    schedule::{Schedule, TimeSlot},
    schema::reservations,
    utils::get_option::{self, get_option},
};
pub(crate) const COMMAND_NAME: &'static str = "reserve";
pub(crate) fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name(COMMAND_NAME)
        .description("Reserve a time slot")
        .create_option(|opt| {
            opt.kind(CommandOptionType::String)
                .name("start_time")
                .description("Start time of reservation")
                .set_autocomplete(true)
                .required(true)
        })
        .create_option(|opt| {
            opt.kind(CommandOptionType::String)
                .name("end_time")
                .description("End time of reservation")
                .set_autocomplete(true)
                .required(true)
        })
}

fn get_date() {}

#[derive(Debug)]
pub struct ReserveCommand;
impl ReserveCommand {
    pub fn new() -> Self {
        ReserveCommand {}
    }
}
#[async_trait]
impl SlashCommand for ReserveCommand {
    fn config(&self) -> crate::commands::CommandConfig {
        crate::commands::CommandConfig {
            accessLevel: crate::commands::AccessLevels::TRADER,
            ..Default::default()
        }
    }
    #[instrument(skip_all, name = "Reserve Command", level = "trace")]
    async fn run(
        &self,
        interaction: ApplicationCommandInteraction,
        ctx: Context,
        config: Arc<ArcSwapAny<Arc<Config>>>,
    ) -> Result<(), CommandError> {
        let Ok(start_time) = NaiveDateTime::parse_from_str(&get_option::<String>(&mut interaction.data.options.iter(), "start_time")?, "%d/%m/%Y %H:%M") else{
            return Err(CommandError::IncorrectParameters("Failed to parse start time as Date".into()));
        };
        //let start_time = start_time.and_local_timezone(Utc);
        let Ok(end_time) = NaiveDateTime::parse_from_str(&get_option::<String>(&mut interaction.data.options.iter(), "end_time")?, "%d/%m/%Y %H:%M") else{
            return Err(CommandError::IncorrectParameters("Failed to parse start time as Date".into()));
        };
        //let end_time = end_time.and_local_timezone(Utc);

        let out = Schedule::create_reservation(NewReservation {
            start_time,
            end_time,
            user_id: i64::from(interaction.user.id),
        })?;

        if out {
            interaction
                .edit_original_interaction_response(&ctx.http, |response| {
                    response.content(format!(
                        "Reservation Created for {} to {}",
                        start_time.to_string(),
                        end_time.to_string()
                    ))
                })
                .await?;
        } else {
            interaction
                .edit_original_interaction_response(&ctx.http, |response| {
                    response.content(format!(
                        "Reservation Failed to create for {} to {}",
                        start_time.to_string(),
                        end_time.to_string()
                    ))
                })
                .await?;
        }

        Ok(())
    }
}
#[async_trait]
impl AutoComplete for ReserveCommand {
    #[instrument(skip_all, name = "AutoComplete", level = "trace")]
    async fn auto_complete(
        &self,
        interaction: serenity::model::prelude::interaction::autocomplete::AutocompleteInteraction,
        ctx: Context,
        config: Arc<Config>,
    ) -> Result<(), CommandError> {
        let mut options = interaction.data.options.iter();

        let focused = options.clone().find(|opt| opt.focused);

        let Some(focused) = focused else{
            return Ok(());
        };

        let formatted: Vec<String>;

        if focused.name == "start_time" {
            let start_time = get_option::<String>(&mut options, "start_time")?;
            let time_slots = Schedule::open_time_slots(None, &config)?;

            formatted = time_slots
                .iter()
                .filter_map(|slot| {
                    match slot{
                        TimeSlot::OPEN(time) => {
                            let str = time.format("%d/%m/%Y %H:%M").to_string();
                            if str.contains(&start_time) {
                                Some(str)
                            } else {
                                None
                            }
                        },
                        TimeSlot::RESERVED { reservation } => {
                            let Ok(user) = user_ops::find_user(reservation.user_id) else{
                                return  Some(format!("reserved by {} at {} till {}",reservation.id,reservation.start_time.format("%d/%m/%Y %H:%M"),reservation.end_time.format("%d/%m/%Y %H:%M")));
                            };
                            Some(format!("reserved by {} at {} till {}",user.tag,reservation.start_time.format("%d/%m/%Y %H:%M"),reservation.end_time.format("%d/%m/%Y %H:%M")))
                        }
                    }
                    
                })
                .collect();
        } else {
            let Ok(start_time) = NaiveDateTime::parse_from_str(&get_option::<String>(&mut options, "start_time")?, "%d/%m/%Y %H:%M") else{
                return Err(CommandError::IncorrectParameters("Failed to parse start time as Date".into()));
            };

            let start_time = start_time.and_local_timezone(Utc).unwrap();
            let time_slots = Schedule::open_time_slots(Some(start_time), &config)?;

            let end_time = get_option::<String>(&mut options, "end_time")?;
          
            formatted = Box::new(time_slots.iter().filter_map(|slot| {
                match slot{
                    TimeSlot::OPEN(time) => {
                        let time = time.with_timezone(&UTC);
                        let value = time.format("%d/%m/%Y %H:%M").to_string();
                        if time < &start_time {
                            None
                        } else if value.contains(&end_time) {
                            Some(value)
                        } else {
                            None
                        }
                    },
                    TimeSlot::RESERVED { reservation } => {
                        let Ok(user) = user_ops::find_user(reservation.user_id) else{
                            return  Some(format!("reserved by {} at {} till {}",reservation.id,reservation.start_time.format("%d/%m/%Y %H:%M"),reservation.end_time.format("%d/%m/%Y %H:%M")));
                        };
                        Some(format!("reserved by {} at {} till {}",user.tag,reservation.start_time.format("%d/%m/%Y %H:%M"),reservation.end_time.format("%d/%m/%Y %H:%M")))
                    }
                }
                
                
            }))
            .collect();
        }
        interaction
            .create_autocomplete_response(&ctx.http, |a| {
                formatted.iter().take(25).for_each(|str| {
                    a.add_string_choice(str.clone(), str);
                });
                a
            })
            .await?;

        Ok(())
    }
}
