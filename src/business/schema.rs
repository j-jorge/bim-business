// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

/// Update the tables to match the state required by the current code.
pub async fn migrate_database(
  client: &mut db::Client,
  assets: &std::path::Path,
) -> result::Result<()> {
  // Wrap the operations in a transaction such that we can apply
  // them all at once, thus avoiding a partial modification if
  // something fails.
  let t: db::Transaction<'_> = client.transaction().await?;

  // We are keeping the current version of the schema into a
  // specific table which will have a single row (or none on
  // creation) with the version number.
  t.batch_execute("create table if not exists meta_version (value integer)")
    .await?;

  let version_row: Option<tokio_postgres::Row> = t
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

  if table_version == 0 {
    tracing::info!("Upgrading tables to 1.");

    t.batch_execute(&std::fs::read_to_string(assets.join("db/1.sql"))?)
      .await?;
  }

  if table_version <= 1 {
    tracing::info!("Upgrading tables to 2.");

    t.batch_execute(&std::fs::read_to_string(assets.join("db/2.sql"))?)
      .await?;
  }

  // Update the schema version too, in the same transaction.
  t.batch_execute(r"truncate table meta_version;").await?;
  t.execute(
    r"
insert into meta_version (value, date) values ($1, '2026-06-15 00:00:00');
",
    &[&CURRENT_VERSION],
  )
  .await?;

  t.commit().await?;

  tracing::info!("Migration done. Final version is {}.", CURRENT_VERSION);

  return Ok(());
}
