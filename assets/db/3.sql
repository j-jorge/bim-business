-- Let's have a fake user for the bots as it simplifies foreign keys
-- in game tables.
insert into user_account
overriding system value
values (0, 'bot-placeholder');

-- Remove the text-based primary key id from game_server. Use a
-- numeric id instead, and keep the text info for documentation.
alter table game_server
drop constraint game_server_pkey;

alter table game_server
rename column id to name;

alter table game_server
add column id bigint primary key generated always as identity;

-- All games, since the dawn of time, and the server on which they
-- were played.
create table game
(
  game_id bigint primary key generated always as identity,
  game_server_id bigint not null references game_server (id)
);

-- Games currently being played.
create table active_game
(
  game_id bigint primary key references game (game_id),
  start_date timestamp not null
);

-- Completed games.
create table done_game
(
  game_id bigint primary key references game (game_id),
  start_date timestamp not null,
  end_date timestamp not null,
  short_game boolean not null
);

-- Which players are in each active game. We keep it distinct from
-- done_game_player because it allows to have stronger constraints
-- that I won't have to check in the code.
create table active_game_player
(
  game_id bigint not null references active_game (game_id),
  user_id bigint not null references user_account (user_id)
);

-- Players can be in at most one active game, bots can be in many.
create unique index user_is_in_single_game
on active_game_player (user_id)
where user_id <> 0;

-- Which players were in each completed game.
create table done_game_player
(
  game_id bigint not null references done_game (game_id),
  user_id bigint not null references user_account (user_id),
  outcome text not null
);

-- Except for bots, players have played a game only once.
create unique index game_user_uniqueness
on done_game_player (game_id, user_id)
where user_id <> 0;

-- Reward of each player in the given game. This is kept for the
-- client to retrieve its reward after a game for display.
create table game_reward
(
  game_id bigint not null references game (game_id),
  user_id bigint primary key references user_account (user_id),
  coins bigint not null,
  -- No reward for the bots.
  check (user_id > 0)
);

-- Not related to games. Add not null constraints forgotten in the
-- initial tables.
alter table game_feature alter column cost_in_coins set not null;
alter table game_server alter column description set not null;
alter table game_server alter column registration_date set not null;
alter table game_server alter column last_seen set not null;
alter table shop alter column coins set not null;

drop table user_arena_statistics;

-- Aggregated stats per player. This should be equivalent to the
-- number of rows in done game player for each player and outcome.
create table arena_stats
(
  user_id bigint primary key references user_account (user_id),
  victories integer not null,
  defeats integer not null,
  draws integer not null,
  kicked integer not null
);
