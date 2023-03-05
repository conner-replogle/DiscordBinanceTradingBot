mod binance_wrapped;
mod commands;
mod config;
mod db;
mod error;
mod event_handler;
mod interval_handler;
mod models;
mod ops;
mod schedule;
mod schema;
mod utils;
use arc_swap::ArcSwap;
use binance::account::Account;
use binance::api::Binance;
use binance::market::Market;
use binance_wrapped::BinanceWrapped;
use commands::config::status::StatusCommand;
use config::Config;
use dotenv::dotenv;
use serenity::async_trait;
use serenity::model::application::interaction::{Interaction, InteractionResponseType};
use serenity::model::gateway::Ready;
use serenity::model::id::GuildId;
use serenity::prelude::*;
use std::env;
use std::sync::Arc;
use tracing::{debug, error, info, instrument, span, trace, Level};
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};
use tokio::sync::RwLock;

use crate::commands::config::create_user::CreateUserCommand;
use crate::commands::trading::balance::BalanceCommand;
use crate::commands::trading::buy::BuyCommand;
use crate::commands::trading::sell::SellCommand;
use crate::config::ValueType;
use crate::db::establish_connection;
use crate::event_handler::Handler;
use crate::utils::message::send_status;

#[tokio::main]
async fn main() {
    //pull enviorment vars from .env
    dotenv().ok();
    //setup logging
    tracing_subscriber::registry()
        .with(fmt::layer().with_file(true).with_line_number(true))
        .with(EnvFilter::from_default_env())
        .init();

    info!("Initialized");

    let config = Arc::new(ArcSwap::from(Arc::new(
        config::Config::first_setup().unwrap(),
    )));

    // Configure the client with your Discord bot token in the environment.
    let token = env::var("TOKEN").expect("Expected a token in the environment");

    //Connect to binance accounts
    let mut acc = BinanceWrapped::new(config.clone());
    acc.load_account();

    let binance = Arc::new(RwLock::from(acc));
    let market: Market = Binance::new_with_config(
        None,
        None,
        &binance::config::Config::default().set_rest_api_endpoint("https://testnet.binance.vision"),
    );

    // Build our client.
    let mut client = Client::builder(token, GatewayIntents::empty())
        .event_handler(Handler::new(binance, config, market))
        .await
        .expect("Error creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform
    // exponential backoff until it reconnects.

    let thread = tokio::spawn(async move {
        if let Err(err) = client.start().await {
            error!("Discord Bot Error {err}")
        }
    });

    thread.await.unwrap()
}
