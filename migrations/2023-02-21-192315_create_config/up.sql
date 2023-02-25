-- Your SQL goes here

CREATE TABLE configs (
  section varchar(255) NOT NULL,
  key varchar(255) PRIMARY KEY NOT NULL,
  value_type int NOT NULL,
  value varchar(1000)
);