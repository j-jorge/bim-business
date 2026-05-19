// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;
use crate::webapi::admin::auth;

type StateHandle = std::sync::Arc<business::leads::Leaders>;

/// Middleware to validate that the request comes from a leader.
async fn auth(
  state_handle: axum::extract::State<StateHandle>,
  request: axum::extract::Request,
  next: axum::middleware::Next,
) -> axum::response::Response<axum::body::Body> {
  return auth::validate_request(&state_handle.0, request, next).await;
}

/// Middleware to validate that the request comes from a leader **or
/// that no leader exists**.
async fn weak_auth(
  state_handle: axum::extract::State<StateHandle>,
  request: axum::extract::Request,
  next: axum::middleware::Next,
) -> axum::response::Response<axum::body::Body> {
  return auth::weak_validate_request(&state_handle.0, request, next).await;
}

/// Public API, returns the tokens of all leaders. User must provide a valid
/// leader token.
async fn list_leaders(
  state_handle: axum::extract::State<StateHandle>,
) -> business::result::Result<String> {
  let leaders: &business::leads::Leaders = &state_handle.0;
  let result: String = serde_json::to_string(&leaders.all_tokens().await?)?;

  return Ok(result);
}

/// Public API, creates a new leader token.
async fn create_leader(
  state_handle: axum::extract::State<StateHandle>,
) -> business::result::Result<String> {
  let leaders: &business::leads::Leaders = &state_handle.0;
  let result: String = serde_json::to_string(&leaders.create_token().await?)?;

  return Ok(result);
}

/// Configure all routes for this service.
pub fn route(state: StateHandle) -> axum::Router {
  // Routes that require an authorization token.
  let strong_auth_routes = axum::Router::new()
    .route("/list", axum::routing::get(list_leaders))
    .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth));

  // Routes that require an authorization token or being in the
  // initialization state (i.e. no configured administrator).
  let weak_auth_routes = axum::Router::new()
    .route("/create", axum::routing::post(create_leader))
    .route_layer(axum::middleware::from_fn_with_state(
      state.clone(),
      weak_auth,
    ));

  return axum::Router::new()
    .merge(strong_auth_routes)
    .merge(weak_auth_routes)
    .with_state(state);
}
