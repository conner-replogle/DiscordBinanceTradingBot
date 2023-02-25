use arc_swap::ArcSwapAny;
use binance::account::Account;
use diesel::IntoSql;
use serenity::{client::Context, model::prelude::component::ButtonStyle};
use std::{sync::Arc, time::Duration};
use tracing::{debug, warn};

use serenity::{
    async_trait,
    builder::CreateApplicationCommand,
    model::prelude::interaction::{
        application_command::ApplicationCommandInteraction, InteractionResponseType,
    },
};

use crate::{
    commands::{CommandError, SlashCommand},
    config::{Config, ValueType},
};
pub(crate) const COMMAND_NAME: &'static str = "sell";
pub(crate) fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name(COMMAND_NAME)
        .description("buy BTC at either market or a fixed price")
}

pub struct SellCommand {
    binance: Account,
}
impl SellCommand {
    pub fn new(binance: Account) -> Self {
        SellCommand { binance }
    }
}
#[async_trait]
impl SlashCommand for SellCommand {
    fn config(&self) -> crate::commands::CommandConfig {
        crate::commands::CommandConfig {
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
        let config = config.load();
        debug!("Executing Sell Command");
        let Ok(btc_free_balance) = format!("{:.6}",self.binance.get_balance("BTC")?.free).parse::<f64>() else{
            return Err(CommandError::ParsingDataError("Parsing Market Data error".into()));
        };
        debug!("Sending buy order with {btc_free_balance}");

        interaction
            .edit_original_interaction_response(&ctx.http, |response| {
                response
                    .content("Confirm placing order at Market Price")
                    .components(|c| {
                        c.create_action_row(|row| {
                            row.create_button(|button| {
                                button
                                    .custom_id("confirmed")
                                    .label("Confirm")
                                    .style(ButtonStyle::Success)
                            })
                            .create_button(|button| {
                                button
                                    .custom_id("canceled")
                                    .label("Cancel")
                                    .style(ButtonStyle::Danger)
                            })
                        })
                    })
            })
            .await?;
        let message = interaction.get_interaction_response(&ctx).await.unwrap();

        let timeout = match config.get("trading", "sell_timeout_s") {
            ValueType::INT(Some(int)) => int,
            val => {
                warn!("config trading/buy_timeout_s does not contain a value {val:?}");
                60
            }
        };

        let a = match message
            .await_component_interaction(&ctx)
            .timeout(Duration::from_secs(timeout as u64))
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
        if a.data.custom_id == "confirmed" {
            let order = self.binance.market_sell("BTCUSDT", btc_free_balance)?;
            interaction
                .edit_original_interaction_response(&ctx.http, |response| {
                    response
                        .content("Order Sent")
                        .embed(|embed| {
                            embed
                                .title(format!("ID{}", order.order_id))
                                .field("Status", order.status, false)
                                .field(
                                    "Filled",
                                    order.cummulative_quote_qty / order.executed_qty,
                                    false,
                                )
                        })
                        .components(|c| c.set_action_rows(Vec::new()))
                })
                .await?;
        } else {
            interaction
                .edit_original_interaction_response(&ctx.http, |response| {
                    response
                        .content("Order Cancelled")
                        .components(|c| c.set_action_rows(Vec::new()))
                })
                .await?;
        }
        Ok(())
    }
}
