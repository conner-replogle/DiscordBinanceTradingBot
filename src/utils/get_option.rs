use std::{any::type_name, error::Error, slice::Iter, str::FromStr};

use serenity::model::prelude::interaction::application_command::CommandDataOption;

use crate::commands::CommandError;

pub fn get_option<T: FromStr>(
    options: &mut Iter<CommandDataOption>,
    name: &str,
) -> Result<T, CommandError> {
    let Some(option) = options.find(|o| o.name == name) else{
        return Err(CommandError::IncorrectParameters(format!("Could not find value of {name}")));
    };
    let Some(value) = option.value.as_ref() else {
        return  Err(CommandError::IncorrectParameters(format!("Could not unwrap value of {name}")));
    };

    let Ok(data) = value.as_str().unwrap().to_string().parse::<T>() else{
        return Err(CommandError::IncorrectParameters(format!("Failed to parse type {} of {}",type_name::<T>(),name)));
    };
    return Ok(data);
}
