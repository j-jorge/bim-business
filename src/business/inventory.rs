// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

pub async fn user_buy_game_feature_slot(
  db: &db::Transaction<'_>,
  user_id: i64,
  slot_index: i16,
) -> result::Result<()> {
  let row_opt: Option<tokio_postgres::Row> = db::query_opt_p(
    db,
    r"select cost_in_coins
      from game_feature_slot
      where index = $1",
    &[&slot_index],
  )
  .await?;

  if let Some(row) = row_opt {
    let cost: i32 = row.get(0);

    wallet::coins_transaction(
      db,
      user_id,
      "game-feature-slot-purchase",
      -i64::from(cost),
    )
    .await?;

    db::execute_p(
      db,
      r"insert into user_available_game_feature_slots
      values ($1, $2)",
      &[&user_id, &slot_index],
    )
    .await?;

    return Ok(());
  }

  return Err(error::Error::Unprocessable);
}

#[derive(serde::Serialize)]
pub struct GameFeatureSlotState {
  pub slot_index: i16,
  pub feature: Option<String>,
}

pub async fn user_selected_game_features(
  db: &db::Client,
  user_id: i64,
) -> result::Result<Vec<GameFeatureSlotState>> {
  return db::collect_p(
    db,
    r"SELECT available.slot_index, feature.name
      FROM user_available_game_feature_slots AS available
      LEFT JOIN user_selected_game_features AS selected
      ON available.slot_index = selected.slot_index
      LEFT JOIN game_feature as feature
      ON selected.feature_id = feature.id
      WHERE available.user_id = $1",
    &[&user_id],
    |row| GameFeatureSlotState {
      slot_index: row.get(0),
      feature: row.get(1),
    },
  )
  .await;
}

#[derive(serde::Deserialize)]
pub struct GameFeatureSlotSelection {
  pub slot_index: i16,
  pub feature: String,
}

pub async fn user_select_game_features(
  db: &db::Transaction<'_>,
  user_id: i64,
  selection: &[GameFeatureSlotSelection],
) -> result::Result<()> {
  let mut query: String = String::from(
    r"insert into user_selected_game_features
      select $1, selection.slot_index, game_feature.id
      from (values",
  );
  let mut parameters =
    Vec::<&(dyn tokio_postgres::types::ToSql + Sync)>::with_capacity(
      1 + selection.len(),
    );
  parameters.push(&user_id);

  let mut separator = ' ';

  for (i, s) in selection.iter().enumerate() {
    query +=
      &format!(r"{}(${}::smallint, ${})", separator, 2 * i + 2, 2 * i + 3);
    parameters.push(&s.slot_index);
    parameters.push(&s.feature);
    separator = ',';
  }

  query += r") as selection(slot_index, feature_name)
               left join game_feature
               on selection.feature_name = game_feature.name
               on conflict (user_id, slot_index)
               do update set feature_id = excluded.feature_id";

  let inserted: u64 = db::execute_p(db, &query, &parameters[..]).await?;

  if inserted as usize != selection.len() {
    return Err(error::Error::Unprocessable);
  }

  return Ok(());
}

pub async fn user_clear_game_feature_slot(
  db: &db::Transaction<'_>,
  user_id: i64,
  slot_index: i16,
) -> result::Result<()> {
  db::execute_p(
    db,
    r"delete from user_selected_game_features
      where user_id = $1
      and slot_index = $2",
    &[&user_id, &slot_index],
  )
  .await?;

  return Ok(());
}

pub async fn user_buy_game_feature(
  db: &db::Transaction<'_>,
  user_id: i64,
  name: &str,
) -> result::Result<()> {
  let row_opt: Option<tokio_postgres::Row> = db::query_opt_p(
    db,
    r"select cost_in_coins, id
      from game_feature
      where name = $1",
    &[&name],
  )
  .await?;

  if let Some(row) = row_opt {
    let cost: i32 = row.get(0);

    wallet::coins_transaction(
      db,
      user_id,
      "game-feature-purchase",
      -i64::from(cost),
    )
    .await?;

    let feature_id: i16 = row.get(1);

    db::execute_p(
      db,
      r"insert into user_available_game_features
      values ($1, $2)",
      &[&user_id, &feature_id],
    )
    .await?;

    return Ok(());
  }

  return Err(error::Error::Unprocessable);
}

pub async fn user_available_game_features(
  db: &db::Client,
  user_id: i64,
) -> result::Result<Vec<String>> {
  return db::collect_p(
    db,
    r"SELECT feature.name
      FROM user_available_game_features AS available
      INNER JOIN game_feature AS feature
      ON available.feature_id = feature.id
      WHERE available.user_id = $1",
    &[&user_id],
    |row| row.get(0),
  )
  .await;
}
