use diesel::{QueryDsl, RunQueryDsl};

use crate::{db::establish_connection, models::NewUser};

use diesel::ExpressionMethods;
pub enum Operations {
    UpdateConfig(crate::models::UpdateConfig),
}

pub fn handle(op: Operations) -> Result<(), diesel::result::Error> {
    match op {
        Operations::UpdateConfig(config) => {
            update_config(config)?;
        }
    }
    Ok(())
}

fn update_config(config: crate::models::UpdateConfig) -> Result<(), diesel::result::Error> {
    use crate::schema::configs::dsl::*;
    let mut connection = establish_connection();
    diesel::update(configs.filter(section.eq(config.section)).find(config.key))
        .set(value.eq(config.value))
        .execute(&mut connection)?;
    Ok(())
}
