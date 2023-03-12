use std::sync::Arc;

use arc_swap::ArcSwap;
use binance::{account::Account, api::Binance, model::{Order, Filters, Balance}, market::Market, general::General,model::Transaction};
use chrono::Utc;
use diesel::{QueryDsl, RunQueryDsl};
use serenity::futures::future::OrElse;
use tracing::{warn, trace, debug, error, instrument};

use crate::{
    config::{Config, ValueType},
    db::establish_connection,
    error::TradingBotError,
    models::{BinanceAccount, ClockStub, Reservation, NewClockStub, NewTransaction, DBTransaction},
    schema::binance_accounts,
};

pub struct BinanceWrapped {
    id: i32,
    pub account: Option<Account>,
    pub market: Market,
    pub general: Option<General>,
    config: Arc<ArcSwap<Config>>,
}
impl BinanceWrapped {
    pub fn new(config: Arc<ArcSwap<Config>>) -> Self {
        Self {
            id: 0,
            account: None,
            market: Binance::new(
                None,
                None,
            ),
            general:None, 
            config,
        }
    }

    //Get for market data or balance nothing with trading
    pub fn get(&self) -> Result<Account, TradingBotError> {
        let Some(account) = self.account.clone() else {
            return Err(TradingBotError::BinanceAccountMissing)
        };
        Ok(account)
    }

    pub fn load_account(&mut self) -> Result<(), TradingBotError> { 
        let config = self.config.load();
        let Some(account_name) = config.get::<String>("trading", "account_name")? else{
            return Err(TradingBotError::ConfigError("No account name set".into()));
        };
        let mut connection = establish_connection();
        use crate::schema::binance_accounts::dsl;
        use diesel::ExpressionMethods;

        //unselect previous account if one
        diesel::update(dsl::binance_accounts.filter(dsl::selected.eq(true)))
            .set(dsl::selected.eq(false))
            .execute(&mut connection)?;

        let db_account = dsl::binance_accounts
            .filter(dsl::name.eq(account_name.clone()))
            .first::<BinanceAccount>(&mut connection)?;
        trace!("Setting Account to {account_name} isPaper{}",db_account.is_paper);
        let account: Account;
        let market: Market;
        let general: General;
        if db_account.is_paper {
           
            account = Binance::new_with_config(
                Some(db_account.api_key.clone()),
                Some(db_account.secret.clone()),
                &binance::config::Config::default()
                    .set_rest_api_endpoint("https://testnet.binance.vision"),
            );
            market = Binance::new_with_config(
                None,
                None,
                &binance::config::Config::default().set_rest_api_endpoint("https://testnet.binance.vision"),
            );
            general = General::new_with_config(
                Some(db_account.api_key),
                Some(db_account.secret),
                &binance::config::Config::default().set_rest_api_endpoint("https://testnet.binance.vision"),
            );
        } else {
            account = Binance::new(Some(db_account.api_key.clone()), Some(db_account.secret.clone()));
            market = Binance::new(
                None,
                None,
            );
            general = General::new(
                Some(db_account.api_key),
                Some(db_account.secret),
            );
            
        }
        diesel::update(dsl::binance_accounts.filter(dsl::name.eq(db_account.name)))
            .set(dsl::selected.eq(true))
            .execute(&mut connection)?;
        self.account = Some(account);
        self.market = market;
        self.general = Some(general);
        self.id = db_account.id;
        Ok(())
    }
}


//Locking Account Unlocking etc
impl BinanceWrapped{

