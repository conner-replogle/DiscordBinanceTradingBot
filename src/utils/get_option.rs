use std::{any::type_name, error::Error, slice::Iter, str::FromStr};
use std::fmt::Debug;
use serenity::model::prelude::interaction::application_command::CommandDataOption;
use tracing::trace;

use crate::commands::CommandError;

pub fn get_option<T: FromStr + Debug>(
    options: &mut Iter<CommandDataOption>,
    name: &str,
) -> Result<T, CommandError> {
    let Some(option) = options.find(|o| o.name == name) else{
        return Err(CommandError::IncorrectParameters(format!("Could not find value of {name}")));
    };
    let Some(value) = option.value.as_ref() else {
        
        return  Err(CommandError::IncorrectParameters(format!("Could not unwrap value of {name}")));
    };
    let mut value_str = value.to_string();
    if value.is_string(){
        value_str.remove(0);
        value_str.pop();

    }

    let Ok(data) = value_str.parse::<T>() else{
        return Err(CommandError::IncorrectParameters(format!("Failed to parse value {} to type {} of {}",value_str,type_name::<T>(),name)));
    };
    trace!("Parsed Option: {name} with value of {data:?}");
    return Ok(data);
}
