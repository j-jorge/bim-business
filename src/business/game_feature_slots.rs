// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Slot {
  pub index: i16,
  pub coins: i32,
}

pub async fn batch_put(
  t: &db::Transaction<'_>,
  slots: &Vec<Slot>,
) -> result::Result<()> {
  if slots.is_empty() {
    return Ok(());
  }

  for slot in slots {
    if slot.index < 0 {
      tracing::error!(
        "Feature slot index '{}' cannot be negative",
        &slot.index
      );
      return Err(error::Error::BadParameter);
    }

    if slot.coins < 0 {
      tracing::error!("Feature slot cost '{}' cannot be negative", slot.index);
      return Err(error::Error::BadParameter);
    }

    db::execute_p(
      t,
      "insert into game_feature_slot \
                 values ($1, $2) \
                 on conflict (index) \
                 do update set cost_in_coins = $2",
      &[&slot.index, &slot.coins],
    )
    .await?;
  }

  return Ok(());
}

pub async fn list(db: &db::Client) -> result::Result<Vec<Slot>> {
  return db::collect(
    db,
    "select index, cost_in_coins from game_feature_slot",
    |row| Slot {
      index: row.get(0),
      coins: row.get(1),
    },
  )
  .await;
}
