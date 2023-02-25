use std::{collections::HashMap, sync::Arc};

use arc_swap::{ArcSwapAny, Guard};
use serenity::{
    async_trait,
    builder::CreateApplicationCommand,
    model::prelude::interaction::{
        application_command::{ApplicationCommandInteraction, CommandDataOption},
        autocomplete::AutocompleteInteraction,
    },
    prelude::Context,
};

use crate::config::Config;

pub mod config;
pub mod schedule;
pub mod trading;

#[derive(thiserror::Error, Debug)]
pub enum CommandError {
    #[error("Serenity Error")]
    SerenityError(#[from] serenity::Error),
    #[error("Binance Error")]
    BinanceError(#[from] binance::errors::Error),
    #[error("Diesel Result Error")]
    DieselError(#[from] diesel::result::Error),
    #[error("Diesel Connection Error")]
    DieselConnectionError(#[from] diesel::result::ConnectionError),
    #[error("Error Parsing Market Data {0}")]
    ParsingDataError(String),
    #[error("Awaiting Interaction Timeout {0}")]
    AwaitingInteractionTimeout(String),
    #[error("Incorrect Parameters {0}")]
    IncorrectParameters(String),
}

pub enum AccessLevels {
    ADMIN,
    TRADER,
    ANY,
}

pub struct CommandConfig {
    pub accessLevel: AccessLevels,
    pub ephermal: bool,
    pub fetch_reply: bool,
}
impl Default for CommandConfig {
    fn default() -> Self {
        Self {
            accessLevel: AccessLevels::ANY,
            ephermal: false,
            fetch_reply: true,
        }
    }
}

#[async_trait]

pub trait SlashCommand: Sync + Send {
    fn config(&self) -> CommandConfig;
    async fn run(
        &self,
        interaction: ApplicationCommandInteraction,
        ctx: Context,
        config: Arc<ArcSwapAny<Arc<Config>>>,
    ) -> Result<(), CommandError>;
}

#[async_trait]
pub trait AutoComplete: Sync + Send {
    async fn auto_complete(
        &self,
        interaction: AutocompleteInteraction,
        ctx: Context,
        config: Arc<Config>,
    ) -> Result<(), CommandError>;
}
