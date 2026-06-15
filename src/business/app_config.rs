// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Entry {
  pub key: String,
  pub value: String,
}

pub async fn batch_put(
  t: &db::Transaction<'_>,
  values: &[Entry],
) -> result::Result<()> {
  let mut query = String::from("insert into app_config values");
  let mut s = ' ';
  let mut parameters =
    Vec::<&(dyn tokio_postgres::types::ToSql + Sync)>::with_capacity(
      values.len(),
    );

  for (i, entry) in values.iter().enumerate() {
    query += &format!(r"{}(${}, ${})", s, 2 * i + 1, 2 * i + 2);
    s = ',';

    parameters.push(&entry.key);
    parameters.push(&entry.value);
  }

  query += " on conflict (key) do update set value = excluded.value";

  db::execute_p(t, &query, &parameters[..]).await?;

  return Ok(());
}

pub async fn batch_erase(
  t: &db::Transaction<'_>,
  keys: &[String],
) -> result::Result<()> {
  for key in keys {
    db::execute_p(t, r"delete from app_config where key = $1", &[&key]).await?;
  }

  return Ok(());
}

pub async fn get<T>(
  db: &impl deadpool_postgres::GenericClient,
  key: &str,
  default: T,
) -> T
where
  T: for<'a> tokio_postgres::types::FromSql<'a> + std::str::FromStr,
  <T as std::str::FromStr>::Err: std::fmt::Display,
{
  let value_opt: Option<String> = get_value(db, key).await;

  if let Some(value_str) = value_opt {
    match value_str.parse::<T>() {
      Ok(value) => {
        return value;
      }
      Err(e) => {
        tracing::error!(
          "can't convert config '{}'={} to requested type: {}",
          key,
          value_str,
          e
        );
      }
    }
  }

  return default;
}

pub async fn get_u64(
  db: &impl deadpool_postgres::GenericClient,
  key: &str,
  default: u64,
) -> u64 {
  let value_opt: Option<String> = get_value(db, key).await;

  if let Some(value_str) = value_opt {
    match value_str.parse::<u64>() {
      Ok(value) => {
        return value;
      }
      Err(e) => {
        tracing::error!(
          "can't convert config '{}'={} to requested type: {}",
          key,
          value_str,
          e
        );
      }
    }
  }

  return default;
}

async fn get_value(
  db: &impl deadpool_postgres::GenericClient,
  key: &str,
) -> Option<String> {
  let query_result: result::Result<Option<tokio_postgres::Row>> =
    db::query_opt_p(
      db,
      r"select value from app_config where key = $1",
      &[&key],
    )
    .await;

  match query_result {
    Err(e) => {
      tracing::error!("failed to fetch config '{}': {}", key, e);
      return None;
    }
    Ok(Some(row)) => {
      return Some(row.get(0));
    }
    _ => {
      return None;
    }
  }
}
