-- Your SQL goes here

CREATE TABLE configs (
  section varchar(64) NOT NULL,
  key varchar(64) PRIMARY KEY NOT NULL,
  value_type int NOT NULL,
  description varchar(1000) NOT NULL,
  value varchar(255)
);