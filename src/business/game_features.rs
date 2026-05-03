// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

// A game feature is just an ID and an associated cost in coins.

pub async fn run_migration(
  transaction: &deadpool_postgres::Transaction<'_>,
  to_version: i32,
) -> result::Result<()> {
  if to_version == 1 {
    transaction
      .batch_execute(
        "create table game_features \
         (id text primary key, cost_in_coins integer)",
      )
      .await?;
  }

  return Ok(());
}

pub struct GameFeatures {
  m_db: deadpool_postgres::Pool,
}

impl GameFeatures {
  pub fn new(db: deadpool_postgres::Pool) -> GameFeatures {
    let result = GameFeatures { m_db: db };

    return result;
  }

  /// Adds game features with the given cost in coins, or update the
  /// price of a game feature if the ID already exists.
  pub async fn batch_put(
    &self,
    features: &std::collections::HashMap<String, i32>,
  ) -> result::Result<()> {
    if features.is_empty() {
      return Ok(());
    }

    let mut client: deadpool_postgres::Object = self.m_db.get().await?;
    let transaction: deadpool_postgres::Transaction<'_> =
      client.transaction().await?;

    for (id, coins) in features {
      if *coins < 0 {
        tracing::error!("Feature cost '{}' cannot be negative", &id);
        return Err(error::Error::InvalidParameter);
      }

      transaction
        .execute(
          "insert into game_features \
           values ($1, $2) \
           on conflict (id) \
           do update set cost_in_coins = $2",
          &[&id, &coins],
        )
        .await?;
    }

    return Ok(transaction.commit().await?);
  }

  /// Returns a map of game feature IDs as keys and their cost as coins as
  /// values.
  pub async fn list(
    &self,
  ) -> result::Result<std::collections::HashMap<String, i32>> {
    return Ok(
      self
        .m_db
        .get()
        .await?
        .query("select id, cost_in_coins from game_features", &[])
        .await?
        .into_iter()
        .map(|row| (row.get(0), row.get(1)))
        .collect(),
    );
  }
}
