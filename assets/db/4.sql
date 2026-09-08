truncate table meta_version;
insert into meta_version values (0, '2026-09-05 00:00:00');

-- Purchases done on the PlayStore.
create table play_store_purchase
(
  order_id text primary key,
  purchase_token text not null,
  user_id bigint references user_account (user_id)
);
