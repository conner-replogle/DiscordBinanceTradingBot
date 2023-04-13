use arc_swap::ArcSwapAny;
use chrono::{NaiveDateTime, Utc};
use chrono_tz::Tz;
use diesel::sql_types::Time;
use serenity::{client::Context, model::prelude::{command::CommandOptionType, interaction::application_command::CommandDataOption, component::ButtonStyle}};
use std::{sync::Arc, time::Duration};
use tracing::{instrument, trace, debug};

use serenity::{
    async_trait,
    builder::CreateApplicationCommand,
    model::prelude::interaction::{
        application_command::ApplicationCommandInteraction, InteractionResponseType,
    },
};
const FORMAT_STRING: &'static str = "%d/%m/%Y %I:%M %P";
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
        .create_option(|opt|
            opt.kind(CommandOptionType::SubCommand)
            .name("create")
            .description("Create a reservation")
            .create_sub_option(|opt| {
                opt.kind(CommandOptionType::String)
                    .name("start_time")
                    .description("Start time of reservation")
                    .set_autocomplete(true)
                    .required(true)
            })
            .create_sub_option(|opt| {
                opt.kind(CommandOptionType::String)
                    .name("end_time")
                    .description("End time of reservation")
                    .set_autocomplete(true)
                    .required(true)
            })
        )
        .create_option(|opt|
            opt.kind(CommandOptionType::SubCommand)
            .name("list")
            .description("list current reservations and delete them if you wish")
        )
        
        
}


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
        
        let config = config.load();

        let options:&Vec<CommandDataOption> = interaction.data.options.as_ref();
        
        if let Some(sub_command) = options.iter().find(|opt| opt.name == "create"){ 
            let time_zone = match config.get::<String>("schedule","timezone")? {
                Some(tz_str) => tz_str.parse::<Tz>().unwrap_or(Tz::UTC),
                None => Tz::UTC
            };
            let Ok(start_time) = NaiveDateTime::parse_from_str(
                &get_option::<String>(&mut sub_command.options.iter(), "start_time")?, FORMAT_STRING) else{
                return Err(CommandError::IncorrectParameters("Failed to parse start time as Date".into()));
            };
            let start_time = start_time.and_local_timezone(time_zone).unwrap();
            let start_time = start_time.with_timezone(&Utc);

            let Ok(end_time) = NaiveDateTime::parse_from_str(&get_option::<String>(&mut sub_command.options.iter(), "end_time")?, FORMAT_STRING) else{
                return Err(CommandError::IncorrectParameters("Failed to parse start time as Date".into()));
            };
            let end_time = end_time.and_local_timezone(time_zone).unwrap();
            let end_time = end_time.with_timezone(&Utc);
            debug!("Attempting to create Reservation at {} till {}",start_time,end_time);
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
                            start_time.with_timezone(&time_zone).format(FORMAT_STRING),
                            end_time.with_timezone(&time_zone).format(FORMAT_STRING)
                        ))
                    })
                    .await?;
            } else {
                interaction
                    .edit_original_interaction_response(&ctx.http, |response| {
                        response.content(format!(
                            "Reservation Failed to create for {} to {}",
                            start_time.with_timezone(&time_zone).format(FORMAT_STRING),
                            end_time.with_timezone(&time_zone).format(FORMAT_STRING)
                        ))
                    })
                    .await?;
            }
        }else{
            let reservation = Schedule::pull_reservations_by_id(i64::from(interaction.user.id))?;


        
            interaction
            .edit_original_interaction_response(&ctx.http, |response| {
                response.content("Click a button to delete reservation");
                response.embed(|e| {
                    e.title(format!(
                        "Reservations"
                    ));
                    for (i,reservation) in reservation.iter().enumerate(){
                        e.field(
                            format!("#{i}"),
                            format!("Start: {} End {}",reservation.start_time.format(FORMAT_STRING),reservation.end_time.format(FORMAT_STRING)),
                            false
                        );
                    }
                    e
                });
                let mut reservations_iter = reservation.iter().enumerate().peekable();
                response.components(|c| {
                    loop {
                        if reservations_iter.peek().is_none(){
                            break;
                        }
                        let next_5 = reservations_iter.clone().take(5);
                    
                        
                        c.create_action_row(|r| {
                            for (i,reservation) in next_5 {
                                r.create_button(|b|
                                    b.label(format!("#{i} {}",reservation.start_time.format(FORMAT_STRING))).custom_id(i.to_string()).style(ButtonStyle::Danger)
                                );
                            }
                            r
                        });
                        if let Err(_) = reservations_iter.advance_by(5){
                            break;
                        }
                        
                    }
                    c
                });
                
                response
            }).await?;
            let message = interaction.get_interaction_response(&ctx).await.unwrap();

            let button = match message
                .await_component_interaction(&ctx)
                .timeout(Duration::from_secs(30))
                .await
            {
                Some(x) => x,
                None => {
                    interaction
                        .edit_original_interaction_response(&ctx.http, |response| {
                            response
                                .content("Order Place Timed Out")
                                .components(|c| c.set_action_rows(Vec::new()))
                        })
                        .await?;
                    return Ok(());
                }
            };
            let reservation_index = button.data.custom_id.parse::<usize>().unwrap();
            let reservation = reservation.get(reservation_index).unwrap();
            debug!("Canceling Reservation");
            Schedule::cancel(reservation.id)?;
            interaction
            .edit_original_interaction_response(&ctx.http, |response| {
                response
                    .content(format!("{} Canceled",reservation.start_time.format(FORMAT_STRING)))
                    .components(|c| c.set_action_rows(Vec::new()))
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
        let options:&Vec<CommandDataOption> = interaction.data.options.as_ref();
        
        if let Some(sub_command) = options.iter().find(|opt| opt.name == "create"){ 
            let mut options = sub_command.options.iter();
            let focused = options.clone().find(|opt| opt.focused);

            let Some(focused) = focused else{
                return Ok(());
            };

            let formatted: Vec<String>;
            let time_zone = match config.get::<String>("schedule","timezone")? {
                Some(tz_str) => tz_str.parse::<Tz>().unwrap_or(Tz::UTC),
                None => Tz::UTC
            };
            if focused.name == "start_time" {
                let start_time = get_option::<String>(&mut options, "start_time")?;
                
                let time_slots = Schedule::open_time_slots(None, &config)?;
                trace!(
                    "Starttime {}",start_time
                );
                formatted = time_slots
                    .iter()
                    .filter_map(|slot| {
                        match slot{
                            TimeSlot::OPEN(mut time) => {
                                let tz_time = time.with_timezone(&time_zone);
                                let value = tz_time.format(FORMAT_STRING).to_string();
                                if value.contains(&start_time) {
                                    Some(value)
                                } else {
                                    None
                                }
                            },
                            TimeSlot::RESERVED { reservation } => {

                                let start = reservation.start_time.with_timezone(&time_zone);
                                let end = reservation.end_time.with_timezone(&time_zone);
                                let Ok(user) = user_ops::find_user(reservation.user_id) else{
                                    return  Some(format!("reserved by {} at {} till {}",reservation.id,start.format(FORMAT_STRING),end.format(FORMAT_STRING)));
                                };
                                Some(format!("reserved by {} at {} till {}",user.tag,start.format(FORMAT_STRING),end.format(FORMAT_STRING)))
                            }
                        }
                        
                    })
                    .collect();
            } else {
                let Ok(start_time) = NaiveDateTime::parse_from_str(&get_option::<String>(&mut options, "start_time")?, FORMAT_STRING) else{
                    return Err(CommandError::IncorrectParameters("Failed to parse start time as Date".into()));
                };
                let start_time = start_time.and_local_timezone(time_zone).unwrap();

                let start_time = start_time.with_timezone(&Utc);
                let time_slots = Schedule::open_time_slots(Some(start_time), &config)?;
                trace!(
                    "Starttime {}",start_time
                );
                let end_time = get_option::<String>(&mut options, "end_time")?;
                formatted = Box::new(time_slots.iter().filter_map(|slot| {
                    match slot{
                        TimeSlot::OPEN(time) => {
                            let tz_time = time.with_timezone(&time_zone);
                            let value = tz_time.format(FORMAT_STRING).to_string();
                            if time < &start_time {
                                None
                            } else if value.contains(&end_time) {
                                Some(value)
                            } else {
                                None
                            }
                        },
                        TimeSlot::RESERVED { reservation } => {
                            let start = reservation.start_time.with_timezone(&time_zone);
                            let end = reservation.end_time.with_timezone(&time_zone);

                            let Ok(user) = user_ops::find_user(reservation.user_id) else{
                                return  Some(format!("reserved by {} at {} till {}",reservation.id,start.format(FORMAT_STRING),end.format(FORMAT_STRING)));
                            };
                            Some(format!("reserved by {} at {} till {}",user.tag,start.format(FORMAT_STRING),end.format(FORMAT_STRING)))
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
        }

        Ok(())
    }
}
