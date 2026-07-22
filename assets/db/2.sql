create table game_feature_slot
(
  index smallint primary key,
  cost_in_coins integer not null
);

alter table meta_version add date timestamp not null;

create table app_config
(
  key text primary key,
  value text not null
);

create table user_account
(
  user_id bigint primary key generated always as identity,
  nickname text not null
);

create table nickname_override
(
  user_id bigint primary key references user_account (user_id),
  nickname text not null
);

create table user_device
(
  device_id text primary key,
  user_id bigint references user_account (user_id)
);

create table user_wallet
(
  user_id bigint primary key references user_account (user_id),
  coins bigint not null
);

create type transaction_origin as enum ('admin', 'app');

create table currency_transaction
(
  user_id bigint references user_account (user_id),
  date timestamp not null,
  origin transaction_origin not null,
  reason text not null,
  initial_balance bigint not null,
  amount bigint not null
);

create unique index currency_transaction_unique_reasons
on currency_transaction (user_id, reason)
where reason = 'legacy';

create table user_available_game_feature_slots
(
  user_id bigint references user_account (user_id),
  slot_index smallint,
  primary key (user_id, slot_index)
);

-- Remove the text-based primary key id from game_feature. Use a
-- numeric id instead, and keep the text info for the clients.
alter table game_feature
drop constraint game_feature_pkey;

alter table game_feature
rename column id to name;

alter table game_feature
add constraint unique_name unique (name);

alter table game_feature
add column id smallint primary key generated always as identity;

create table user_available_game_features
(
  user_id bigint references user_account (user_id),
  feature_id smallint references game_feature (id),
  primary key (user_id, feature_id)
);

create table user_selected_game_features
(
  user_id bigint references user_account (user_id),
  slot_index smallint not null,
  feature_id smallint references game_feature (id),

  unique (user_id, slot_index),

  -- The slot must be available for this user
  constraint user_owns_the_slot
  foreign key (user_id, slot_index)
  references user_available_game_feature_slots,

  -- The game feature must be available for this user.
  constraint user_owns_the_feature
  foreign key (user_id, feature_id)
  references user_available_game_features
);

create table user_arena_statistics
(
  user_id bigint primary key references user_account (user_id),
  game_count integer not null,
  victories integer not null,
  defeats integer not null
);

create table sessions
(
  token text primary key,
  user_id bigint references user_account (user_id),
  device_id text references user_device (device_id),
  created_at timestamp not null,
  expires_at timestamp not null,
  last_used_at timestamp not null
);
