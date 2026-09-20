truncate table meta_version;
insert into meta_version values (0, '2026-09-20 00:00:00');

-- Remember when the user has changed its nickname.
alter table user_account add column last_nickname_change timestamp;
update user_account set last_nickname_change = 'epoch';
alter table user_account alter column last_nickname_change set not null;
