use diesel::{QueryDsl, RunQueryDsl};

use crate::models::User;
use crate::schema::users::dsl::users;
use crate::{db::establish_connection, models::NewUser};

pub enum Operations<'a> {
    CreateUser(NewUser<'a>),
}

pub fn handle(op: Operations) -> Result<(), diesel::result::Error> {
    match op {
        Operations::CreateUser(user) => {
            create_user(user)?;
        }
    }
    Ok(())
}

fn create_user(user: NewUser) -> Result<(), diesel::result::Error> {
    let mut connection = establish_connection();
    diesel::insert_into(users)
        .values(&user)
        .execute(&mut connection)?;
    Ok(())
}
pub fn find_user(user_id: i64) -> Result<User, diesel::result::Error> {
    use crate::schema::users::dsl::*;
    use diesel::ExpressionMethods;
    let mut connection = establish_connection();
    return users.filter(id.eq(user_id)).first::<User>(&mut connection);
}
