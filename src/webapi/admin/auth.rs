// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;

use axum::response::IntoResponse;

pub fn extract_auth(
  headers: &axum::http::header::HeaderMap,
) -> Option<&axum::http::header::HeaderValue> {
  return headers.get(axum::http::header::AUTHORIZATION);
}

/// Check that the token in the authorization header is an element of
/// the leader list.
///
/// When the parameter allow_init is true then the function will pass
/// if there is no configured administrator (i.e. when the application
/// is executed for the first time).
async fn valid_admin_internal(
  db: &business::db::Client,
  auth_header: Option<&axum::http::header::HeaderValue>,
  allow_init: bool,
) -> business::result::Result<bool> {
  if let Some(header) = auth_header
    && let Ok(token_str) = header.to_str()
  {
    return Ok(
      business::leads::validate_token(db, token_str).await?
        || (allow_init
          && business::leads::is_in_initialization_state(db).await?),
    );
  } else {
    tracing::error!("Missing header in request.");
  }

  return Ok(false);
}

/// Check that the token in the authorization header is an element of
/// the leader list.
async fn valid_admin(
  db: &business::db::Client,
  auth_header: Option<&axum::http::header::HeaderValue>,
) -> business::result::Result<bool> {
  return valid_admin_internal(db, auth_header, false).await;
}

/// Check that the token in the authorization header is an element of
/// the leader list or else that there is no configured leader.
async fn weak_valid_admin(
  db: &business::db::Client,
  auth_header: Option<&axum::http::header::HeaderValue>,
) -> business::result::Result<bool> {
  return valid_admin_internal(db, auth_header, true).await;
}

/// Middleware to validate that the request comes from a leader.
pub async fn validate_request(
  db_pool: &deadpool_postgres::Pool,
  request: axum::extract::Request,
  next: axum::middleware::Next,
) -> axum::response::Response<axum::body::Body> {
  let db_result: Result<business::db::Client, _> = db_pool.get().await;

  if db_result.is_err() {
    tracing::error!("Failed to get a DB from the pool.");
    return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
  }

  let db: business::db::Client = db_result.unwrap();

  // I would have wanted to pass the request directly to
  // validate_admin but it does not work. See this discussion:
  //
  // middleware::from_fn fails if fn calls async fn with &Request argument.
  // https://github.com/tokio-rs/axum/discussions/2571
  //
  // The workaround of passing the request by value would not work
  // since I need the request for the call to next.run() below, so I
  // extract the authorization header here.
  let r: business::result::Result<bool> =
    valid_admin(&db, extract_auth(request.headers())).await;

  if r.is_err() {
    return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
  }

  if r.unwrap() {
    return next.run(request).await;
  }

  return (axum::http::StatusCode::UNAUTHORIZED).into_response();
}

/// Middleware to validate that the request comes from a leader **or
/// that no leader exists**.
pub async fn weak_validate_request(
  db_pool: &deadpool_postgres::Pool,
  request: axum::extract::Request,
  next: axum::middleware::Next,
) -> axum::response::Response<axum::body::Body> {
  let db_result: Result<business::db::Client, _> = db_pool.get().await;

  if db_result.is_err() {
    tracing::error!("Failed to get a DB from the pool.");
    return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
  }

  let db: business::db::Client = db_result.unwrap();

  let r: business::result::Result<bool> =
    weak_valid_admin(&db, extract_auth(request.headers())).await;

  if r.is_err() {
    return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
  }

  if r.unwrap() {
    return next.run(request).await;
  }

  return (axum::http::StatusCode::UNAUTHORIZED).into_response();
}
