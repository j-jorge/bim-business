// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Feature {
  pub name: String,
  pub coins: i32,
}

/// Adds game features with the given cost in coins, or update the
/// price of a game feature if the name already exists.
pub async fn batch_put(
  t: &db::Transaction<'_>,
  features: &Vec<Feature>,
) -> result::Result<()> {
  if features.is_empty() {
    return Ok(());
  }

  for f in features {
    if f.coins < 0 {
      tracing::error!("Feature cost '{}' cannot be negative", &f.name);
      return Err(error::Error::BadParameter);
    }

    db::execute_p(
      t,
      "insert into game_feature \
           values ($1, $2, default) \
           on conflict (name) \
           do update set cost_in_coins = $2",
      &[&f.name, &f.coins],
    )
    .await?;
  }

  return Ok(());
}

pub async fn list(db: &db::Client) -> result::Result<Vec<Feature>> {
  return db::collect(
    db,
    "select name, cost_in_coins from game_feature",
    |row| Feature {
      name: row.get(0),
      coins: row.get(1),
    },
  )
  .await;
}
