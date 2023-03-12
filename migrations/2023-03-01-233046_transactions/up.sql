-- Your SQL goes here
CREATE TABLE transactions (
  id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
  clock_stub_id INTEGER NOT NULL REFERENCES clock_stubs (id),

  buyOrderTime TEXT NOT NULL ,

  buyOrderIds varchar NOT NULL,
  buyReady tinyint NOT NULL DEFAULT 0,
  buyAvgPrice DECIMAL,

  sellOrderIds varchar NOT NULL,
  sellReady tinyint NOT NULL DEFAULT 0,
  sellAvgPrice DECIMAL



);
