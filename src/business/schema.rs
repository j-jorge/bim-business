// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

/// Update the tables to match the state required by the current code.
pub async fn migrate_database(
  mut client: deadpool_postgres::Object,
) -> result::Result<()> {
  // We are keeping the current version of the schema into a
  // specific table which will have a single row (or none on
  // creation) with the version number.
  client
    .batch_execute("create table if not exists meta_version (value integer)")
    .await?;

  let version_row: Option<tokio_postgres::Row> = client
    .query_opt("select value from meta_version", &[])
    .await
    .unwrap();
  let table_version: i32 = match version_row {
    None => 0,
    Some(r) => r.get(0),
  };
  const CURRENT_VERSION: i32 = 2;

  if table_version == CURRENT_VERSION {
    return Ok(());
  }

  // Wrap the operations in a transaction such that we can apply
  // them all at once, thus avoiding a partial modification if
  // something fails.
  let t: deadpool_postgres::Transaction<'_> = client.transaction().await?;

  if table_version == 0 {
    println!("Upgrading tables to 1.");

    t.batch_execute(
      r"
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
",
    )
    .await?;
  }

  if table_version <= 1 {
    println!("Upgrading tables to 2.");

    t.batch_execute(
      r"
create table game_feature_slot
(
  index integer primary key,
  cost_in_coins integer
);
alter table meta_version add date timestamp;

",
    )
    .await?;
  }

  // Update the schema version too, in the same transaction.
  t.batch_execute(r"truncate table meta_version;").await?;
  t.execute(
    r"
insert into meta_version (value, date) values ($1, '2026-06-8 00:00:00');
",
    &[&CURRENT_VERSION],
  )
  .await?;

  t.commit().await?;

  tracing::info!("Migration done. Final version is {}.", CURRENT_VERSION);

  return Ok(());
}
