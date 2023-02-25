use arc_swap::{ArcSwap, ArcSwapAny, Guard};
use binance::{account::Account, market::Market};
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
    model::prelude::{component::ButtonStyle, AttachmentId, AttachmentType, EmbedImage, Message},
    FutureExt,
};
use std::{borrow::Cow, future::IntoFuture, path::Path, sync::Arc, task::Poll, time::Duration};
use tokio::{fs::File, pin, select, time};
use tracing::{debug, instrument, warn};

use serenity::{
    async_trait, builder::CreateApplicationCommand,
    model::prelude::interaction::application_command::ApplicationCommandInteraction,
};

use crate::{
    commands::{CommandError, SlashCommand},
    config::{Config, ValueType},
};

pub(crate) const COMMAND_NAME: &'static str = "price";
pub(crate) fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name(COMMAND_NAME)
        .description("view real time price of btc")
}

pub struct PriceCommand {
    binance: Account,
    market: Market,
}

impl PriceCommand {
    pub fn new(binance: Account, market: Market) -> Self {
        PriceCommand { binance, market }
    }
}

#[async_trait]
impl SlashCommand for PriceCommand {
    fn config(&self) -> crate::commands::CommandConfig {
        crate::commands::CommandConfig {
            accessLevel: crate::commands::AccessLevels::TRADER,
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

        let mut msg = interaction
            .get_interaction_response(&ctx.http)
            .await
            .unwrap();

        let mut interaction_future =
            Box::from(msg.await_component_interactions(&ctx.shard).build());

        let mut interval = time::interval(Duration::from_millis(1000));
        let mut prices = vec![];
        let len = match config.get("trading", "price_command_price_len") {
            ValueType::INT(Some(a)) => a,
            _ => {
                warn!("Trading/price_command_price_len did not have a value");
                60
            }
        };
        loop {
            let a = msg.clone();
            let id = a.attachments.first();
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
                        //TODO
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

                    _ => {}
                }
            }
            //MAKE SYMBOL A CONFIG
            let price = self.market.get_price("BTCUSDT").unwrap();
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
                    e.image("attachment://image.png")
                        .field("Current Price", price.price, false)
                })
                .content("")
                .attachment(AttachmentType::Path(Path::new("data/image.png")))
                .components(|c| {
                    c.create_action_row(|r| {
                        r.create_button(|b| {
                            b.custom_id("cancel")
                                .label("Cancel")
                                .style(ButtonStyle::Danger)
                        })
                        .create_button(|b| {
                            b.custom_id("buy").label("Buy").style(ButtonStyle::Success)
                        })
                    })
                })
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
    root.fill(&BLACK)?;
    let mut chart = ChartBuilder::on(&root)
        .caption(
            "Current Stock Data",
            ("sans-serif", 50).into_font().color(&WHITE),
        )
        .margin(5)
        .x_label_area_size(30)
        .y_label_area_size(50)
        
        .build_cartesian_2d(
            0 as f32..(prices.len()) as f32,
            *(prices
                .iter()
                .min_by(|x, y| x.total_cmp(y))
                .unwrap_or(&10000f32)) as f32
                ..*(prices
                    .iter()
                    .max_by(|x, y| x.total_cmp(y))
                    .unwrap_or(&30000f32)),
        )?;

    chart.configure_mesh().draw()?;

    chart
        .draw_series(LineSeries::new(
            prices.iter().enumerate().map(|(i, x)| (i as f32, *x)),
            &RED,
        ))?
        .label("BTCUSDT");

    chart
        .configure_series_labels()
        .label_font(("sans-serif", 30).into_font().color(&WHITE))
        .background_style(&BLACK.mix(0.8))
        .border_style(&WHITE)
        .draw()?;

    root.present()?;

    Ok(())
}
