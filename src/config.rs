use std::{collections::HashMap, hash::Hash, str::FromStr};

use diesel::{
    result::{self, DatabaseErrorKind},
    QueryDsl, RunQueryDsl, SqliteConnection,
};
use serenity::json::Value;
use tracing::{debug, error, info, log::warn, trace};

use crate::{
    db::establish_connection,
    error::TradingBotError,
    models::{self, NewConfig},
};
use diesel::ExpressionMethods;

#[derive(Debug, Clone, Copy)]
pub enum ValueType {
    STRING,
    INT,
    BIGINT,
    BOOL,
}
impl ValueType {
    fn to_i32(self) -> i32 {
        match self {
            ValueType::STRING => 0,
            ValueType::INT => 1,
            ValueType::BIGINT => 2,
            ValueType::BOOL => 3,
        }
    }
    fn from_i32(n: i32) -> Self {
        match n {
            0 => ValueType::STRING,
            1 => ValueType::INT,
            2 => ValueType::BIGINT,
            3 => ValueType::BOOL,
            _ => {
                panic!("Tried to parse type got {n}")
            }
        }
    }
}

fn insert_config(
    config: NewConfig,
    connection: &mut SqliteConnection,
) -> Result<bool, result::Error> {
    use crate::schema::configs::dsl::*;
    if let Err(err) = diesel::insert_into(configs)
        .values(&config)
        .execute(connection)
    {
        match err {
            result::Error::DatabaseError(err, _) => {
                if let DatabaseErrorKind::UniqueViolation = err {
                    warn!(
                        "Unique Violation warning updating config for {}/{} ",
                        config.section, config.key
                    );
                    return Ok(false);
                }
            }
            _ => {}
        }
        return Err(err);
    }
    info!(
        "Created Config Entry for {}/{} ",
        config.section, config.key
    );

    Ok(true)
}
#[derive(Debug,Clone)]
pub struct CachedConfig{
    value: Option<String>,
    description: String

}

