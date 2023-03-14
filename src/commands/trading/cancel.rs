use arc_swap::{ArcSwap, ArcSwapAny, Guard};
use binance::account::Account;
use diesel::{query_dsl::methods::FilterDsl, QueryDsl};
use serenity::{client::Context, model::prelude::{component::ButtonStyle, command::CommandOptionType}};
use std::{sync::Arc, thread, time::Duration, f32::consts::E};
use tracing::{debug, warn, trace};
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

pub(crate) const COMMAND_NAME: &'static str = "cancel";
pub(crate) fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name(COMMAND_NAME)
        .description("cancel the active order")

}

pub struct CancelCommand {
    binance: Arc<RwLock<BinanceWrapped>>,
}

impl CancelCommand {
    pub fn new(binance: Arc<RwLock<BinanceWrapped>>) -> Self {
        CancelCommand { binance }
    }
}
#[async_trait]
impl SlashCommand for CancelCommand {
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
        debug!("Executing Orders Command");
        let binance = self.binance.read().await;
    
        let symbol = &match config.get::<String>("trading", "symbol")?{
            Some(n) => n,
            None => "BTCUSDT".into()
        };

        let Some(account) = &binance.account else{
            return Err(CommandError::TradingBotError(TradingBotError::BinanceAccountMissing))
        };
        let Some(active_transaction) = binance.get_transaction()? else{
            return Err(CommandError::TradingBotError(TradingBotError::ActiveTransaction("No active Transaction".into())))
        };

        if active_transaction.buyAvgPrice.is_none(){
            let mut ids = active_transaction.buyOrderIds.split(',');
            let id = ids.last().unwrap();
            trace!("Last Buy Order {}",id);
            if id ==""{
                return Err(CommandError::ParsingDataError("No last buy order".into()))

            }
            let last_order = id.parse::<u64>().unwrap();
            account.cancel_order(symbol, last_order)?;

        }else{
            let mut ids = active_transaction.sellOrderIds.split(',');
            let id = ids.last().unwrap();
            trace!("Last sell Order {}",id);
            if id ==""{
                return Err(CommandError::ParsingDataError("No last sell order".into()))
            }
            let last_order = id.parse::<u64>().unwrap();
            account.cancel_order(symbol, last_order)?;

        }

        interaction.edit_original_interaction_response(&ctx.http, |i| i.content("Done")).await?;
        

        
        
        

        Ok(())
    }
}
