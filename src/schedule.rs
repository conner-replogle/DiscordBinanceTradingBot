use std::sync::Arc;
use std::time::{self};

use crate::config::Config;
use crate::error::TradingBotError;
use crate::models::NewReservation;
use crate::schema::reservations;
use crate::{db::establish_connection, models::Reservation};
use chrono::{Date, DateTime, Duration, DurationRound, NaiveDateTime, Utc};
use diesel::ExpressionMethods;
use diesel::{QueryDsl, RunQueryDsl, SqliteConnection};
use tracing::log::warn;

pub enum TimeSlot {
    RESERVED { reservation: Reservation },
    OPEN(DateTime<Utc>),
}

pub struct Schedule {
    reservations: Vec<Reservation>,
}

impl Schedule {
    pub fn intitalize() -> Result<Self, diesel::result::Error> {
        Ok(Self {
            reservations: Self::pull_reservations()?,
        })
    }

    pub fn open_time_slots(
        set_start_time: Option<DateTime<Utc>>,
        config: &Arc<Config>,
    ) -> Result<Vec<TimeSlot>, TradingBotError> {
        let interval = match config.get("schedule", "reservation_interval_min")? {
            Some(num) => num,
            None => 15,
        };
        let reservations = Self::pull_reservations()?;
        let mut right_now = set_start_time.unwrap_or(
            Utc::now()
                .duration_trunc(Duration::minutes(interval as i64))
                .unwrap(),
        );
        let max_reservation = right_now + Duration::days(2);
        let mut possible_times = Vec::new();
        while right_now < max_reservation {
            right_now = right_now + Duration::minutes(interval as i64);
            let mut slot_reserved = None;

            for reservation in reservations.iter() {
                if set_start_time.is_some() {
                    if reservation.start_time == right_now.naive_utc() {
                        possible_times.push(TimeSlot::OPEN(right_now));
                        return Ok(possible_times);
                    }
                } else {
                    if reservation.start_time <= right_now.naive_utc()
                        && right_now.naive_utc() < reservation.end_time
                    {
                        slot_reserved = Some(reservation);
                        break;
                    }
                }
            }
            if let Some(reserved) = slot_reserved {
                possible_times.push(TimeSlot::RESERVED {
                    reservation: reserved.clone(),
                });
            } else {
                possible_times.push(TimeSlot::OPEN(right_now));
            }
        }

        Ok(possible_times)
    }

    pub fn create_reservation(
        new_reservation: NewReservation,
    ) -> Result<bool, diesel::result::Error> {
        let mut connection: SqliteConnection = establish_connection();

        use crate::schema::reservations::dsl;
        let reservations = Self::pull_reservations()?;

        //       |-------|
        //  |----|       |-----||---------|

        for reservation in reservations.iter() {
            if reservation.start_time < new_reservation.end_time
                && reservation.end_time > new_reservation.start_time
            {
                return Ok(false);
            }
        }
        diesel::insert_into(dsl::reservations)
            .values(new_reservation)
            .execute(&mut connection)?;
        Ok(true)
    }

    pub fn pull_reservations() -> Result<Vec<Reservation>, diesel::result::Error> {
        let mut connection: SqliteConnection = establish_connection();
        use crate::schema::reservations::dsl::*;
        let pulled_reser = reservations
            .order(start_time.asc())
            .load::<Reservation>(&mut connection)?;
        return Ok(pulled_reser);
    }
}
