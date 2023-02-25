use arc_swap::{ArcSwapAny, Guard};
use binance::account::Account;
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

pub(crate) const COMMAND_NAME: &'static str = "balance";
pub(crate) fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name(COMMAND_NAME)
        .description("Check the balance of the account")
}

pub struct BalanceCommand {
    binance: Account,
}
impl BalanceCommand {
    pub fn new(binance: Account) -> Self {
        BalanceCommand { binance }
    }
}
#[async_trait]
impl SlashCommand for BalanceCommand {
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
        let btc = self.binance.get_balance("BTC")?;
        let usdt = self.binance.get_balance("USDT")?;

        interaction
            .edit_original_interaction_response(&ctx.http, |response| {
                response.content("").embed(|embed| {
                    embed
                        .title("Account Balances")
                        .field(
                            "BTC",
                            format!("Free {} Locked {}", btc.free, btc.locked),
                            false,
                        )
                        .field(
                            "USDT",
                            format!("Free {} Locked {}", usdt.free, usdt.locked),
                            false,
                        )
                })
            })
            .await?;
        Ok(())
    }
}
