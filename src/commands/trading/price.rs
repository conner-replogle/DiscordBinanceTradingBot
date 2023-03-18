use arc_swap::{ArcSwap, ArcSwapAny, Guard};
use binance::{account::Account, market::Market, model::SymbolPrice};
use plotters::{
    backend::PixelFormat,
    prelude::{BitMapBackend, ChartBuilder, DrawingArea, IntoDrawingArea, PathElement},
    series::LineSeries,
    style::{Color, IntoFont, BLACK, RED, WHITE},
};
use serenity::{
    builder::CreateComponents,
    client::Context,
    futures::StreamExt,
    model::prelude::{component::{ButtonStyle, InputTextStyle}, AttachmentId, AttachmentType, EmbedImage, Message, interaction::InteractionResponseType},
    FutureExt,
};
use std::{borrow::Cow, future::IntoFuture, path::Path, sync::Arc, task::Poll, time::Duration};
use tokio::{fs::File,sync::RwLock, pin, select, time};
use tracing::{debug, instrument, warn};

use serenity::{
    async_trait, builder::CreateApplicationCommand,
    model::prelude::interaction::application_command::ApplicationCommandInteraction,
};

use crate::{
    binance_wrapped::BinanceWrapped,
    commands::{CommandError, SlashCommand},
    config::{Config, ValueType}, error::TradingBotError,
};

pub(crate) const COMMAND_NAME: &'static str = "price";
pub(crate) fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name(COMMAND_NAME)
        .description("view real time price of btc")
}

pub struct PriceCommand {
    binance: BinanceWrapped,
    market: Market,
}

impl PriceCommand {
    pub fn new(binance: BinanceWrapped, market: Market) -> Self {
        PriceCommand { binance, market }
    }
}