    pub fn is_clocked_in(&self) -> Result<Option<ClockStub>,TradingBotError>{
        use crate::schema::binance_accounts::dsl;
        use diesel::ExpressionMethods;
        let mut connection = establish_connection();
        let result = match dsl::binance_accounts.filter(dsl::selected.eq(true)).select(dsl::active_clock_stub).first::<Option<i32>>(&mut connection){
            Ok(a) => a,
            Err(err) => {
                trace!("Clocking in error {err:?}");
                match err{
                    diesel::result::Error::NotFound => {
                        return Err(TradingBotError::BinanceAccountMissing);
                    }
                    err => {
                        
                        return Err(TradingBotError::DieselError(err));
                    }
                }
              
            }
        };

        let Some(clock_stub_id) = result else{
            return Ok(None);
        };
        use crate::schema::clock_stubs::dsl::*;


        let clock_stub = clock_stubs.filter(id.eq(clock_stub_id)).first::<ClockStub>(&mut connection)?;

        Ok(Some(clock_stub))    
    }
    pub fn get_transaction(&self) -> Result<Option<DBTransaction>,TradingBotError>{
        let mut connection = establish_connection();

        let Some(clock_stub) =  self.is_clocked_in()? else{
            return Ok(None);
        };
        let Some(transcation_id) = clock_stub.active_transaction else{
            trace!("No active transaction");
            return Ok(None)
        };
        let transaction: DBTransaction;
        {
            use diesel::ExpressionMethods;
            use crate::schema::transactions::dsl;
            transaction = dsl::transactions.filter(dsl::id.eq(transcation_id)).first::<DBTransaction>(&mut connection)?;
        }
        Ok(Some(transaction))

    }

    pub fn is_reserved(&self) -> Result<Option<Reservation>,TradingBotError>{
        use crate::schema::binance_accounts::dsl;
        use diesel::ExpressionMethods;
        let mut connection = establish_connection();
        let result = dsl::binance_accounts.filter(dsl::selected.eq(true)).select(dsl::active_reservation).first::<Option<i32>>(&mut connection)?;

        let Some(reservation_id) = result else{
            return Ok(None);
        };
        use crate::schema::reservations::dsl::*;


        let reservation = reservations.filter(id.eq(reservation_id)).first::<Reservation>(&mut connection)?;

        Ok(Some(reservation))    
    }
    #[instrument(skip(self))]
    pub fn unlock(&self,user_id: Option<i64>) -> Result<(),TradingBotError>{
        trace!("Checking Clock");

        let is_clocked_in = self.is_clocked_in()?;
        let Some(clock_stub) = is_clocked_in else{
            return Err(TradingBotError::LockingBinanceAccount(format!("Account is not locked")));
        };
        trace!("Checking ID");

        if let Some(id) = user_id{
            if clock_stub.user_id != id{
                return Err(TradingBotError::LockingBinanceAccount(format!("Account is locked by someone else")));
            }
        }
        trace!("Clocking out User");


        let mut connection = establish_connection();

        //Clock out stub setting out time

        {
            use crate::schema::clock_stubs::dsl;
            use diesel::ExpressionMethods;
            diesel::update(dsl::clock_stubs.filter(dsl::id.eq(clock_stub.id)))
            .set(dsl::end_time.eq(Some(Utc::now().naive_utc())))
            .execute(&mut connection)?;
            trace!("ClockStub end time set")

        }


        {
            trace!("unlocking account");

            use crate::schema::binance_accounts::dsl;
            use diesel::ExpressionMethods;
            diesel::update(dsl::binance_accounts.filter(dsl::selected.eq(true)))
            .set(dsl::active_clock_stub.eq::<Option<i32>>(None))
            .execute(&mut connection)?;

            debug!("account unlocked");
        }
        Ok(())
    }

    #[instrument(skip(self))]
    pub fn lock(&self,user_id: i64) -> Result<(),TradingBotError>{
        //check is_reserved() continue if reserved by user or if no reservation
        trace!("Checking Reservation");
        let is_reservation = self.is_reserved()?;
        if let Some(reservation) = is_reservation{
            if reservation.user_id != user_id{
                return Err(TradingBotError::LockingBinanceAccount(format!("Account is reserved")));
            }
        }
        trace!("Checking Clock");

        //check is_clocked in continue player is not clocked in and no one else is clocked in unless their is a reservation for the player clocking in
        let is_clocked_in = self.is_clocked_in()?;
        
        if let Some(clock_stub) = is_clocked_in{
            if clock_stub.user_id == user_id{
                return Err(TradingBotError::LockingBinanceAccount(format!("Account is locked by you")));
            }else if is_reservation.is_none(){
                return Err(TradingBotError::LockingBinanceAccount(format!("Account is locked by someone else")));
            }else{
                trace!("Unlocking user since other user is attempting to clock in with active reservation");
                self.unlock(Some(clock_stub.user_id))?;
            }
        }
        trace!("Locking account for user");

        

        let mut connection = establish_connection();

        let clock_stub: ClockStub;
        {

            //Create ClockStub
            use crate::schema::clock_stubs::dsl;

            clock_stub = diesel::insert_into(dsl::clock_stubs).values(NewClockStub{
                start_time: Utc::now(),
                user_id,
                last_interaction: Utc::now(),
            }).get_result::<ClockStub>(&mut connection)?.clone();
            trace!("Clock stub created");
        }
        {
            //Set ClockStub as Active

            use crate::schema::binance_accounts::dsl;
            use diesel::ExpressionMethods;
            diesel::update(dsl::binance_accounts.filter(dsl::selected.eq(true)))
            .set(dsl::active_clock_stub.eq(Some(clock_stub.id)))
            .execute(&mut connection)?;
            debug!("Clock stub Active and account is locked");

            
        }
        Ok(())
    }

}



