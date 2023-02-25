-- Your SQL goes here



CREATE TABLE users (
  id bigint UNIQUE PRIMARY KEY NOT NULL,
  tag varchar(255) UNIQUE NOT NULL
);