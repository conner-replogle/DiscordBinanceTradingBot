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
use tokio::sync::RwLock;

use crate::{
    commands::{AutoComplete, CommandError, SlashCommand},
    config::Config,
    models::NewReservation,
    ops::user_ops,
    schedule::{Schedule, TimeSlot},
    schema::reservations,
    utils::get_option::{self, get_option}, binance_wrapped::BinanceWrapped,
};
pub(crate) const COMMAND_NAME: &'static str = "clock";
pub(crate) fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name(COMMAND_NAME)
        .description("Change your clocked in status")
}



pub struct ClockCommand{
    account: Arc<RwLock<BinanceWrapped>>
}
impl ClockCommand {
    pub fn new(account: Arc<RwLock<BinanceWrapped>>) -> Self {
        ClockCommand {account}
    }
}
#[async_trait]
impl SlashCommand for ClockCommand {
    fn config(&self) -> crate::commands::CommandConfig {
        crate::commands::CommandConfig {
            accessLevel: crate::commands::AccessLevels::TRADER,
            ..Default::default()
        }
    }
    #[instrument(skip_all, name = "Clock Command", level = "trace")]
    async fn run(
        &self,
        interaction: ApplicationCommandInteraction,
        ctx: Context,
        config: Arc<ArcSwapAny<Arc<Config>>>,
    ) -> Result<(), CommandError> {
        let account = self.account.read().await;     
        let clocked_in = account.is_clocked_in()?;   

        if let Some(_) = clocked_in{
            account.unlock(Some(interaction.user.id.0 as i64))?;
            interaction.edit_original_interaction_response(&ctx.http, |i|
            i.content("Clocked out")).await?;
        }else{
            account.lock(interaction.user.id.0 as i64)?;
            interaction.edit_original_interaction_response(&ctx.http, |i|
                i.content("Clocked In")).await?;
        }

        Ok(())
    }
}