//Buying Selling Order etc
impl BinanceWrapped{
    pub fn get_balance(&self) -> Result<(Balance,Balance),TradingBotError>{
        let Some(account) = self.account.as_ref()  else{
            return Err(TradingBotError::BinanceAccountMissing);
        };
        let symbol = match self.config.load().get::<String>("trading", "symbol")? {
            Some(symbol) => symbol,
            None => "BTCUSDT".into(),
        };
        let general = binance::general::General{
            client:account.client.clone()
        };
        let symbol_info = general.get_symbol_info(symbol)?;
        let quote_balance = account.get_balance(symbol_info.quote_asset)?;
        let base_balance = account.get_balance(symbol_info.base_asset)?;
        return Ok((base_balance,quote_balance))
    }
    #[instrument(skip(self))]
    pub fn buy(&self,price:Option<f32>,percentage: Option<f64>) -> Result<Transaction,TradingBotError>{
        let Some(stub) = self.is_clocked_in()? else {
            return Err(TradingBotError::NotClockedIn(String::new()))
        };
        let opt_transaction = self.get_transaction()?;
        if let Some(transaction) = &opt_transaction{
            if transaction.buyAvgPrice.is_some() || !transaction.buyReady{
                return Err(TradingBotError::ActiveTransaction("Must sell before buying or wait for previous order to settle".into()))
            }
        }
        let Some(account) = self.account.as_ref()  else{
            error!("Account is missing");
            return Err(TradingBotError::BinanceAccountMissing);
        };
        let symbol = match self.config.load().get::<String>("trading", "symbol")? {
            Some(symbol) => symbol,
            None => "BTCUSDT".into(),
        };
        let Some(general) = self.general.as_ref()  else{
            error!("General is missing but not account");
            return Err(TradingBotError::BinanceAccountMissing);
        };
        let symbol_info = general.get_symbol_info(&symbol)?;
        trace!("Getting Symbol Info for {}",symbol);
        dbg!(symbol_info.clone());
        let Ok(balance) = account.get_balance(symbol_info.quote_asset)?.free.parse::<f64>() else{
            return Err(TradingBotError::ParsingDataError("Could no parse balance".into()));
        };
        let adjusted_balance = format!("{:.5}",(balance - 1.0) * percentage.unwrap_or(1.0)).parse::<f64>().unwrap();
        let order: Transaction;
        if let Some(price) = price{
            let price = format!("{:.1$}",price,2).parse::<f64>().unwrap();


            let quantity = format!("{:.5}",adjusted_balance/(price as f64)).parse::<f64>().unwrap();
            if quantity < 0.0{
                return Err(TradingBotError::ParsingDataError("Insuffecient balance".into()))
            }

            trace!("Sending buy limit order for %{} of account with Qty:{} @{}",percentage.unwrap_or(1.0)*100.,quantity,price);
            order = account.limit_buy(symbol, quantity, price as f64)?;

        }else{
            trace!("Sending buy market order for %{} of account",percentage.unwrap_or(1.0)*100.);
            order = account.market_buy_using_quote_quantity(&symbol, adjusted_balance)?;
        }
        //file transaction
        if opt_transaction.is_none(){
            let transaction: DBTransaction;
            {
                use crate::schema::transactions::dsl;
                let mut connection = establish_connection();
                transaction = diesel::insert_into(dsl::transactions).values(NewTransaction{
                    clock_stub_id: stub.id,
                    buyOrderIds: format!("{}",order.order_id),
                    sellOrderIds: "".into(),
                }).get_result(&mut connection)?;
                trace!("Transaction Created")
            }
            {
                use crate::schema::clock_stubs::dsl;
                use diesel::ExpressionMethods;
                let mut connection = establish_connection();
                diesel::update(dsl::clock_stubs.filter(dsl::id.eq(stub.id))).set(dsl::active_transaction.eq(Some(transaction.id))).execute(&mut connection)?;
                trace!("Transaction Linked")
            }

        }else if let Some(transaction) = &opt_transaction{
            use crate::schema::transactions::dsl;
            use diesel::ExpressionMethods;
            let mut connection = establish_connection();

            diesel::update(dsl::transactions.filter(dsl::id.eq(transaction.id))).set((dsl::buyReady.eq(false),dsl::buyOrderIds.eq(format!("{},{}",transaction.buyOrderIds,order.order_id)))).execute(&mut connection)?;



        }
        
        



        return Ok(order);
    }
    #[instrument(skip(self))]
    pub fn sell(&self,price:Option<f32>,percentage: Option<f64>) -> Result<Transaction,TradingBotError>{
        let Some(stub) = self.is_clocked_in()? else {
            return Err(TradingBotError::NotClockedIn(String::new()))
        };
        let Some(transaction) = self.get_transaction()? else {
            return Err(TradingBotError::ActiveTransaction("Must buy before selling".into()));
        };
       
        if transaction.buyAvgPrice.is_none() || !transaction.sellReady{
            return Err(TradingBotError::ActiveTransaction("Must wait previous order to settle before selling".into()));
        }
        let Some(account) = self.account.as_ref()  else{
            error!("Account is missing");
            return Err(TradingBotError::BinanceAccountMissing);
        };
        let symbol = match self.config.load().get::<String>("trading", "symbol")? {
            Some(symbol) => symbol,
            None => "BTCUSDT".into(),
        };
        let Some(general) = self.general.as_ref()  else{
            error!("General is missing but not account");
            return Err(TradingBotError::BinanceAccountMissing);
        };
        let symbol_info = general.get_symbol_info(&symbol)?;
        trace!("Getting Symbol Info for {}",symbol);
        dbg!(symbol_info.clone());
        let Ok(balance) = account.get_balance(symbol_info.base_asset)?.free.parse::<f64>() else{
            return Err(TradingBotError::ParsingDataError("Could no parse balance".into()));
        };
        let adjusted_balance = format!("{:.5}",(balance - 0.00001) * percentage.unwrap_or(1.0)).parse::<f64>().unwrap();
        let order: Transaction;


        if let Some(price) = price{
            let price = format!("{:.1$}",price,2).parse::<f64>().unwrap();
            trace!("Sending sell limit order for %{} of account with Qty:{} @{}",percentage.unwrap_or(1.0)*100.,adjusted_balance,price);
            order = account.limit_sell(symbol, adjusted_balance, price)?;

        }else{
            trace!("Sending sell market order for %{} of account with balance {}",percentage.unwrap_or(1.0)*100.,adjusted_balance);
            order = account.market_sell(&symbol, adjusted_balance)?;
        }

        {
            use crate::schema::transactions::dsl;
            use diesel::ExpressionMethods;
            let mut connection = establish_connection();
            if transaction.sellOrderIds == ""{
                diesel::update(dsl::transactions.filter(dsl::id.eq(transaction.id))).set((dsl::sellReady.eq(false),dsl::sellOrderIds.eq(format!("{}",order.order_id)))).execute(&mut connection)?;
            }else{
                diesel::update(dsl::transactions.filter(dsl::id.eq(transaction.id))).set((dsl::sellReady.eq(false),dsl::sellOrderIds.eq(format!("{},{}",transaction.sellOrderIds,order.order_id)))).execute(&mut connection)?;
            }
            trace!("Sell Order ID Set");
        }



        return Ok(order);
    }

}
