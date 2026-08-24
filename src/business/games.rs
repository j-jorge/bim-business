// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

struct Internals {
  m_date_of_last_clean_up: std::time::SystemTime,
}

pub struct Service {
  m_internals: tokio::sync::RwLock<Internals>,
}

impl Service {
  pub fn new() -> Service {
    return Service {
      m_internals: tokio::sync::RwLock::new(Internals {
        m_date_of_last_clean_up: std::time::SystemTime::now(),
      }),
    };
  }

  pub async fn maybe_run_clean_up_job(&self, db_pool: &db::Pool) -> () {
    if let Ok(mut client) = db_pool.get().await {
      let now = std::time::SystemTime::now();

      {
        let internals = self.m_internals.read().await;
        let clean_up_delay = std::time::Duration::from_mins(
          app_config::get_u64(&client, "games.clean_up_interval.minutes", 10)
            .await,
        );

        if internals.m_date_of_last_clean_up + clean_up_delay > now {
          return;
        }
      }

      let reward_lifespan = std::time::Duration::from_mins(
        app_config::get_u64(&client, "games.reward_lifespan.minutes", 1).await,
      );

      let auto_removal_duration = std::time::Duration::from_mins(
        app_config::get_u64(&client, "games.auto_removal.minutes", 30).await,
      );

      let mut internals = self.m_internals.write().await;

      if let Ok(transaction) = client.transaction().await
        // Remove non-claimed rewards.
        && let Ok(_) = db::execute_p(
          &transaction,
          r"delete from game_reward
            using done_game
            where game_reward.game_id = done_game.game_id
            and done_game.end_date < $1",
          &[&(now - reward_lifespan)],
        )
        .await
        // Remove players from games that are going on for too long.
        && let Ok(_) = db::execute_p(
          &transaction,
            r"delete from active_game_player
              using active_game
              where active_game_player.game_id = active_game.game_id
              and start_date < $1",
          &[&(now - auto_removal_duration)],
        )
        .await
        // Remove games that are going on for too long.
        && let Ok(_) = db::execute_p(
          &transaction,
          r"delete from active_game where start_date < $1",
          &[&(now - auto_removal_duration)],
        )
        .await
        && let Ok(_) = transaction.commit().await
      {
        internals.m_date_of_last_clean_up = now;
      }
    }
  }
}

#[derive(serde::Serialize)]
pub struct StartedResult {
  game_id: i64,
}

pub async fn started(
  transaction: &db::Transaction<'_>,
  game_server: i64,
  players: &[i64],
) -> result::Result<StartedResult> {
  let game_id: i64 = db::query_one_p(
    transaction,
    r"insert into game values(default, $1) returning game_id",
    &[&game_server],
  )
  .await?
  .get(0);

  db::execute_p(
    transaction,
    r"insert into active_game values ($1, $2)",
    &[&game_id, &std::time::SystemTime::now()],
  )
  .await?;

  let mut query = String::from(r"insert into active_game_player values");
  let mut parameters =
    Vec::<&(dyn tokio_postgres::types::ToSql + Sync)>::with_capacity(
      2 * players.len(),
    );
  let mut separator = ' ';

  for (i, player) in players.iter().enumerate() {
    query += &format!(r"{}(${}, ${})", separator, 2 * i + 1, 2 * i + 2);
    separator = ',';
    parameters.push(&game_id);
    parameters.push(player);
  }

  db::execute_p(transaction, &query, &parameters).await?;

  return Ok(StartedResult { game_id });
}

#[derive(serde::Deserialize, PartialEq, Debug, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum PlayerGameOutcome {
  Defeated,
  Kicked,
  Draw,
  Victory,
}

