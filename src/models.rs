use diesel::prelude::*;

use crate::schema::configs;
use crate::schema::reservations;
use crate::schema::users;

#[derive(Insertable)]
#[diesel(table_name = users)]
pub struct NewUser<'a> {
    pub id: i64,
    pub tag: &'a str,
}

#[derive(Identifiable, Queryable)]
#[diesel(table_name = users)]
pub struct User {
    pub id: i64,
    pub tag: String,
}

#[derive(Insertable)]
#[diesel(table_name = configs)]
pub struct NewConfig<'a> {
    pub section: &'a str,
    pub key: &'a str,
    pub value_type: i32,
    pub value: Option<&'a str>,
}

#[derive(Queryable, PartialEq, Selectable, Debug)]
pub struct Config {
    pub section: String,
    pub key: String,
    pub value_type: i32,
    pub value: Option<String>,
}
#[derive(Insertable)]
#[diesel(table_name = reservations)]
pub struct NewReservation {
    pub start_time: chrono::NaiveDateTime,
    pub end_time: chrono::NaiveDateTime,
    pub user_id: i64,
}

#[derive(Identifiable,Clone, Copy, Queryable, PartialEq, Selectable, Debug, Associations)]
#[diesel(belongs_to(User))]
#[diesel(table_name = reservations)]
pub struct Reservation {
    pub id: i32,
    pub start_time: chrono::NaiveDateTime,
    pub end_time: chrono::NaiveDateTime,
    pub user_id: i64,
}
