use arc_swap::ArcSwapAny;
use chrono::{NaiveDateTime, Utc};
use diesel::sql_types::Time;
use serenity::{client::Context, model::prelude::command::CommandOptionType};
use std::sync::Arc;
use tracing::{instrument, trace, debug};

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
pub(crate) const COMMAND_NAME: &'static str = "unlock";
pub(crate) fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name(COMMAND_NAME)
        .description("force unlock")
}



pub struct UnlockCommand{
    account: Arc<RwLock<BinanceWrapped>>
}
impl UnlockCommand {
    pub fn new(account: Arc<RwLock<BinanceWrapped>>) -> Self {
        UnlockCommand {account}
    }
}
#[async_trait]
impl SlashCommand for UnlockCommand {
    fn config(&self) -> crate::commands::CommandConfig {
        crate::commands::CommandConfig {
            accessLevel: crate::commands::AccessLevels::ADMIN,
            ..Default::default()
        }
    }
    #[instrument(skip_all, name = "Unlock Command", level = "trace")]
    async fn run(
        &self,
        interaction: ApplicationCommandInteraction,
        ctx: Context,
        config: Arc<ArcSwapAny<Arc<Config>>>,
    ) -> Result<(), CommandError> {
        debug!("Unlock Command");

        let account = self.account.read().await;     
        account.unlock(None)?;
        interaction.edit_original_interaction_response(&ctx.http, |i|
            i.content("Unlocked")).await?;

        Ok(())
    }
}

