-- Your SQL goes here

CREATE TABLE clock_stubs (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  start_time TIMESTAMP NOT NULL,
  end_time TIMESTAMP,
  user_id bigint NOT NULL REFERENCES users (id),
  last_interaction TIMESTAMP NOT NULL,
  active_transaction INTEGER REFERENCES transactions (id)
);
