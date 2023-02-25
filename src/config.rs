use std::{collections::HashMap, hash::Hash};

use diesel::{
    result::{self, DatabaseErrorKind},
    QueryDsl, RunQueryDsl, SqliteConnection,
};
use serenity::json::Value;
use tracing::{debug, error, info, log::warn, trace};

use crate::{
    db::establish_connection,
    models::{self, NewConfig},
};
use diesel::ExpressionMethods;

#[derive(Debug, Clone)]
pub enum ValueType {
    STRING(Option<String>),
    INT(Option<i32>),
    BIGINT(Option<i64>),
    BOOL(Option<bool>),
    NULL,
}
impl ValueType {
    fn to_i32(self) -> i32 {
        match self {
            ValueType::STRING(_) => 0,
            ValueType::INT(_) => 1,
            ValueType::BIGINT(_) => 2,
            ValueType::BOOL(_) => 3,
            ValueType::NULL => -1,
        }
    }
}
impl ValueType {
    fn from(value: Option<String>, value_type: i32) -> Self {
        match value_type {
            x if x == (ValueType::STRING(None).to_i32()) => ValueType::STRING(value),
            x if x == (ValueType::INT(None).to_i32()) => {
                if let Some(num_str) = value {
                    let num = num_str.parse::<i32>().unwrap();
                    ValueType::INT(Some(num))
                } else {
                    ValueType::INT(None)
                }
            }
            x if x == (ValueType::BIGINT(None).to_i32()) => {
                if let Some(num_str) = value {
                    let num = num_str.parse::<i64>().unwrap();
                    ValueType::BIGINT(Some(num))
                } else {
                    ValueType::BIGINT(None)
                }
            }
            x if x == (ValueType::BOOL(None).to_i32()) => {
                if let Some(num_str) = value {
                    let num = num_str.parse::<bool>().unwrap();
                    ValueType::BOOL(Some(num))
                } else {
                    ValueType::BOOL(None)
                }
            }
            _ => ValueType::NULL,
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

#[derive(Debug, Clone)]
pub struct Config {
    pub cached: HashMap<String, HashMap<String, ValueType>>,
}
impl Config {
    pub fn get(&self, section: &str, key: &str) -> ValueType {
        trace!("Getting {section}/{key}");
        if let Some(section_map) = self.cached.get(section) {
            section_map.get(key).unwrap_or(&ValueType::NULL).clone()
        } else {
            ValueType::NULL
        }
    }
    pub fn first_setup() -> Result<Self, diesel::result::Error> {
        let mut connection = establish_connection();

        insert_config(
            models::NewConfig {
                section: "roles",
                key: "admin",
                value_type: ValueType::BIGINT(None).to_i32(),
                value: None,
            },
            &mut connection,
        )?;

        insert_config(
            models::NewConfig {
                section: "roles",
                key: "trader",
                value_type: ValueType::BIGINT(None).to_i32(),
                value: None,
            },
            &mut connection,
        )?;

        insert_config(
            models::NewConfig {
                section: "trading",
                key: "account_api",
                value_type: ValueType::STRING(None).to_i32(),
                value: None,
            },
            &mut connection,
        )?;

        insert_config(
            models::NewConfig {
                section: "trading",
                key: "account_secret",
                value_type: ValueType::STRING(None).to_i32(),
                value: None,
            },
            &mut connection,
        )?;

        insert_config(
            models::NewConfig {
                section: "trading",
                key: "paperAccount",
                value_type: ValueType::BOOL(None).to_i32(),
                value: None,
            },
            &mut connection,
        )?;

        insert_config(
            models::NewConfig {
                section: "trading",
                key: "buy_timeout_s",
                value_type: ValueType::INT(None).to_i32(),
                value: Some(&60.to_string()),
            },
            &mut connection,
        )?;
        insert_config(
            models::NewConfig {
                section: "trading",
                key: "sell_timeout_s",
                value_type: ValueType::INT(None).to_i32(),
                value: Some(&60.to_string()),
            },
            &mut connection,
        )?;
        insert_config(
            models::NewConfig {
                section: "trading",
                key: "price_command_price_len",
                value_type: ValueType::INT(None).to_i32(),
                value: Some(&100.to_string()),
            },
            &mut connection,
        )?;

        insert_config(
            models::NewConfig {
                section: "general",
                key: "command_timeout",
                value_type: ValueType::INT(None).to_i32(),
                value: Some(&(5 * 60).to_string()),
            },
            &mut connection,
        )?;

        insert_config(
            models::NewConfig {
                section: "schedule",
                key: "reservation_interval_min",
                value_type: ValueType::INT(None).to_i32(),
                value: Some(&(15).to_string()),
            },
            &mut connection,
        )?;

        Self::load()
    }

    pub fn load() -> Result<Self, diesel::result::Error> {
        use crate::schema::configs::dsl::*;
        let mut connection = establish_connection();
        let results = configs.load::<models::Config>(&mut connection)?;
        let mut cached: HashMap<String, HashMap<String, ValueType>> = HashMap::new();
        results.iter().for_each(|config| {
            let config_value = ValueType::from(config.value.clone(), config.value_type);
            let other = cached.get_mut(&config.section);
            if let Some(a) = other {
                a.insert(config.key.clone(), config_value);
            } else {
                cached.insert(config.section.clone(), HashMap::new());
                let a = cached.get_mut(&config.section).unwrap();
                a.insert(config.key.clone(), config_value);
            }
        });
        debug!("Cache {cached:?}");
        Ok(Config { cached })
    }
}
