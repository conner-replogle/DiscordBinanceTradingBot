use arc_swap::{ArcSwap, ArcSwapAny};
use binance::account::Account;
use diesel::IntoSql;
use serenity::{client::Context, model::prelude::{component::ButtonStyle, command::CommandOptionType}};
use std::{sync::Arc, time::Duration};
use tracing::{debug, warn, trace};
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
    config::{Config, ValueType}, utils::get_option::get_option, error::TradingBotError,
};
pub(crate) const COMMAND_NAME: &'static str = "sell";
pub(crate) fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name(COMMAND_NAME)
        .description("sell BTC at either market or a fixed price")
        .create_option(|opt|
            opt.name("price")
            .description("price to sell at leave blank for market")
            .kind(CommandOptionType::Number)
            //.set_autocomplete(true)
        )
        .create_option(|opt|
            opt.name("quantity")
            .description("account percentage to sell with leave blank to sell with whole account 0-1")
            .kind(CommandOptionType::Number)
        )
}

pub struct SellCommand {
    binance: Arc<RwLock<BinanceWrapped>>,
}
impl SellCommand {
    pub fn new(binance: Arc<RwLock<BinanceWrapped>>) -> Self {
        SellCommand { binance }
    }
}
#[async_trait]
impl SlashCommand for SellCommand {
    fn config(&self) -> crate::commands::CommandConfig {
        crate::commands::CommandConfig {
            accessLevel: crate::commands::AccessLevels::TRADER,
            counts_as_activity: true,
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
        let binance = self.binance.read().await;
        trace!("Locked Binance Account");
        if let Some(stub) = binance.is_clocked_in()?{
            if stub.user_id != interaction.user.id.0 as i64{
                return Err(CommandError::TradingBotError(TradingBotError::NotClockedIn("".into())))
            }
        }else{
            return Err(CommandError::TradingBotError(TradingBotError::NotClockedIn("".into())))
        }
        let price = match get_option::<f32>(&mut interaction.data.options.iter(), "price"){
            Ok(price) => Some(price),
            Err(err) => {
                warn!("Error parsing price {err}");
                None
            }
        };
        let quantity = match get_option::<f64>(&mut interaction.data.options.iter(), "quantity"){
            Ok(quantity) => Some(quantity),
            Err(err) => {
                warn!("Error parsing quantity {err}");
                None
            }
        };
        let market_orders_allowed = match config.get("trading", "market_orders")? {
            Some(int) => int,
            None => true,
        };

        let confirm_order = match config.get("trading", "confirm_orders")? {
            Some(int) => int,
            None => true,
        };

        if !market_orders_allowed && price.is_none(){
            interaction
            .edit_original_interaction_response(&ctx.http, |response| {
                response
                    .content("Market orders are disabled please provide a price")
                }
            ).await?;
            return Ok(());

        }
        if confirm_order{
            let msg = format!("Confirm placing order at {}",if price.is_some() {price.unwrap().to_string()}else{"Market Price".into()});
            trace!(msg);
            interaction
                .edit_original_interaction_response(&ctx.http, |response| {
                    response
                        .content(msg)
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

            let timeout = match config.get("trading", "sell_timeout_s")? {
                Some(int) => int,
                None => 60,
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
            trace!("Recieved button response");
            if a.data.custom_id != "confirmed" {
                interaction
                    .edit_original_interaction_response(&ctx.http, |response| {
                        response
                            .content("Order Cancelled")
                            .components(|c| c.set_action_rows(Vec::new()))
                    })
                    .await?;
                return Ok(())
            }
            
        }
        trace!("sending sell");
        interaction
        .edit_original_interaction_response(&ctx.http, |r| {
        r.content("sending sell")
        }).await?;
        let order = binance.sell(price, quantity)?;//TODO ADD QUANTITY PARAM
        interaction.edit_original_interaction_response(&ctx, |response| {
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
        Ok(())
    }
}
