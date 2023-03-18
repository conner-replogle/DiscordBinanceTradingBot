use arc_swap::{ArcSwap, ArcSwapAny, Guard};
use binance::account::Account;
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
    binance_wrapped::BinanceWrapped,
    commands::{CommandError, SlashCommand},
    config::Config,
};

pub(crate) const COMMAND_NAME: &'static str = "balance";
pub(crate) fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name(COMMAND_NAME)
        .description("Check the balance of the account")
}

pub struct BalanceCommand {
    binance: BinanceWrapped,
}
impl BalanceCommand {
    pub fn new(binance: BinanceWrapped) -> Self {
        BalanceCommand { binance }
    }
}
#[async_trait]
impl SlashCommand for BalanceCommand {
    fn config(&self) -> crate::commands::CommandConfig {
        crate::commands::CommandConfig {
            accessLevel: crate::commands::AccessLevels::ADMIN,
            ephermal: true,
            ..Default::default()
        }
    }

    async fn run(
        &self,
        interaction: ApplicationCommandInteraction,
        ctx: Context,
        config: Arc<ArcSwapAny<Arc<Config>>>,
    ) -> Result<(), CommandError> {
        let binance = self.binance;
        
        let (base,quote) = binance.get_balance()?;


        interaction
            .edit_original_interaction_response(&ctx.http, |response| {
                response.content("").embed(|embed| {
                    embed
                        .title("Account Balances")
                        .field(
                            format!("{}",base.asset),
                            format!("Free {} Locked {}", base.free, base.locked),
                            false,
                        )
                        .field(
                            format!("{}",quote.asset),
                            format!("Free {} Locked {}", quote.free, quote.locked),
                            false,
                        )
                })
            })
            .await?;
        Ok(())
    }
}
