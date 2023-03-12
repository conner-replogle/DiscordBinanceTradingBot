-- Your SQL goes here
PRAGMA foreign_keys = ON;
CREATE TABLE reservations (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  start_time  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M','utc')),
  end_time TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M','utc')),
  alerted tinyint NOT NULL default 0,
  user_id bigint NOT NULL REFERENCES users (id)
);
