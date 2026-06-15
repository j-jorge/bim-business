// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;
use crate::webapi::admin::auth;

type StateHandle = deadpool_postgres::Pool;

/// Middleware to validate that the request comes from a leader.
async fn auth(
  state: axum::extract::State<StateHandle>,
  request: axum::extract::Request,
  next: axum::middleware::Next,
) -> axum::response::Response<axum::body::Body> {
  return auth::validate_request(&state.0, request, next).await;
}

/// Middleware to validate that the request comes from a leader **or
/// that no leader exists**.
async fn weak_auth(
  state: axum::extract::State<StateHandle>,
  request: axum::extract::Request,
  next: axum::middleware::Next,
) -> axum::response::Response<axum::body::Body> {
  return auth::weak_validate_request(&state.0, request, next).await;
}

/// Public API, returns the tokens of all leaders. User must provide a valid
/// leader token.
async fn list_leaders(
  state: axum::extract::State<StateHandle>,
) -> business::result::Result<axum::Json<Vec<String>>> {
  let db: business::db::Client = state.0.get().await?;

  return Ok(axum::Json(business::leads::all_tokens(&db).await?));
}

/// Public API, creates a new leader token.
async fn create_leader(
  state: axum::extract::State<StateHandle>,
) -> business::result::Result<axum::Json<String>> {
  let db: business::db::Client = state.0.get().await?;

  return Ok(axum::Json(business::leads::create_token(&db).await?));
}

/// Configure all routes for this service.
pub fn route(db: deadpool_postgres::Pool) -> axum::Router {
  // Routes that require an authorization token.
  let strong_auth_routes = axum::Router::new()
    .route("/list", axum::routing::get(list_leaders))
    .route_layer(axum::middleware::from_fn_with_state(db.clone(), auth));

  // Routes that require an authorization token or being in the
  // initialization state (i.e. no configured administrator).
  let weak_auth_routes = axum::Router::new()
    .route("/create", axum::routing::post(create_leader))
    .route_layer(axum::middleware::from_fn_with_state(db.clone(), weak_auth));

  return axum::Router::new()
    .merge(strong_auth_routes)
    .merge(weak_auth_routes)
    .with_state(db);
}
