-- Your SQL goes here
PRAGMA foreign_keys = ON;
CREATE TABLE reservations (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  start_time DATETIME NOT NULL,
  end_time DATETIME NOT NULL,
  user_id bigint NOT NULL REFERENCES users (id)
);
