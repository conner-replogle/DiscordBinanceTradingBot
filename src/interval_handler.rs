use std::error::Error;
use std::future::Future;
use std::sync::{Arc};
use binance::account::{OrderSide, Account};
use binance::model::Order;
use tokio::sync::RwLock;

use arc_swap::ArcSwap;
use chrono::{Duration, Utc};

use clokwerk::{AsyncScheduler, TimeUnits};
use diesel::{QueryDsl, RunQueryDsl};

use serenity::model::prelude::ChannelId;
use serenity::prelude::Context;

use tracing::{debug, instrument, trace, warn, error};

use crate::binance_wrapped::BinanceWrapped;
use crate::config::{Config};
use crate::db::{establish_connection, self};
use crate::models::{Reservation, BinanceAccount, ClockStub, DBTransaction};
pub async fn run(ctx: Arc<Context>, config: Arc<ArcSwap<Config>>, binance: Arc<RwLock<BinanceWrapped>>) {
    let mut scheduler = AsyncScheduler::new();
    debug!("We running");
    let ctx_clone = ctx.clone();
    let con_clone = config.clone();
    scheduler.every(1.minute()).run(move || {
        return handle_errors(handle_reservations(ctx.clone(), config.clone()));
    });
    let ctx_clone2 = ctx_clone.clone();
    let con_clone2 = con_clone.clone();
    scheduler.every(5.seconds()).run(move || {
        return handle_errors(handle_orders(ctx_clone.clone(), con_clone.clone(),binance.clone()));
    });
 
    scheduler.every(1.minute()).run(move || {
        return handle_errors(handle_afk(ctx_clone2.clone(), con_clone2.clone()));
    });

    loop {
        scheduler.run_pending().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
}
async fn handle_errors(fun: impl Future<Output = Result<(), Box<dyn Error>>>) {
    if let Err(err) = fun.await {
        warn!("error occured {} from {:?}", err, err.source());
    }
}

#[instrument(name = "AFK Handler", skip_all)]
async fn handle_afk(
    ctx: Arc<Context>,
    config: Arc<ArcSwap<Config>>) -> Result<(), Box<dyn Error>> {
        
        
        
        
        
        Ok(())
}


use diesel::ExpressionMethods;
#[instrument(name = "Order Handler", skip_all)]
async fn handle_orders(
    ctx: Arc<Context>,
    config: Arc<ArcSwap<Config>>,
    binance_w: Arc<RwLock<BinanceWrapped>>
) -> Result<(), Box<dyn Error>> {
    let mut connection = establish_connection();
    let config = config.load();
    let dbinance = binance_w.read().await;
    let Ok(account) = dbinance.get() else {
        trace!("No Account");
        return Ok(())
    };

    let transaction: DBTransaction;
    {
        let transaction_id: Option<i32>;
        {
            use crate::schema::binance_accounts::dsl;
            transaction_id = dsl::binance_accounts.filter(dsl::selected.eq(true)).select(dsl::active_transaction).get_result::<Option<i32>>(&mut connection)?;
            trace!("Active Transaction ID {:?}",transaction_id);
    
        }
        let Some(transaction_id) = transaction_id else {
            return Ok(());
        };
        {
            use crate::schema::transactions::dsl;
            transaction = dsl::transactions.filter(dsl::id.eq(transaction_id)).get_result::<DBTransaction>(&mut connection)?;
            trace!("Active Transaction {:?}",transaction);

        }

    }
    
    let Some(symbol) = config.get::<String>("trading", "symbol")? else {
        trace!("No symbol Set");
        return Ok(())
    };
    if transaction.buyAvgPrice.is_none(){// Check buy Status
        trace!("Pulling buy order");
        let balance = dbinance.get_balance()?;
        let mut ids = transaction.buyOrderIds.split(',');

        
        let id = transaction.buyOrderIds.split(',').last().unwrap();
        if id ==""{
            return Ok(());
        }
        trace!("Last Buy Order {}",id);
        
        let last_order = id.parse::<u64>().unwrap();
        let order = account.order_status(&symbol, last_order)?;
        match order.status.as_str(){
            x if x =="FILLED" || x == "CANCELED"  => {
                debug!("Buy Order filled");
                use crate::schema::transactions::dsl;
                if balance.1.free.parse::<f32>()? + balance.1.locked.parse::<f32>()? <= 5.00{//MAKE CONFIG
                    //Close buy out
                    use crate::schema::transactions::dsl;
                    trace!("Buy Completed");
                    let mut orders = Vec::new();
                    while let Some(order_str) = ids.next(){
                        let order_id = order_str.parse::<u64>().unwrap();
                        let order = account.order_status(&symbol, order_id)?;
                        let ct = order.cummulative_quote_qty.parse::<f64>().unwrap();
                        let eq = order.executed_qty.parse::<f64>().unwrap();
                        let price = ct/eq;
                        let quantity = eq;
                        if quantity <= 0.0{
                            continue;
                        }
                        if price.is_nan(){
                            continue;
                        }
                        orders.push((price,quantity));
                    }
                    trace!("Buy Orders:{:?}",orders);
                    if orders.len() == 0{
                        trace!("No Orders");
                        return Ok(());
                    }
                    let total_qty = orders.iter().fold(0.0,|n,(_,q)| n+q);
                    let avgPrice = orders.iter().fold(0.0,|t,(p,q)| 
                        t+p * (q/total_qty)
                    );
                    
        
                    debug!("Buy Completed with price {}",avgPrice);
        
                    diesel::update(dsl::transactions.filter(dsl::id.eq(transaction.id))).set((dsl::buyReady.eq(false),dsl::sellReady.eq(true),dsl::buyAvgPrice.eq(Some(avgPrice)))).execute(&mut connection)?;
                }else{
                    diesel::update(dsl::transactions.filter(dsl::id.eq(transaction.id))).set(dsl::buyReady.eq(true)).execute(&mut connection)?;
                }
            }

            a => {
                error!("Unknown status {a}");
            }
        }
    }else{
        let balance = dbinance.get_balance()?;
        let mut ids = transaction.sellOrderIds.split(',');

        let id = ids.clone().last().unwrap();
        if id ==""{
            return Ok(());
        }

        trace!("Last Sell Order {}",id);
        
        let last_order = id.parse::<u64>().unwrap();
        let order = account.order_status(&symbol, last_order)?;
        match order.status.as_str(){
            x if x == "FILLED" || x ==  "CANCELED" => {
                debug!("Sell order filled");
                use crate::schema::transactions::dsl;
                if balance.0.free.parse::<f32>()? + balance.0.locked.parse::<f32>()? <= 0.0001{//MAKE CONFIG
                    //Close buy out
                    use crate::schema::transactions::dsl;
                    trace!("Sell Completed");
                    let mut orders = Vec::new();
                    while let Some(order_str) = ids.next(){
                        let order_id = order_str.parse::<u64>().unwrap();
                        let order = account.order_status(&symbol, order_id)?;
                        let ct = order.cummulative_quote_qty.parse::<f64>().unwrap();
                        let eq = order.executed_qty.parse::<f64>().unwrap();
                        let price = ct/eq;
                        let quantity = eq;
                        if quantity <= 0.0{
                            continue;
                        }
                        if price.is_nan(){
                            continue;
                        }
                        orders.push((price,quantity));
                    }
                    trace!("Sell Orders:{:?}",orders);

                    let total_qty = orders.iter().fold(0.0,|n,(_,q)| n+q);
                    let avgPrice = orders.iter().fold(0.0,|t,(p,q)| 
                        t+p * (q/total_qty)
                    );
        
                    debug!("Sell Completed with price {}",avgPrice);
        
        
                    diesel::update(dsl::transactions.filter(dsl::id.eq(transaction.id))).set((dsl::sellReady.eq(false),dsl::sellAvgPrice.eq(Some(avgPrice)))).execute(&mut connection)?;
                
                    
                //      DISCONNECT FROM ACTIVE TRANSACTION
                    trace!("Closing order");
                    {
                        use crate::schema::binance_accounts::dsl;
        
                        diesel::update(dsl::binance_accounts.filter(dsl::active_transaction.eq(Some(transaction.id)))).set(dsl::active_transaction.eq::<Option<i32>>(None)).execute(&mut connection)?;
                        debug!("Order Closed");
                    }
                }else{
                    diesel::update(dsl::transactions.filter(dsl::id.eq(transaction.id))).set(dsl::sellReady.eq(true)).execute(&mut connection)?;

                }
            }

            a => {
                trace!("Unknown status {a}");
            }
            
        }
        
    }
    return Ok(());

}

#[instrument(name = "Reservation Handler", skip_all)]
async fn handle_reservations(
    ctx: Arc<Context>,
    config: Arc<ArcSwap<Config>>,
) -> Result<(), Box<dyn Error>> {
    let config = config.load();
    use crate::schema::reservations::dsl;
    use diesel::ExpressionMethods;
    let mut connection = establish_connection();

    //LOAD CONFIGS
    let alert_time_min = match config.get::<i32>("schedule", "reservation_alert_min")? {
        Some(a) => a,
        None => 15,
    };

    let lock_time_min = match config.get::<i32>("schedule", "reservation_lock_min")? {
        Some(a) => a,
        None => 15,
    };

    let Some(config_reservation_channel) = config.get::<i64>("channels", "reservation_alert")? else{
        warn!("No reservation channel ");
        return Ok(());
    };

    let next_reservation = dsl::reservations
        .order(dsl::start_time.asc())
        .first::<Reservation>(&mut connection)? as Reservation;

    let time_to_alert = next_reservation.start_time - Duration::minutes(alert_time_min as i64);
    let time_to_lock = next_reservation.start_time + Duration::minutes(lock_time_min as i64);
    let now = Utc::now();

    //If the time to alert the player has passed and the reservation was not marked as alerted
    if !next_reservation.alerted && time_to_alert < now.naive_utc() {
        let time_to = next_reservation.start_time - now.naive_utc();

        //Alert player
        ChannelId(config_reservation_channel as u64)
            .send_message(&ctx, |m| {
                m.content(format!(
                    "<@{}> next reservation at {} starts in {}mins",
                    next_reservation.user_id,
                    next_reservation.start_time,
                    time_to.num_minutes()
                ))
            })
            .await?;
        //Mark alerted in db
        diesel::update(dsl::reservations.filter(dsl::id.eq(next_reservation.id)))
            .set(dsl::alerted.eq(true))
            .execute(&mut connection)?;
    }

    if next_reservation.start_time < now.naive_utc() && now.naive_utc() < time_to_lock {
        trace!("Lock Reservation");
        {
        
            use crate::schema::binance_accounts::dsl;

            if let Some(reservation) = dsl::binance_accounts.filter(dsl::selected.eq(true)).select(dsl::active_reservation).first::<Option<i32>>(&mut connection)?{
                if reservation == next_reservation.id{
                    return Ok(());
                }
            }


            diesel::update(dsl::binance_accounts.filter(dsl::selected.eq(true))).set(dsl::active_reservation.eq(Some(next_reservation.id))).execute(&mut connection)?;
        }
    


    } else if time_to_lock < now.naive_utc() {
        {
            use crate::schema::binance_accounts::dsl;
            diesel::update(dsl::binance_accounts.filter(dsl::selected.eq(true))).set(dsl::active_reservation.eq::<Option<i32>>(None)).execute(&mut connection)?;

        }
        diesel::delete(dsl::reservations.filter(dsl::id.eq(next_reservation.id)))
            .execute(&mut connection)?;
        trace!("Reservation cleared")
    }

    Ok(())
}