#[async_trait]
impl SlashCommand for PriceCommand {
    fn config(&self) -> crate::commands::CommandConfig {
        crate::commands::CommandConfig {
            accessLevel: crate::commands::AccessLevels::TRADER,
            counts_as_activity: true,
            ..Default::default()
        }
    }
    #[instrument(skip_all, name = "Price Command", level = "trace")]
    async fn run(
        &self,
        interaction: ApplicationCommandInteraction,
        ctx: Context,
        config: Arc<ArcSwapAny<Arc<Config>>>,
    ) -> Result<(), CommandError> {
        let config = config.load();
        debug!("Executiuting Price Command");
        let binance = self.binance;
        let mut msg = interaction
            .get_interaction_response(&ctx.http)
            .await
            .unwrap();

        let mut interaction_future =
            Box::from(msg.await_component_interactions(&ctx.shard).build());

        let mut interval = time::interval(Duration::from_millis(1000));
        let mut prices = vec![];

        let len = match config.get("trading", "price_command_price_len")? {
            Some(int) => int,
            None => 60,
        };
        let symbol = match config.get::<String>("trading", "symbol")? {
            Some(symbol) => symbol,
            None => "BTCUSDT".into(),
        };
        let mut price = self.market.get_price(&symbol).unwrap();
        loop {
            let transaction = binance.get_transaction()?;
            let a = msg.clone();
            let id = a.attachments.first();
            let  components;

            if let Some(Some(a)) = interaction_future.next().now_or_never() {
                match a.data.custom_id.as_str() {
                    "cancel" => {
                        
                        msg.edit(&ctx.http, |a| {
                            if let Some(uid) = id {
                                a.remove_existing_attachment(uid.id);
                            }
                            a.content("Price Closed")
                                .set_embeds(Vec::new())
                                .set_components(CreateComponents::default())
                        })
                        .await?;

                        return Ok(());
                    }
                    "buy" => {
                        if let Some(stub) = binance.is_clocked_in()?{
                            if stub.user_id != interaction.user.id.0 as i64{
                                return Err(CommandError::TradingBotError(TradingBotError::NotClockedIn("".into())))
                            }
                        }else{
                            return Err(CommandError::TradingBotError(TradingBotError::NotClockedIn("".into())))
                        }
                        binance.buy(Some(price.price as f32), None)?;
                        a.create_interaction_response(&ctx, |r| {
                            r.kind(InteractionResponseType::UpdateMessage).interaction_response_data(|a| {
                            a.content(format!("Bought @${}",price.price))
                        })})
                        .await?;
                        
                        
                    }
                    "market_buy" => {
                        //TODO finish this shi
                        if let Some(stub) = binance.is_clocked_in()?{
                            if stub.user_id != interaction.user.id.0 as i64{
                                return Err(CommandError::TradingBotError(TradingBotError::NotClockedIn("".into())))
                            }
                        }else{
                            return Err(CommandError::TradingBotError(TradingBotError::NotClockedIn("".into())))
                        }
                        binance.buy(None, None)?;
                        a.create_interaction_response(&ctx, |r| {
                            r.kind(InteractionResponseType::UpdateMessage).interaction_response_data(|a| {
                            a.content("Bought @Market")

                        })})
                        .await?;
                        
                    }
                    "market_sell" => {
                        //TODO finish this shi
                        if let Some(stub) = binance.is_clocked_in()?{
                            if stub.user_id != interaction.user.id.0 as i64{
                                return Err(CommandError::TradingBotError(TradingBotError::NotClockedIn("".into())))
                            }
                        }else{
                            return Err(CommandError::TradingBotError(TradingBotError::NotClockedIn("".into())))
                        }
                        binance.sell(None, None)?;
                        a.create_interaction_response(&ctx, |r| {
                            r.kind(InteractionResponseType::UpdateMessage).interaction_response_data(|a| {
                            a.content("Sold @Market")
                        })})
                        .await?;
                        
                    }
                    "sell" => {
                        if let Some(stub) = binance.is_clocked_in()?{
                            if stub.user_id != interaction.user.id.0 as i64{
                                return Err(CommandError::TradingBotError(TradingBotError::NotClockedIn("".into())))
                            }
                        }else{
                            return Err(CommandError::TradingBotError(TradingBotError::NotClockedIn("".into())))
                        }
                        binance.sell(Some(price.price as f32), None)?;
                        a.create_interaction_response(&ctx, |r| {
                            r.kind(InteractionResponseType::UpdateMessage).interaction_response_data(|a| {
                            a.content(format!("Sold @${}",price.price))
                        })
                        })
                        .await?;
                       
                    }
                    _ => {}
                }
            }
            price = self.market.get_price(&symbol).unwrap();

            if transaction.as_ref().is_some() && transaction.as_ref().unwrap().sellReady && transaction.as_ref().unwrap().sellAvgPrice.is_none(){
                let mut c = CreateComponents::default();
                c.create_action_row(|r|
                    
                    r.create_button(|b|
                        b.custom_id("sell")
                        .label(format!("Sell@${:.5}",price.price))
                        .style(ButtonStyle::Success)
                    ).create_button(|b|
                        b.custom_id("market_sell")
                        .label("Market Sell")
                        .style(ButtonStyle::Success)
                    )
                    .create_button(|b|
                        b.custom_id("cancel")
                        .label("Cancel")
                        .style(ButtonStyle::Danger)
                    )           
                );
                components = Some(c)
            }else if transaction.is_none() || (transaction.as_ref().unwrap().buyReady && transaction.as_ref().unwrap().buyAvgPrice.is_none()){
                let mut c = CreateComponents::default();
                c.create_action_row(|r|
                r.create_button(|b|
                    b.custom_id("buy")
                    .label(format!("Buy@${:.5}",price.price))
                    .style(ButtonStyle::Success)
                ).create_button(|b|
                    b.custom_id("market_buy")
                    .label("Market Buy")
                    .style(ButtonStyle::Success)
                )
                .create_button(|b|
                    b.custom_id("cancel")
                    .label("Cancel")
                    .style(ButtonStyle::Danger)
                )
                );
                components = Some(c)
            }else{
                components = None;
            }
                
               
            
            //MAKE SYMBOL A CONFIG
            prices.push(price.price as f32);
            if prices.len() > len as usize {
                prices.remove(0);
            }
            draw_canvas(&prices).unwrap();

            msg.edit(&ctx.http, |m| {
                if let Some(at) = id {
                    m.remove_existing_attachment(at.id);
                }

                m.embed(|e| {
                    e.image("attachment://image.png");
                    if let Some(transaction) =&transaction{
                        if let Some(price) = transaction.buyAvgPrice{
                            e.field("Bought At Price", format!("${:.5}",price), false);

                        }
                    }
                    e
                })
                .attachment(AttachmentType::Path(Path::new("data/image.png")))
                .set_components(components.unwrap_or(CreateComponents::default()))
                
            })
            .await
            .unwrap();

            interval.tick().await;
        }

        Ok(())
    }
}

fn draw_canvas(prices: &Vec<f32>) -> Result<(), Box<dyn std::error::Error>> {
    let (x, y) = (720, 480);
    let root = BitMapBackend::new("data/image.png", (x, y)).into_drawing_area();
    root.fill(&WHITE)?;
    
    let mut chart = ChartBuilder::on(&root)
        .caption(
            "BTCUSDT",//TODO CHANGE THIS BASED ON SYMBOL
            ("sans-serif", 50).into_font().color(&BLACK),
        )
        .margin(5)
        .x_label_area_size(50)
        .y_label_area_size(50)
    
        .build_cartesian_2d(
            0 as f32..(prices.len()) as f32,
            *(prices
                .iter()
                .min_by(|x, y| x.total_cmp(y))
                .unwrap_or(&10000f32)) as f32
                ..*(prices
                    .iter()
                    .max_by(|x, y| x.total_cmp(y))//TODO FIX THIS MESS
                    .unwrap_or(&30000f32)),
        )?;

    chart.configure_mesh().draw()?;

    chart
        .draw_series(LineSeries::new(
            prices.iter().enumerate().map(|(i, x)| (i as f32, *x)),
            &RED,
        ))?
        .label(format!("{:.6}",prices.last().unwrap_or(&0.0)));

    chart
        .configure_series_labels()
        .label_font(("sans-serif", 30).into_font().color(&BLACK))
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;

    root.present()?;

    Ok(())
}
