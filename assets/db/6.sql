truncate table meta_version;
insert into meta_version values (0, '2026-09-20 00:00:01');

-- Convert timestamp columns to timestamptz. Everything is officially UTC.
alter table meta_version
alter column date type timestamptz
using date at time zone 'UTC';

alter table currency_transaction
alter column date type timestamptz
using date at time zone 'UTC';

alter table sessions
alter column created_at type timestamptz
using created_at at time zone 'UTC';

alter table sessions
alter column expires_at type timestamptz
using expires_at at time zone 'UTC';

alter table sessions
alter column last_used_at type timestamptz
using last_used_at at time zone 'UTC';

alter table active_game
alter column start_date type timestamptz
using start_date at time zone 'UTC';

alter table done_game
alter column start_date type timestamptz
using start_date at time zone 'UTC';

alter table done_game
alter column end_date type timestamptz
using end_date at time zone 'UTC';

alter table game_server
alter column registration_date type timestamptz
using registration_date at time zone 'UTC';

alter table game_server
alter column last_seen type timestamptz
using last_seen at time zone 'UTC';

alter table user_account
alter column last_nickname_change type timestamptz
using last_nickname_change at time zone 'UTC';
