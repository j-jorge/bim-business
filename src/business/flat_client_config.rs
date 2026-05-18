// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

// Key-value storage for the client's config parameters that would not
// need special handling.

pub async fn run_migration(
  transaction: &deadpool_postgres::Transaction<'_>,
  to_version: i32,
) -> result::Result<()> {
  if to_version == 1 {
    transaction
      .batch_execute(
        "create table flat_client_config \
           (key text primary key, \
           type smallint, \
           int64_value bigint, \
           text_value text)",
      )
      .await?;
  }

  return Ok(());
}

pub struct FlatClientConfig {
  m_db: db::Wrapper,
}

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

impl FlatClientConfig {
  pub fn new(db: deadpool_postgres::Pool) -> FlatClientConfig {
    return FlatClientConfig {
      m_db: db::Wrapper::new(db),
    };
  }

  pub async fn batch_put(&self, entries: &[Entry]) -> result::Result<()> {
    let mut client: deadpool_postgres::Object = self.m_db.client().await?;
    let t: deadpool_postgres::Transaction<'_> =
      db::transaction(&mut client).await?;

    for entry in entries {
      match &entry.value {
        Value::Int64(i) => {
          Self::internal_put(&t, &entry.key, DbValueType::Int64, *i, "")
            .await?;
        }
        Value::Text(s) => {
          Self::internal_put(&t, &entry.key, DbValueType::Text, 0, s).await?;
        }
      };
    }

    t.commit().await?;

    return Ok(());
  }

  async fn internal_put(
    transaction: &deadpool_postgres::Transaction<'_>,
    key: &str,
    t: DbValueType,
    int64_value: i64,
    text_value: &str,
  ) -> result::Result<()> {
    transaction
      .execute(
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

  pub async fn batch_erase(&self, keys: &[String]) -> result::Result<()> {
    let mut client: deadpool_postgres::Object = self.m_db.client().await?;
    let transaction: deadpool_postgres::Transaction<'_> =
      db::transaction(&mut client).await?;

    for key in keys {
      transaction
        .execute(
          "delete from flat_client_config \
               where key = $1",
          &[&key],
        )
        .await?;
    }

    return Ok(transaction.commit().await?);
  }

  pub async fn all_entries(&self) -> result::Result<Vec<Entry>> {
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

    return self
      .m_db
      .collect(
        "select key, type, int64_value, text_value from flat_client_config",
        row_to_entry,
      )
      .await?;
  }
}
