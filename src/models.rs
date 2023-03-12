use diesel::prelude::*;

use crate::schema::binance_accounts;
use crate::schema::configs;
use crate::schema::reservations;
use crate::schema::users;
use crate::schema::clock_stubs;
use crate::schema::transactions;


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
    pub description: &'a str,
    pub value: Option<&'a str>,
}

#[derive(Queryable, PartialEq, Selectable, Debug)]
pub struct Config {
    pub section: String,
    pub key: String,
    pub value_type: i32,
    pub description: String,
    pub value: Option<String>,
}
#[derive(Queryable, PartialEq, Selectable, Debug)]
#[diesel(table_name = configs)]

pub struct UpdateConfig {
    pub section: String,
    pub key: String,
    pub value: Option<String>,
}
#[derive(Insertable)]
#[diesel(table_name = reservations)]
pub struct NewReservation {
    pub start_time: chrono::NaiveDateTime,
    pub end_time: chrono::NaiveDateTime,
    pub user_id: i64,
}

#[derive(Identifiable, Clone, Copy, Queryable, PartialEq, Selectable, Debug, Associations)]
#[diesel(belongs_to(User))]
#[diesel(table_name = reservations)]
pub struct Reservation {
    pub id: i32,
    pub start_time: chrono::NaiveDateTime,
    pub end_time: chrono::NaiveDateTime,
    pub alerted: bool,
    pub user_id: i64,
}

#[derive(Insertable)]
#[diesel(table_name = binance_accounts)]
pub struct NewBinanceAccount {
    pub name: String,
    pub is_paper: bool,
    pub api_key: String,
    pub secret: String,
}

#[derive(Identifiable, Clone, Queryable, PartialEq, Selectable, Debug)]
#[diesel(table_name = binance_accounts)]
pub struct BinanceAccount {
    pub id: i32,
    pub name: String,
    pub selected: bool,
    pub is_paper: bool,
    pub api_key: String,
    pub secret: String,
    pub active_clock_stub: Option<i32>,
    pub active_reservation: Option<i32>,
    pub active_transaction: Option<i32>

}



#[derive(Insertable)]
#[diesel(table_name = clock_stubs)]
pub struct NewClockStub {
    pub start_time: chrono::NaiveDateTime,
    pub user_id: i64,
    pub last_interaction: chrono::NaiveDateTime,
}


#[derive(Identifiable, Clone, Queryable, PartialEq, Selectable, Debug)]
#[diesel(table_name = clock_stubs)]
pub struct ClockStub {
    pub id: i32,
    pub start_time: chrono::NaiveDateTime,
    pub end_time: Option<chrono::NaiveDateTime>,
    pub user_id: i64,
    pub last_interaction: chrono::NaiveDateTime,
}

#[allow(non_snake_case)]
#[derive(Insertable)]
#[diesel(table_name = transactions)]
pub struct NewTransaction {
    pub clock_stub_id: i32,
    pub buyOrderIds: String,
    pub sellOrderIds: String,

}

#[allow(non_snake_case)]
#[derive(Identifiable, Clone, Queryable, PartialEq, Selectable, Debug)]
#[diesel(table_name = transactions)]
pub struct DBTransaction {
    pub id:i32,
    pub clock_stub_id: i32,
    pub buyOrderTime: chrono::NaiveDateTime,
    pub buyOrderIds: String,
    pub buyReady: bool,
    pub buyAvgPrice:  Option<f64>,
    pub sellOrderIds: String,
    pub sellReady: bool,
    pub sellAvgPrice:  Option<f64>,
}
