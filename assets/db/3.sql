create table bot
(
  user_id bigint primary key references user_account (user_id)
);

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
  game_server_id bigint references game_server (id)
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
  game_id bigint references active_game (game_id),
  user_id bigint primary key references user_account (user_id),
  unique (game_id, user_id)
);

-- Which players were in each completed game.
create table done_game_player
(
  game_id bigint references done_game (game_id),
  user_id bigint references user_account (user_id),
  unique (game_id, user_id)
);

-- Reward of each player in the given game. This is kept for the
-- client to retrieve its reward after a game for display.
create table game_reward
(
  game_id bigint references game (game_id),
  user_id bigint primary key references user_account (user_id),
  coins bigint not null
);
