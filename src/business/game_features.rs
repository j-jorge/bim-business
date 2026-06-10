// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

// A game feature is just an ID and an associated cost in coins.

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Feature {
  pub id: String,
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

  /// Adds game features with the given cost in coins, or update the
  /// price of a game feature if the ID already exists.
  pub async fn batch_put(&self, features: &Vec<Feature>) -> result::Result<()> {
    if features.is_empty() {
      return Ok(());
    }

    let mut client: deadpool_postgres::Object = self.m_db.client().await?;
    let transaction: deadpool_postgres::Transaction<'_> =
      db::transaction(&mut client).await?;

    for f in features {
      if f.coins < 0 {
        tracing::error!("Feature cost '{}' cannot be negative", &f.id);
        return Err(error::Error::BadParameter);
      }

      transaction
        .execute(
          "insert into game_feature \
           values ($1, $2) \
           on conflict (id) \
           do update set cost_in_coins = $2",
          &[&f.id, &f.coins],
        )
        .await?;
    }

    return Ok(transaction.commit().await?);
  }

  pub async fn list(&self) -> result::Result<Vec<Feature>> {
    return self
      .m_db
      .collect("select id, cost_in_coins from game_feature", |row| {
        Feature {
          id: row.get(0),
          coins: row.get(1),
        }
      })
      .await;
  }
}
