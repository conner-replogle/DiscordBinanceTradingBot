#[derive(thiserror::Error, Debug)]
pub enum TradingBotError {
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
    #[error("Locking Binance Account {0}")]
    LockingBinanceAccount(String),
    #[error("Must Be clocked in {0}")]
    NotClockedIn(String),

    #[error(" Transaction Error{0}")]
    ActiveTransaction(String),

    #[error("Config Error {0}")]
    ConfigError(String),
    #[error("Make sure Binance account is properly setup")]
    BinanceAccountMissing,
}
