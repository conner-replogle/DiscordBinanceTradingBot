-- Your SQL goes here
PRAGMA foreign_keys = ON;
CREATE TABLE reservations (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  start_time  NOT NULL,
  end_time DATETIME NOT NULL,
  alerted tinyint NOT NULL default 0,
  user_id bigint NOT NULL REFERENCES users (id)
);
