use arc_swap::ArcSwapAny;
use chrono::{NaiveDateTime, Utc};
use chrono_tz::Tz;
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
pub(crate) const COMMAND_NAME: &'static str = "summary";
pub(crate) fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name(COMMAND_NAME)
        .description("get employee summaries")

}


#[derive(Debug)]
pub struct SummaryCommand;
impl SummaryCommand {
    pub fn new() -> Self {
        SummaryCommand {}
    }
}
#[async_trait]
impl SlashCommand for SummaryCommand {
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

        
        Ok(())
    }
}