pub async fn over(
  transaction: &db::Transaction<'_>,
  game_server: i64,
  game_id: i64,
  game_duration: std::time::Duration,
  players: &[i64],
  outcome: &[PlayerGameOutcome],
) -> result::Result<()> {
  if players.len() != outcome.len() {
    tracing::warn!(
      "Game {} has inconsistent outcome: {} players and {} outcome.",
      game_id,
      players.len(),
      outcome.len()
    );
    return Err(error::Error::Unprocessable);
  }

  if outcome
    .iter()
    .filter(|&o| *o == PlayerGameOutcome::Victory)
    .count()
    > 1
  {
    tracing::warn!(
      "There can't be more than one victory in game {}. Outcome is {:?}.",
      game_id,
      outcome
    );
    return Err(error::Error::Unprocessable);
  }

  // Sanity check: the game_id should have been played on this game server.
  if !db::exists_p(
    transaction,
    r"select * from game where game_id = $1 and game_server_id = $2",
    &[&game_id, &game_server],
  )
  .await?
  {
    tracing::warn!(
      "Game {} was not played on server {}.",
      game_id,
      game_server
    );
    return Err(error::Error::Unprocessable);
  }

  // Remove the players from the active game, since the game is not
  // active anymore.
  let mut initial_players: Vec<i64> = db::collect_p(
    transaction,
    r"delete from active_game_player
      where game_id = $1
      returning user_id",
    &[&game_id],
    |r| r.get(0),
  )
  .await?;
  initial_players.sort();

  let mut sorted_players: Vec<i64> = players.to_vec();
  sorted_players.sort();

  // Sanity check again: the player should be in the game.
  if sorted_players != initial_players {
    tracing::warn!(
      "Players of game {} are different {:?} vs {:?}.",
      game_id,
      initial_players,
      sorted_players
    );
    return Err(error::Error::Unprocessable);
  }

  // The game is over, we can remove it from from the active games.
  let start_date: std::time::SystemTime = db::query_one_p(
    transaction,
    r"delete from active_game where game_id = $1 returning start_date",
    &[&game_id],
  )
  .await?
  .get(0);

  let now = std::time::SystemTime::now();

  let short_game: bool = game_duration.as_secs()
    < app_config::get_u64(
      transaction,
      "games.max_duration_for_short_game.seconds",
      30,
    )
    .await;

  // We can register that the game is done, since it is done…
  db::execute_p(
    transaction,
    r"insert into done_game values ($1, $2, $3, $4)",
    &[&game_id, &start_date, &now, &short_game],
  )
  .await?;

  // And also keep track of the players who have played this game.
  {
    let outcome_str: Vec<String> =
      outcome.iter().map(|o| o.to_string()).collect();
    let mut query = String::from(r"insert into done_game_player values");
    let mut parameters =
      Vec::<&(dyn tokio_postgres::types::ToSql + Sync)>::with_capacity(
        2 * players.len() + 1,
      );
    parameters.push(&game_id);

    let mut separator: char = ' ';

    for (i, p) in players.iter().enumerate() {
      query +=
        &format!(r"{}($1, ${}, ${})", separator, 2 * (i + 1), 2 * (i + 1) + 1);
      parameters.push(p);
      parameters.push(&outcome_str[i]);
      separator = ',';
    }

    db::execute_p(transaction, &query, &parameters).await?;
  }

  // Now distribute the rewards.
  let victory_reward: i64 = if short_game {
    app_config::get(transaction, "games.coins_per_short_game_victory", 0).await
  } else {
    app_config::get(transaction, "games.coins_per_victory", 10).await
  };
  let defeat_reward: i64 = if short_game {
    app_config::get(transaction, "games.coins_per_short_game_defeat", 0).await
  } else {
    app_config::get(transaction, "games.coins_per_defeat", 10).await
  };
  let draw_reward: i64 = if short_game {
    app_config::get(transaction, "games.coins_per_short_game_draw", 0).await
  } else {
    app_config::get(transaction, "games.coins_per_draw", 10).await
  };

  const BOT_PLACEHOLDER: i64 = 0;

  for (i, p) in players.iter().enumerate() {
    if *p == BOT_PLACEHOLDER {
      continue;
    }

    let (coins, reason) = match outcome[i] {
      PlayerGameOutcome::Defeated => (defeat_reward, "game-defeat"),
      PlayerGameOutcome::Kicked => {
        continue;
      }
      PlayerGameOutcome::Draw => (draw_reward, "game-draw"),
      PlayerGameOutcome::Victory => (victory_reward, "game-victory"),
    };

    if coins == 0 {
      db::execute_p(
        transaction,
        r"delete from game_reward where user_id = $1",
        &[p],
      )
      .await?;
    } else {
      db::execute_p(
        transaction,
        r"insert into game_reward
          values ($1, $2, $3)
          on conflict (user_id) do update set (game_id, coins) = ($1, $3)",
        &[&game_id, p, &coins],
      )
      .await?;
    }

    wallet::coins_transaction(transaction, *p, reason, coins).await?;
  }

  return Ok(());
}

pub async fn consume_reward(
  transaction: &db::Transaction<'_>,
  game_id: i64,
  user_id: i64,
) -> result::Result<i64> {
  let optional_row: Option<tokio_postgres::Row> = db::query_opt_p(
    transaction,
    r"delete from game_reward
      where game_id = $1
      and user_id = $2
      returning coins",
    &[&game_id, &user_id],
  )
  .await?;

  if let Some(row) = optional_row {
    let coins: i64 = row.get(0);
    return Ok(coins);
  }

  return Err(error::Error::BadParameter);
}
