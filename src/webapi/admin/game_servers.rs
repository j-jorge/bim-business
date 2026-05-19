// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;
use crate::webapi::admin::auth;

type JsonMap = std::collections::HashMap<String, serde_json::value::Value>;

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
  axum::response::Json(request): axum::response::Json<RegisterRequest>,
) -> business::result::Result<String> {
  let game_servers: &business::game_servers::GameServers =
    &state_handle.0.game_servers;

  let token: String = game_servers
    .register(&request.id, &request.description)
    .await?;

  return Ok(serde_json::to_string(&token)?);
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
  axum::response::Json(request): axum::response::Json<SetTimeToLiveRequest>,
) -> business::result::Result<()> {
  let game_servers: &business::game_servers::GameServers =
    &state_handle.0.game_servers;

  game_servers.set_clean_up_delay(std::time::Duration::from_mins(
    request.delay_in_minutes,
  ));

  return Ok(());
}

#[derive(serde::Deserialize)]
struct KeepAliveRequest {
  token: String,
  host: String,
  version: u64,
  protocol_version: u64,
}

/**
 * Write down that a game server is online, and with which clients it
 * can talk. The business answers with the expected delay for the next
 * notification from the game server.
 *
 * Example:
 * {
 *   "token": "my-game-server-token",
 *   "host": "domain:port",
 *   "version": "Version of the server.",
 *   "protocol_version":
 *     "Version of the game protocol understood by the server."
 * }
 *
 * Response:
 * {
 *   "callback_delay_seconds": 124
 * }
 */
async fn keep_alive(
  state_handle: axum::extract::State<ServiceState>,
  axum::response::Json(request): axum::response::Json<KeepAliveRequest>,
) -> business::result::Result<String> {
  let game_servers: &business::game_servers::GameServers =
    &state_handle.0.game_servers;

  let callback_delay: std::time::Duration = game_servers
    .keep_alive(
      &request.token,
      request.host,
      request.version,
      request.protocol_version,
    )
    .await?;

  let mut result = JsonMap::new();
  result.insert(
    "callback_delay_seconds".to_string(),
    serde_json::to_value(callback_delay.as_secs())?,
  );

  return Ok(serde_json::to_string(&result)?);
}

/// List all game servers. This requires an administrator.
async fn list(
  state_handle: axum::extract::State<ServiceState>,
) -> business::result::Result<String> {
  let game_servers: &business::game_servers::GameServers =
    &state_handle.0.game_servers;

  let mut result = JsonMap::new();
  let servers_info: Vec<business::game_servers::GameServerInfo> =
    game_servers.all().await?;

  for server in servers_info {
    let mut server_info = JsonMap::new();

    server_info
      .insert("token".to_string(), serde_json::to_value(server.token)?);
    server_info.insert(
      "description".to_string(),
      serde_json::to_value(server.description)?,
    );
    server_info.insert(
      "registration_date".to_string(),
      serde_json::to_value(server.registration_date.to_rfc3339())?,
    );
    server_info.insert(
      "last_seen".to_string(),
      serde_json::to_value(server.last_seen.to_rfc3339())?,
    );

    if let Some(info) = server.info {
      let mut info_map = JsonMap::new();
      info_map.insert("host".to_string(), serde_json::to_value(info.host)?);
      info_map
        .insert("version".to_string(), serde_json::to_value(info.version)?);
      info_map.insert(
        "protocol_version".to_string(),
        serde_json::to_value(info.protocol_version)?,
      );
      server_info.insert("info".to_string(), serde_json::to_value(info_map)?);
      server_info.insert("online".to_string(), serde_json::to_value(true)?);
    } else {
      server_info.insert("online".to_string(), serde_json::to_value(false)?);
    }

    result.insert(server.id, serde_json::to_value(server_info)?);
  }

  return Ok(serde_json::to_string(&result)?);
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
    .route("/keep-alive", axum::routing::post(keep_alive))
    .with_state(state);
}
