-- Your SQL goes here

CREATE TABLE clock_stubs (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  start_time DATETIME NOT NULL,
  end_time DATETIME,
  user_id bigint NOT NULL REFERENCES users (id),
  last_interaction DATETIME NOT NULL
);
