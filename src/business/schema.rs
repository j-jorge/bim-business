// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

/// Update the tables to match the state required by the current code.
pub async fn migrate_database(
  client: &mut db::Client,
  assets: &std::path::Path
) -> result::Result<()>
{
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
  let table_version: i32 = match version_row
  {
    None => 0,
    Some(r) => r.get(0)
  };

  let mut final_version: i32 = table_version;

  loop
  {
    let sql_file: std::path::PathBuf =
      assets.join(format!("db/{}.sql", final_version + 1));

    if !std::fs::exists(&sql_file)?
    {
      break;
    }

    final_version += 1;
    tracing::info!("Upgrading tables to {final_version}.");

    t.batch_execute(&std::fs::read_to_string(sql_file)?).await?;
  }

  if final_version != table_version
  {
    t.execute(r"update meta_version set value = $1", &[&final_version])
      .await?;
    t.commit().await?;
  }

  tracing::info!("Migration done. Final version is {}.", final_version);

  return Ok(());
}
