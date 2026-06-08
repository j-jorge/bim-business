// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

pub async fn run_migration(
  transaction: &deadpool_postgres::Transaction<'_>,
  to_version: i32,
) -> result::Result<()> {
  if to_version == 2 {
    transaction
      .batch_execute(
        "create table game_feature_slot \
         (index integer primary key, cost_in_coins integer)",
      )
      .await?;
  }

  return Ok(());
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Slot {
  pub index: i32,
  pub coins: i32,
}

pub struct Repository {
  m_db: db::Wrapper,
}

impl Repository {
  pub fn new(db: deadpool_postgres::Pool) -> Repository {
    return Repository {
      m_db: db::Wrapper::new(db),
    };
  }

  pub async fn batch_put(&self, slots: &Vec<Slot>) -> result::Result<()> {
    if slots.is_empty() {
      return Ok(());
    }

    let mut client: deadpool_postgres::Object = self.m_db.client().await?;
    let transaction: deadpool_postgres::Transaction<'_> =
      db::transaction(&mut client).await?;

    for slot in slots {
      if slot.index < 0 {
        tracing::error!(
          "Feature slot index '{}' cannot be negative",
          &slot.index
        );
        return Err(error::Error::BadParameter);
      }

      if slot.coins < 0 {
        tracing::error!(
          "Feature slot cost '{}' cannot be negative",
          slot.index
        );
        return Err(error::Error::BadParameter);
      }

      transaction
        .execute(
          "insert into game_feature_slot \
           values ($1, $2) \
           on conflict (index) \
           do update set cost_in_coins = $2",
          &[&slot.index, &slot.coins],
        )
        .await?;
    }

    return Ok(transaction.commit().await?);
  }

  pub async fn list(&self) -> result::Result<Vec<Slot>> {
    return self
      .m_db
      .collect(
        "select index, cost_in_coins from game_feature_slot",
        |row| Slot {
          index: row.get(0),
          coins: row.get(1),
        },
      )
      .await;
  }
}
