// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;
use crate::webapi::admin::auth;

#[derive(Clone)]
pub struct ServiceState {
  db: deadpool_postgres::Pool,
  game_servers: std::sync::Arc<business::game_servers::GameServers>,
}

/// Middleware to validate that the request comes from a leader.
async fn auth(
  state: axum::extract::State<ServiceState>,
  request: axum::extract::Request,
  next: axum::middleware::Next,
) -> axum::response::Response<axum::body::Body> {
  return auth::validate_request(&state.0.db, request, next).await;
}

#[derive(serde::Deserialize)]
struct RegisterRequest {
  id: String,
  description: String,
}

/**
 * Register a new game server, creating a token for it. This requires
 * an administrator.
 *
 * Example:
 * {
 *   "id": "my-game-server",
 *   "description": "Some description."
 * }
 *
 * Response:
 * "some-token"
 */
async fn register(
  state: axum::extract::State<ServiceState>,
  axum::Json(request): axum::Json<RegisterRequest>,
) -> business::result::Result<axum::Json<String>> {
  let game_servers: &business::game_servers::GameServers =
    &state.0.game_servers;

  let token: String = game_servers
    .register(&state.0.db.get().await?, &request.id, &request.description)
    .await?;

  return Ok(axum::Json(token));
}

/// List all game servers. This requires an administrator.
async fn list(
  state: axum::extract::State<ServiceState>,
) -> business::result::Result<
  axum::Json<Vec<business::game_servers::GameServerInfo>>,
> {
  let game_servers: &business::game_servers::GameServers =
    &state.0.game_servers;

  return Ok(axum::Json(
    game_servers.all(&state.0.db.get().await?).await?,
  ));
}

/// Configure all routes for this service.
pub fn route(
  game_servers: std::sync::Arc<business::game_servers::GameServers>,
  db: deadpool_postgres::Pool,
) -> axum::Router {
  let state = ServiceState { db, game_servers };

  return axum::Router::new()
    .route("/register", axum::routing::post(register))
    .route("/list", axum::routing::get(list))
    .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth))
    .with_state(state);
}
