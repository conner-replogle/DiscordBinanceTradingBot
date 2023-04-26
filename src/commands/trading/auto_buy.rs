use arc_swap::{ArcSwap, ArcSwapAny, Guard};
use binance::account::Account;
use serenity::{client::Context, model::prelude::{component::ButtonStyle, command::CommandOptionType, interaction::InteractionResponseType}};
use std::{sync::Arc, thread, time::Duration};
use tracing::{debug, warn, trace};
use tokio::sync::RwLock;

use serenity::{
    async_trait, builder::CreateApplicationCommand,
    model::prelude::interaction::application_command::ApplicationCommandInteraction,
};

use crate::{
    binance_wrapped::BinanceWrapped,
    commands::{CommandError, SlashCommand},
    config::{Config, ValueType}, utils::get_option::get_option, error::TradingBotError,
};

pub(crate) const COMMAND_NAME: &'static str = "auto_buy";
pub(crate) fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name(COMMAND_NAME)
        .description("buy BTC at market and sell at a offset price")
}

pub struct AutoBuyCommand {
    binance: Arc<RwLock<BinanceWrapped>>,
}

impl AutoBuyCommand {
    pub fn new(binance: Arc<RwLock<BinanceWrapped>>) -> Self {
        AutoBuyCommand { binance }
    }
}
#[async_trait]
impl SlashCommand for AutoBuyCommand {
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

        let binance = self.binance.read().await;
        trace!("Locked Binance Account");
        if let Some(stub) = binance.is_clocked_in()?{
            if stub.user_id != interaction.user.id.0 as i64{
                return Err(CommandError::TradingBotError(TradingBotError::NotClockedIn("".into())))
            }
        }else{
            return Err(CommandError::TradingBotError(TradingBotError::NotClockedIn("".into())))
        }
        let market_orders_allowed = match config.get("trading", "market_orders")? {
            Some(int) => int,
            None => true,
        };

        if !market_orders_allowed{
            interaction
            .edit_original_interaction_response(&ctx.http, |response| {
                response
                    .content("Market orders are disabled please provide a price")
                }
            ).await?;
            return Ok(());

        }


        let msg = format!("Pick a offset price to sell at after market buy ");
        interaction
            .edit_original_interaction_response(&ctx.http, |response| {
                response
                    .content(msg)
                    .components(|c| {
                        c.create_action_row(|row| {
                            row.create_button(|button| {
                                button
                                    .custom_id("cancel")
                                    .label("Cancel")
                                    .style(ButtonStyle::Success)
                            }).create_button(|button| {
                                button
                                    .custom_id("1")
                                    .label("1")
                                    .style(ButtonStyle::Success)
                            }).create_button(|button| {
                                button
                                    .custom_id("3")
                                    .label("3")
                                    .style(ButtonStyle::Success)
                            }).create_button(|button| {
                                button
                                    .custom_id("5")
                                    .label("5")
                                    .style(ButtonStyle::Success)
                            }).create_button(|button| {
                                button
                                    .custom_id("10")
                                    .label("10")
                                    .style(ButtonStyle::Success)
                            })
                            
                        })
                    })
            })
            .await?;
        let message = interaction.get_interaction_response(&ctx).await.unwrap();
        let timeout = match config.get("trading", "buy_auto_timeout_s")? {
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
        trace!("Recieved Button Interaction");
        if a.data.custom_id != "cancel" {
            let Some(binance_account) = binance.account.as_ref() else {
                a.edit_original_interaction_response(&ctx, |response| {
                    response
                        .content("Request failed No Account set")
                }).await?;
                return Ok(());
            };
            trace!("Sending BUY");
            a.create_interaction_response(&ctx, |r| {
                r.kind(InteractionResponseType::DeferredUpdateMessage)
            }).await?;
            let order = binance.buy(None, None)?;
            debug!("Order {:#?}",order);
            let transaction;
            loop{
                if let Ok(Some(db_transaction)) = binance.get_transaction(){
                    if db_transaction.sellReady && db_transaction.buyAvgPrice.is_some(){
                        a.edit_original_interaction_response(&ctx, |response| {
                            response
                                .content("Market Order filled sending sell order")
                        })
                        .await?; 
                        transaction =db_transaction;
                        break;
                    }
                }else{
                    a.edit_original_interaction_response(&ctx, |response| {
                        response
                            .content("Waiting for market order to fill")
                    })
                    .await?; 
                }
                

                tokio::time::sleep(Duration::from_millis(100)).await;    
            }
            debug!("Order Completed");
                
            let price = a.data.custom_id.parse::<f32>().unwrap();
            let buy_price = transaction.buyAvgPrice.unwrap() as f32;
            debug!("selling at price {}",buy_price+price);
            let order = binance.sell(Some(buy_price+price), None)?;
            a.edit_original_interaction_response(&ctx, |response| {
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
            trace!("Order Canceled");

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