#[derive(Debug, Clone)]
pub struct Config {
    pub cached: HashMap<String, HashMap<String, CachedConfig>>,
    pub cached_types: HashMap<String, HashMap<String, ValueType>>,
}
impl Config {
    pub fn get<T: FromStr>(&self, section: &str, key: &str) -> Result<Option<T>, TradingBotError> {
        //trace!("Getting {section}/{key}");
        if let Some(section_map) = self.cached.get(section) {
            let result = section_map.get(key);
            match result {
                Some(a) => {
                    let Some(value) = &a.value else{
                        warn!("{section}/{key} was null");
                        return Ok(None);
                    };
                    let typed = value.parse::<T>();
                    let Ok(value)  = typed else{
                        return Ok(None);
                    };
                    return Ok(Some(value));
                }
                None => {
                    warn!("{section}/{key} did not exist");
                    return Ok(None);
                }
            }
        } else {
            error!("Section does not exist for {section}/{key}");
            Err(TradingBotError::ConfigError(
                "Section does not exist for {section}/{key}".into(),
            ))
        }
    }
    pub fn get_type(&self, section: &str, key: &str) -> Result<ValueType, TradingBotError> {
        if let Some(section_map) = self.cached_types.get(section) {
            let result = section_map.get(key);
            match result {
                Some(value) => return Ok(*value),
                None => {
                    warn!("{section}/{key} was null");
                    return Err(TradingBotError::ConfigError(
                        "Type doesn't exist for {section}/{key}".into(),
                    ));
                }
            }
        } else {
            error!("Section does not exist for {section}/{key}");
            Err(TradingBotError::ConfigError(
                "Section does not exist for {section}/{key}".into(),
            ))
        }
    }
    pub fn first_setup() -> Result<Self, diesel::result::Error> {
        let mut connection = establish_connection();

        insert_config(
            models::NewConfig {
                section: "roles",
                key: "admin",
                value_type: ValueType::BIGINT.to_i32(),
                value: None,
                description: "Role ID for admins",
            },
            &mut connection,
        )?;

        insert_config(
            models::NewConfig {
                section: "roles",
                key: "trader",
                value_type: ValueType::BIGINT.to_i32(),
                value: None,
                description: "Role ID for traders",
            },
            &mut connection,
        )?;

        insert_config(
            models::NewConfig {
                section: "trading",
                key: "account_name",
                value_type: ValueType::STRING.to_i32(),
                value: None,
                description: "Active Account Name DO NOT SET IN /set_config only /account set command",
            },
            &mut connection,
        )?;

        insert_config(
            models::NewConfig {
                section: "trading",
                key: "symbol",
                value_type: ValueType::STRING.to_i32(),
                value: Some("BTCUSDT"),
                description: "The symbol pair you are trading",
            },
            &mut connection,
        )?;

        insert_config(
            models::NewConfig {
                section: "trading",
                key: "buy_timeout_s",
                value_type: ValueType::INT.to_i32(),
                value: Some(&60.to_string()),
                description: "The time for the buy command to stop waiting for a response",
            },
            &mut connection,
        )?;
        insert_config(
            models::NewConfig {
                section: "trading",
                key: "sell_timeout_s",
                value_type: ValueType::INT.to_i32(),
                value: Some(&60.to_string()),
                description: "The time for the sell command to stop waiting for a response",
            },
            &mut connection,
        )?;
        insert_config(
            models::NewConfig {
                section: "trading",
                key: "price_command_price_len",
                value_type: ValueType::INT.to_i32(),
                value: Some(&100.to_string()),
                description: "The max length of intervals show on the /price command",
            },
            &mut connection,
        )?;

        insert_config(
            models::NewConfig {
                section: "general",
                key: "command_timeout",
                value_type: ValueType::INT.to_i32(),
                value: Some(&(5 * 60).to_string()),
                description: "The overall timeout for commands before the thread is killed",
            },
            &mut connection,
        )?;

        insert_config(
            models::NewConfig {
                section: "schedule",
                key: "reservation_interval_min",
                value_type: ValueType::INT.to_i32(),
                value: Some(&(15).to_string()),
                description: "The interval in between avaliable reservations DO NOT CHANGE WITHOUT CLEARING RESERVATIONS"

            },
            &mut connection,
        )?;
        insert_config(
            models::NewConfig {
                section: "schedule",
                key: "timezone",
                value_type: ValueType::STRING.to_i32(),
                value: Some("America/Chicago"),
                description: "Timezone https://en.wikipedia.org/wiki/List_of_tz_database_time_zones",
            },
            &mut connection,
        )?;

        insert_config(
            models::NewConfig {
                section: "schedule",
                key: "reservation_alert_min",
                value_type: ValueType::INT.to_i32(),
                value: Some(&(15).to_string()),
                description: "Amount of mins before a reservation to alert the user",
            },
            &mut connection,
        )?;
        insert_config(
            models::NewConfig {
                section: "schedule",
                key: "reservation_lock_min",
                value_type: ValueType::INT.to_i32(),
                value: Some(&(15).to_string()),
                description: "Amount of mins after a reservation that the account is locked",
            },
            &mut connection,
        )?;

        insert_config(
            models::NewConfig {
                section: "channels",
                key: "reservation_alert",
                value_type: ValueType::BIGINT.to_i32(),
                value: None,
                description: "Channel ID for reservation alerts to be sent in",
            },
            &mut connection,
        )?;
        insert_config(
            models::NewConfig {
                section: "trading",
                key: "quote_asset_threshold",
                value_type: ValueType::STRING.to_i32(),
                value: Some("10".into()),
                description: "Amount of money in quote balance for it to count as a full buy",
            },
            &mut connection,
        )?;
        insert_config(
            models::NewConfig {
                section: "trading",
                key: "base_asset_threshold",
                value_type: ValueType::STRING.to_i32(),
                value: Some("0.0001".into()),
                description: "Amount of money in base balance for it to count as a full sell",
            },
            &mut connection,
        )?;
        insert_config(
            models::NewConfig {
                section: "channels",
                key: "order_status",
                value_type: ValueType::BIGINT.to_i32(),
                value: None,
                description: "The channel to send order clearing notifications None == OFF",
            },
            &mut connection,
        )?;
        insert_config(
            models::NewConfig {
                section: "channels",
                key: "afk_channel",
                value_type: ValueType::BIGINT.to_i32(),
                value: None,
                description: "The channel to send afk checks too None == OFF",
            },
            &mut connection,
        )?;
        insert_config(
            models::NewConfig {
                section: "schedule",
                key: "afk_warn_min",
                value_type: ValueType::INT.to_i32(),
                value: Some("15"),
                description: "Time in mins of inactivity before a afk check",
            },
            &mut connection,
        )?;
        insert_config(
            models::NewConfig {
                section: "schedule",
                key: "afk_timeout_min",
                value_type: ValueType::INT.to_i32(),
                value: Some("5"),
                description: "Time to click the button before being timed out",
            },
            &mut connection,
        )?;




        Self::load()
    }

    pub fn load() -> Result<Self, diesel::result::Error> {
        use crate::schema::configs::dsl::*;
        let mut connection = establish_connection();
        let results = configs.load::<models::Config>(&mut connection)?;
        let mut cached: HashMap<String, HashMap<String, CachedConfig>> = HashMap::new();
        let mut cached_types: HashMap<String, HashMap<String, ValueType>> = HashMap::new();

        results.iter().for_each(|config| {
            let vtype = ValueType::from_i32(config.value_type);
            let other = cached.get_mut(&config.section);
            if let Some(a) = other {
                a.insert(config.key.clone(), CachedConfig{
                  value:config.value.clone(),
                  description: config.description.clone()
                });
            } else {
                cached.insert(config.section.clone(), HashMap::new());
                let a = cached.get_mut(&config.section).unwrap();
                a.insert(config.key.clone(), CachedConfig{
                    value:config.value.clone(),
                    description: config.description.clone()
                  });
            }

            let other = cached_types.get_mut(&config.section);

            if let Some(a) = other {
                a.insert(config.key.clone(), vtype);
            } else {
                cached_types.insert(config.section.clone(), HashMap::new());
                let a = cached_types.get_mut(&config.section).unwrap();
                a.insert(config.key.clone(), vtype);
            }
        });
        debug!("Cache {cached:?}");
        Ok(Config {
            cached,
            cached_types,
        })
    }
}
