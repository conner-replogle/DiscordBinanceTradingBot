-- Your SQL goes here

CREATE TABLE clock_stubs (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  start_time TEXT NOT NULL,
  end_time TEXT,
  user_id bigint NOT NULL REFERENCES users (id),
  last_interaction TEXT NOT NULL,
);
