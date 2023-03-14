use std::error::Error;
use std::future::Future;
use std::sync::{Arc};
use binance::account::{OrderSide, Account};
use binance::model::Order;
use serenity::builder::CreateComponents;
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
    let binance2 = binance.clone();
    scheduler.every(2.seconds()).run(move || {
        return handle_errors(handle_orders(ctx_clone.clone(), con_clone.clone(),binance2.clone()));
    });
     
    scheduler.every(1.minute()).run(move || {
        return handle_errors(handle_afk(ctx_clone2.clone(), con_clone2.clone(),binance.clone()));
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
    config: Arc<ArcSwap<Config>>,
    binance_w: Arc<RwLock<BinanceWrapped>>) -> Result<(), Box<dyn Error>> {
    let mut connection = establish_connection();
    let config = config.load();
    let dbinance = binance_w.read().await;
    let Some(afk_channel) = config.get::<u64>("channels", "afk_channel")?else {
        return Ok(())
    };
    let Some(stub) = dbinance.is_clocked_in()? else {
        return Ok(())
    };
    let afk_warn_min = match config.get::<i32>("schedule", "afk_warn_min")? {
        Some(a) => a,
        None => 15,
    };
    let afk_timeout_min = match config.get::<i32>("schedule", "afk_timeout_min")? {
        Some(a) => a,
        None => 5,
    };
     

    if !stub.afk_warn_flag{
        let time_to_afk = stub.last_interaction + Duration::minutes(afk_warn_min as i64);
        if Utc::now() >  time_to_afk{
            //Set Flag Before
            

            //Send message to confirm AFK
            let mut msg = ChannelId(afk_channel)
                .send_message(&ctx, |m| {
                    m.content(format!("<@{}> AFK WARNING",
                    stub.user_id)
                    ).components(|c|
                        c.create_action_row(|r|
                            r.create_button(|b| 
                                b.label("Click Me")
                                .custom_id("afk_clear")
                            )
                        )
                    )
            }).await?;
            {
                use crate::schema::clock_stubs::dsl;
                diesel::update(dsl::clock_stubs.filter(dsl::id.eq(stub.id))).set(dsl::afk_warn_flag.eq(true)).execute(&mut connection)?;


            }
            match msg.await_component_interaction(&*ctx).timeout(Duration::minutes(afk_timeout_min.into()).to_std()?).await{
               Some(_) => {
                    msg.edit(&*ctx,|m| m
                            .set_components(CreateComponents::default())
                            .content("AFK VAILIDATED")
                    ) .await?;
                    use crate::schema::clock_stubs::dsl;
                    diesel::update(dsl::clock_stubs.filter(dsl::id.eq(stub.id))).set(dsl::last_interaction.eq(Utc::now())).execute(&mut connection)?;

               },
               None => {
                   //FAILED AFK CHECK
                   dbinance.unlock(None)?;
                   msg.edit(&*ctx,|m| m
                            .set_components(CreateComponents::default())
                            .content("AFK FAILED ACCOUNT UNLOCKED")
                    ) .await?;

                  
               }

            }


            
 
            //Mark alerted in db
           
        }   
    }

    
        
        
        
        
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
    let order_status = config.get::<u64>("channels", "order_status")?;
    

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
        let Some(quote_balance) = config.get::<String>("trading", "quote_asset_threshold")? else {
            trace!("No Balance Set");
            return Ok(())
        };
        let quote_balance = quote_balance.parse::<f32>().unwrap();
        let last_order = id.parse::<u64>().unwrap();
        let order = account.order_status(&symbol, last_order)?;
        match order.status.as_str(){
            x if x =="FILLED" || x == "CANCELED"  => {
                debug!("Buy Order filled");
                use crate::schema::transactions::dsl;
                if balance.1.free.parse::<f32>()? + balance.1.locked.parse::<f32>()? <= quote_balance{//MAKE CONFIG
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
                    if let Some(channel_id) = order_status{
                        if let Some(stub) = dbinance.is_clocked_in()?{
                            ChannelId(channel_id)
                                .send_message(&ctx, |m| {
                                    m.content(format!("<@{}> Buy order Cleared@{avgPrice} Ready to sell",
                                    stub.user_id
                                ))
                            }).await?;
                        }
                    }
                }else{
                    diesel::update(dsl::transactions.filter(dsl::id.eq(transaction.id))).set(dsl::buyReady.eq(true)).execute(&mut connection)?;
                    if let Some(channel_id) = order_status{
                        if let Some(stub) = dbinance.is_clocked_in()?{
                            ChannelId(channel_id)
                                .send_message(&ctx, |m| {
                                    m.content(format!("<@{}> Buy order Cleared Ready to buy again",
                                    stub.user_id
                                ))
                            }).await?;
                        }
                    }
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
        let Some(base_balance) = config.get::<String>("trading", "base_asset_threshold")? else {
            trace!("No Balance Set");
            return Ok(())
        };
        let base_balance = base_balance.parse::<f32>().unwrap();
        let last_order = id.parse::<u64>().unwrap();
        let order = account.order_status(&symbol, last_order)?;
        match order.status.as_str(){
            x if x == "FILLED" || x ==  "CANCELED" => {
                debug!("Sell order filled");
                use crate::schema::transactions::dsl;
                if balance.0.free.parse::<f32>()? + balance.0.locked.parse::<f32>()? <= base_balance{//MAKE CONFIG
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
                    if let Some(channel_id) = order_status{
                        if let Some(stub) = dbinance.is_clocked_in()?{
                            ChannelId(channel_id)
                                .send_message(&ctx, |m| {
                                    m.content(format!("<@{}> Sell order Cleared@{avgPrice} Ready to buy",
                                    stub.user_id
                                ))
                            }).await?;
                        }
                    }
                }else{
                    diesel::update(dsl::transactions.filter(dsl::id.eq(transaction.id))).set(dsl::sellReady.eq(true)).execute(&mut connection)?;
                    if let Some(channel_id) = order_status{
                        if let Some(stub) = dbinance.is_clocked_in()?{
                            ChannelId(channel_id)
                                .send_message(&ctx, |m| {
                                    m.content(format!("<@{}> Sell order Cleared ready to sell again",
                                    stub.user_id
                                ))
                            }).await?;
                        }
                    }
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
    if !next_reservation.alerted && time_to_alert < now {
        let time_to = next_reservation.start_time - now;

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

    if next_reservation.start_time < now && now < time_to_lock {
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
    


    } else if time_to_lock < now {
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
