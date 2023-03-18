use arc_swap::{ArcSwapAny, Guard};
use last_git_commit::LastGitCommit;
use serenity::client::Context;
use std::sync::Arc;
use tokio::sync::RwLock;
use serenity::{
    async_trait,
    builder::CreateApplicationCommand,
    model::prelude::interaction::{
        application_command::ApplicationCommandInteraction, InteractionResponseType,
    },
};

use crate::{
    commands::{CommandError, SlashCommand},
    config::Config, binance_wrapped::BinanceWrapped,
};
pub(crate) const COMMAND_NAME: &'static str = "status";
pub(crate) fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name(COMMAND_NAME)
        .description("Check the bot's status")
}

pub struct StatusCommand{
    binance: Arc<RwLock<BinanceWrapped>>
}
impl StatusCommand {
    pub fn new(binance: Arc<RwLock<BinanceWrapped>>) -> Self {
        StatusCommand {
            binance
        }
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
        let binance = self.binance.read().await;
        let mut binance_status = "".into();
        if let Some(account) = binance.account.as_ref(){
            let account_info = account.get_account();
            binance_status = match account_info{
                Err(err) => format!("❌ {}",err),
                Ok(_) => format!("✅")
            }
        }

        let lgc = LastGitCommit::new().build().unwrap();
        
        //Check if order handler is up
        //Check if rrservation handler is up
        interaction
            .edit_original_interaction_response(&ctx.http, |response| response.embed(|e|
            e.field("Binance", binance_status, false)
            ).embed(|e|
                e.title("Git Commit")
                .field("Message", lgc.message().unwrap_or(&"No Message".into()), false)
                .field("Git hash", lgc.id().short(), false)
            ))
            .await?;
        Ok(())
    }
}
