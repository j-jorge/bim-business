create table flat_client_config
(
  key text primary key,
  type smallint,
  int64_value bigint,
  text_value text
);

create table leads (token text unique);

create table game_feature
(
  id text primary key,
  cost_in_coins integer
);

create table game_server
(
  id text primary key,
  token text unique,
  description text,
  registration_date timestamp,
  last_seen timestamp
);

create table shop (id text primary key, coins integer);
