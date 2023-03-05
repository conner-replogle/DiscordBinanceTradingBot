-- Your SQL goes here
CREATE TABLE binance_accounts (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  name VARCHAR NOT NULL UNIQUE,
  selected tinyint NOT NULL default 0,
  is_paper tinyint NOT NULL default 0,
  api_key VARCHAR NOT NULL UNIQUE,
  secret VARCHAR NOT NULL UNIQUE,
  active_clock_stub INTEGER REFERENCES clock_stubs (id),
  active_reservation INTEGER REFERENCES reservations (id)
);
