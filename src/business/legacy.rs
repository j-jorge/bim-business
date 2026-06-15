// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

pub enum TransferResult {
  Disabled,
  Done,
  AlreadyDone,
}

#[derive(serde::Deserialize)]
pub struct GameStatistics {
  game_count: i32,
  victory_count: i32,
  defeat_count: i32,
}

pub async fn transfer(
  db: &db::Transaction<'_>,
  user_id: i64,
  coins: i64,
  game_features: &[String],
  slots: &[i16],
  game_feature_selection: &[inventory::GameFeatureSlotSelection],
  arena_stats: &GameStatistics,
) -> result::Result<TransferResult> {
  if !app_config::get(db, "legacy.enable_transfer", false).await {
    return Ok(TransferResult::Disabled);
  }

  // Enable the slots.
  {
    let mut query: String = String::from(
      r"insert into user_available_game_feature_slots
      values",
    );
    let mut parameters =
      Vec::<&(dyn tokio_postgres::types::ToSql + Sync)>::with_capacity(
        1 + slots.len(),
      );
    parameters.push(&user_id);

    let mut separator = ' ';

    for (i, slot_index) in slots.iter().enumerate() {
      query += &format!(r"{}($1, ${})", separator, i + 2);
      parameters.push(slot_index);
      separator = ',';
    }

    query += r" on conflict do nothing";
    db::execute_p(db, &query, &parameters[..]).await?;
  }

  // Enable the game features.
  {
    let mut query = String::from(
      r"insert into user_available_game_features
      select $1, game_feature.id
      from (values ",
    );
    let mut parameters =
      Vec::<&(dyn tokio_postgres::types::ToSql + Sync)>::with_capacity(
        1 + game_features.len(),
      );
    parameters.push(&user_id);

    let mut separator = ' ';

    for (i, name) in game_features.iter().enumerate() {
      query += &format!(r"{}(${})", separator, i + 2);
      parameters.push(name);
      separator = ',';
    }

    query += r") as u(feature_name)
                 left join game_feature
                 on u.feature_name = game_feature.name
                 on conflict do nothing";
    db::execute_p(db, &query, &parameters[..]).await?;
  }

  inventory::user_select_game_features(db, user_id, game_feature_selection)
    .await?;

  db::execute_p(
    db,
    r"insert into user_arena_statistics
      values ($1, $2, $3, $4)
      on conflict do nothing",
    &[
      &user_id,
      &arena_stats.game_count,
      &arena_stats.victory_count,
      &arena_stats.defeat_count,
    ],
  )
  .await?;

  let r: result::Result<()> =
    wallet::coins_transaction(db, user_id, "legacy", coins).await;

  if let Err(error) = r {
    if error == error::Error::UniqueViolation {
      return Ok(TransferResult::AlreadyDone);
    }

    return Err(error);
  }

  return Ok(TransferResult::Done);
}
