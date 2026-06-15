// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

// Key-value storage for the client's config parameters that would not
// need special handling.

pub enum Value {
  Int64(i64),
  Text(String),
}

pub struct Entry {
  pub key: String,
  pub value: Value,
}

// Code for the type of the value as stored in the data base.
enum DbValueType {
  Int64 = 0,
  Text = 1,
}

pub async fn batch_put(
  t: &db::Transaction<'_>,
  entries: &[Entry],
) -> result::Result<()> {
  for entry in entries {
    match &entry.value {
      Value::Int64(i) => {
        internal_put(t, &entry.key, DbValueType::Int64, *i, "").await?;
      }
      Value::Text(s) => {
        internal_put(t, &entry.key, DbValueType::Text, 0, s).await?;
      }
    };
  }

  return Ok(());
}

async fn internal_put(
  transaction: &db::Transaction<'_>,
  key: &str,
  t: DbValueType,
  int64_value: i64,
  text_value: &str,
) -> result::Result<()> {
  db::execute_p(
    transaction,
    "insert into flat_client_config \
           values ($1, $2, $3, $4) \
           on conflict (key) \
           do update set \
              type = $2, \
              int64_value = $3, \
              text_value = $4
           ",
    &[&key, &(t as i16), &int64_value, &text_value],
  )
  .await?;

  return Ok(());
}

pub async fn batch_erase(
  t: &db::Transaction<'_>,
  keys: &[String],
) -> result::Result<()> {
  for key in keys {
    db::execute_p(
      t,
      "delete from flat_client_config \
               where key = $1",
      &[&key],
    )
    .await?;
  }

  return Ok(());
}

pub async fn all_entries(db: &db::Client) -> result::Result<Vec<Entry>> {
  fn row_to_entry(r: tokio_postgres::Row) -> result::Result<Entry> {
    const INT64_TYPE: i16 = DbValueType::Int64 as i16;
    const TEXT_TYPE: i16 = DbValueType::Text as i16;

    match r.get(1) {
      INT64_TYPE => {
        return Ok(Entry {
          key: r.get(0),
          value: Value::Int64(r.get(2)),
        });
      }
      TEXT_TYPE => {
        return Ok(Entry {
          key: r.get(0),
          value: Value::Text(r.get(3)),
        });
      }
      _ => {
        return Err(error::Error::BadParameter);
      }
    }
  }

  return db::collect(
    db,
    "select key, type, int64_value, text_value from flat_client_config",
    row_to_entry,
  )
  .await?;
}
