use arc_swap::{ArcSwap, ArcSwapAny, Guard};
use binance::account::Account;
use diesel::{query_dsl::methods::FilterDsl, QueryDsl};
use serenity::{client::Context, model::prelude::{component::ButtonStyle, command::CommandOptionType}};
use std::{sync::Arc, thread, time::Duration};
use tracing::{debug, warn};
use tokio::{sync::RwLock, time};

use serenity::{
    async_trait, builder::CreateApplicationCommand,
    model::prelude::interaction::application_command::ApplicationCommandInteraction,
};

use crate::{
    binance_wrapped::BinanceWrapped,
    commands::{CommandError, SlashCommand},
    config::{Config, ValueType}, utils::get_option::get_option, error::TradingBotError, db::establish_connection, models::DBTransaction,
};

pub(crate) const COMMAND_NAME: &'static str = "orders";
pub(crate) fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name(COMMAND_NAME)
        .description("view past orders")
        .create_option(|opt|
            opt.name("length")
            .description("How many orders to go back default is 1")
            .max_int_value(25)
            .kind(CommandOptionType::Integer)
        )

}

pub struct OrdersCommand {
    binance: Arc<RwLock<BinanceWrapped>>,
}

impl OrdersCommand {
    pub fn new(binance: Arc<RwLock<BinanceWrapped>>) -> Self {
        OrdersCommand { binance }
    }
}
#[async_trait]
impl SlashCommand for OrdersCommand {
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
        let length = match get_option::<u32>(&mut interaction.data.options.iter(), "length"){
            Ok(price) => Some(price),
            Err(err) => {
                warn!("Error parsing length {err}");
                None
            }
        };
        debug!("Executing Orders Command");
        let binance = self.binance.read().await;
        let orders: Vec<DBTransaction>;
        {
            use crate::schema::transactions::dsl;
            use diesel::ExpressionMethods;
            use diesel::RunQueryDsl;
            let mut connection = establish_connection(); 
            orders = dsl::transactions.order(dsl::buyOrderTime.desc()).limit(length.unwrap_or(1).into()).get_results::<DBTransaction>(&mut connection)?;
        }
        let symbol = &match config.get::<String>("trading", "symbol")?{
            Some(n) => n,
            None => "BTCUSDT".into()
        };
        let mut interval = time::interval(Duration::from_secs(2));
        let mut dots = true;
        loop{
        interaction.edit_original_interaction_response(&ctx.http, |i| {
            for (n,order) in orders.iter().enumerate(){
                
                let mut buy_orders = Vec::new();
                let mut sell_orders = Vec::new();

                if let Some(account) = binance.account.as_ref(){
                    let mut buy_order_ids = order.buyOrderIds.split(',');
                    while let Some(order_str) = buy_order_ids.next(){
                        if order_str.is_empty(){
                            continue;
                        }
                        let order_id = order_str.parse::<u64>().unwrap();
                        if let Ok(order) = account.order_status(symbol.clone(), order_id){
                            let ct = order.cummulative_quote_qty.parse::<f64>().unwrap();
                            let eq = order.executed_qty.parse::<f64>().unwrap();
                            let price = ct/eq;
                            let quantity = eq;
                            buy_orders.push((price,order));
                        }
                        
                    }
                    let mut sell_order_ids = order.sellOrderIds.split(',');
                    while let Some(order_str) = sell_order_ids.next(){
                        if order_str.is_empty(){
                            continue;
                        }
                        let order_id = order_str.parse::<u64>().unwrap();
                        if let Ok(order) = account.order_status(symbol.clone(), order_id){
                            let ct = order.cummulative_quote_qty.parse::<f64>().unwrap();
                            let eq = order.executed_qty.parse::<f64>().unwrap();
                            let price = ct/eq;
                            let quantity = eq;
                            sell_orders.push((price,order));
                        }
                    }
                     
                }
                if dots{
                    i.content("Pulling data..");
                }else{
                    i.content("Pulling data...");
                }
                dots = !dots;

                i.embed(|e| 
                {

                    e.title(format!("#{} BUY",n+1));
                    
                    for (price,order) in buy_orders.iter(){
                        let cq = order.cummulative_quote_qty.parse::<f32>().unwrap();
                        let oq = order.orig_qty.parse::<f32>().unwrap();
                        let eq = order.executed_qty.parse::<f32>().unwrap();
                        e.field("TPrice", if order.price == 0.0{"market".into()}else{order.price.to_string()}, false);
                        
                        e.field("AvgPrice",price,true)
                        .field("Percentage Done",(eq/oq)*100.0,false)
                        .field("Status",order.status.clone(),false);
                    }

                    return e;
                }).embed(|e| 
                    {
    
                        e.title(format!("#{} Sell",n+1));
                        
                        for (price,order) in sell_orders.iter(){
                            let cq = order.cummulative_quote_qty.parse::<f32>().unwrap();
                            let oq = order.orig_qty.parse::<f32>().unwrap();
                            let eq = order.executed_qty.parse::<f32>().unwrap();
                            
                            e.field("TPrice", if order.price == 0.0{"market".into()}else{order.price.to_string()}, false);
                        
                            
                            e.field("AvgPrice",price,false)
                            .field("Percentage Done",(eq/oq)*100.0,false)
                            .field("Status",order.status.clone(),false);
                        }
    
                        return e;
                    }
                );
            }
            i
            
        }).await?;
        interval.tick().await;
        }
        

        Ok(())
    }
}
