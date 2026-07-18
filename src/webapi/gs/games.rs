// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;
use crate::webapi::gs::auth;

#[derive(Clone)]
struct StateHandle {
  games: std::sync::Arc<business::games::Service>,
  db: deadpool_postgres::Pool,
}

/// Middleware to validate that the request comes from known game server.
async fn auth(
  state: axum::extract::State<StateHandle>,
  request: axum::extract::Request,
  next: axum::middleware::Next,
) -> axum::response::Response<axum::body::Body> {
  return auth::validate_request(&state.0.db, request, next).await;
}

#[derive(serde::Deserialize)]
struct GameStartedNotification {
  players: Vec<i64>,
}

#[axum::debug_handler]
async fn game_started(
  server_id: axum::Extension<i64>,
  state: axum::extract::State<StateHandle>,
  axum::Json(request): axum::Json<GameStartedNotification>,
) -> business::result::Result<axum::Json<business::games::StartedResult>> {
  state.0.games.maybe_run_clean_up_job(&state.0.db).await;

  let mut client: business::db::Client = state.0.db.get().await?;
  let transaction: business::db::Transaction<'_> = client.transaction().await?;

  let result: business::games::StartedResult =
    business::games::started(&transaction, server_id.0, &request.players)
      .await?;

  transaction.commit().await?;

  return Ok(axum::Json(result));
}

#[derive(serde::Deserialize)]
struct GameOverNotification {
  game_id: i64,
  has_a_winner: bool,
  players: Vec<i64>,
  player_ranks: Vec<i8>,
}

#[axum::debug_handler]
async fn game_over(
  server_id: axum::Extension<i64>,
  state: axum::extract::State<StateHandle>,
  axum::Json(request): axum::Json<GameOverNotification>,
) -> business::result::Result<()> {
  let mut client: business::db::Client = state.0.db.get().await?;
  let transaction: business::db::Transaction<'_> = client.transaction().await?;

  business::games::over(
    &transaction,
    server_id.0,
    request.game_id,
    request.has_a_winner,
    &request.players,
    &request.player_ranks,
  )
  .await?;

  transaction.commit().await?;

  return Ok(());
}

/// Configure all routes for this service.
pub fn route(
  games: std::sync::Arc<business::games::Service>,
  db: deadpool_postgres::Pool,
) -> axum::Router {
  let state = StateHandle { games, db };

  return axum::Router::new()
    .route("/game-started", axum::routing::post(game_started))
    .route("/game-over", axum::routing::post(game_over))
    .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth))
    .with_state(state);
}
