// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;
use crate::webapi::admin::auth;

#[derive(Clone)]
pub struct ServiceState {
  leaders: std::sync::Arc<business::leads::Leaders>,
  game_servers: std::sync::Arc<business::game_servers::GameServers>,
}

/// Middleware to validate that the request comes from a leader.
async fn auth(
  state_handle: axum::extract::State<ServiceState>,
  request: axum::extract::Request,
  next: axum::middleware::Next,
) -> axum::response::Response<axum::body::Body> {
  return auth::validate_request(&state_handle.0.leaders, request, next).await;
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
  state_handle: axum::extract::State<ServiceState>,
  axum::Json(request): axum::Json<RegisterRequest>,
) -> business::result::Result<axum::Json<String>> {
  let game_servers: &business::game_servers::GameServers =
    &state_handle.0.game_servers;

  let token: String = game_servers
    .register(&request.id, &request.description)
    .await?;

  return Ok(axum::Json(token));
}

#[derive(serde::Deserialize)]
struct SetTimeToLiveRequest {
  delay_in_minutes: u64,
}

/**
 * Changes the delay between two runs of the removal of inactive game servers.
 *
 * Example:
 * {
 *   "delay_in_minutes": 1
 * }
 */
async fn set_time_to_live(
  state_handle: axum::extract::State<ServiceState>,
  axum::Json(request): axum::Json<SetTimeToLiveRequest>,
) -> business::result::Result<()> {
  let game_servers: &business::game_servers::GameServers =
    &state_handle.0.game_servers;

  game_servers.set_clean_up_delay(std::time::Duration::from_mins(
    request.delay_in_minutes,
  ));

  return Ok(());
}

/// List all game servers. This requires an administrator.
async fn list(
  state_handle: axum::extract::State<ServiceState>,
) -> business::result::Result<
  axum::Json<Vec<business::game_servers::GameServerInfo>>,
> {
  let game_servers: &business::game_servers::GameServers =
    &state_handle.0.game_servers;

  return Ok(axum::Json(game_servers.all().await?));
}

/// Configure all routes for this service.
pub fn route(
  leaders: std::sync::Arc<business::leads::Leaders>,
  game_servers: std::sync::Arc<business::game_servers::GameServers>,
) -> axum::Router {
  let state = ServiceState {
    leaders,
    game_servers,
  };

  return axum::Router::new()
    .route("/register", axum::routing::post(register))
    .route("/list", axum::routing::get(list))
    .route("/set-time-to-live", axum::routing::post(set_time_to_live))
    .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth))
    .with_state(state);
}
