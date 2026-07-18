// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

pub async fn get_or_create_bots(
  transaction: &db::Transaction<'_>,
  count: usize,
) -> result::Result<Vec<i64>> {
  let mut result = Vec::<i64>::with_capacity(count);

  for row in db::query_p(
    transaction,
    r"select bot.user_id
      from bot
      left join active_game_player
      on bot.user_id = active_game_player.user_id
      where active_game_player.user_id is null
      limit $1",
    &[&(count as i64)],
  )
  .await?
  {
    let id: i64 = row.get(0);
    result.push(id);
  }

  // Create new bot users if needed.
  while result.len() != count {
    let user_id: i64 = db::query_one(
      transaction,
      r"insert into user_account values (default, '') returning user_id",
    )
    .await?
    .get(0);
    result.push(user_id);

    db::execute_p(transaction, r"insert into bot values ($1)", &[&user_id])
      .await?;

    db::execute_p(
      transaction,
      "update user_account set nickname = $1 where user_id = $2",
      &[
        &(String::from(
          bot_names::BOT_NAMES[user_id as usize % bot_names::BOT_NAMES.len()],
        ) + "🔩"),
        &user_id,
      ],
    )
    .await?;
  }

  return Ok(result);
}
