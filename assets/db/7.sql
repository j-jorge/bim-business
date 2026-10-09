truncate table meta_version;
insert into meta_version values (0, '2026-10-08 00:00:00');

-- Players are now allowed to be in multiple games simultaneously.
drop index user_is_in_single_game;

alter table game_reward
drop constraint game_reward_pkey;

-- In a given game, a player must appear only once, unless it's a bot.
create unique index active_game_user_uniqueness
on active_game_player (game_id, user_id)
where user_id <> 0;

create unique index game_reward_game_user_uniqueness
on game_reward (game_id, user_id);
