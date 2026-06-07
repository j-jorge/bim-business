// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;
use crate::webapi::gs::auth;

type JsonMap = std::collections::HashMap<String, serde_json::value::Value>;

type StateHandle = std::sync::Arc<business::game_servers::GameServers>;

/// Middleware to validate that the request comes from a leader.
async fn auth(
  state_handle: axum::extract::State<StateHandle>,
  request: axum::extract::Request,
  next: axum::middleware::Next,
) -> axum::response::Response<axum::body::Body> {
  return auth::validate_request(&state_handle.0, request, next).await;
}

#[derive(serde::Deserialize)]
struct HelloRequest {
  host: String,
  version: u64,
  protocol_version: u64,
}

/**
 * Write down that a game server is online, and with which clients it
 * can talk. The business answers with the expected delay for the next
 * notification from the game server.
 *
 * This requires a valid game server token, passed in the
 * Authorization header.
 *
 * Example:
 * {
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
async fn hello(
  headers: axum::http::header::HeaderMap,
  state_handle: axum::extract::State<StateHandle>,
  axum::response::Json(request): axum::response::Json<HelloRequest>,
) -> business::result::Result<String> {
  // The authorization header has been validated by the authorization layer.
  let token: &str = headers
    .get(axum::http::header::AUTHORIZATION)
    .unwrap()
    .to_str()
    .unwrap();
  let game_servers: &business::game_servers::GameServers = &state_handle.0;

  let callback_delay: std::time::Duration = game_servers
    .hello(
      token,
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

/// Configure all routes for this service.
pub fn route(state: StateHandle) -> axum::Router {
  return axum::Router::new()
    .route("/", axum::routing::post(hello))
    .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth))
    .with_state(state);
}
