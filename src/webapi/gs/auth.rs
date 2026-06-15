// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;

use axum::response::IntoResponse;

/// Middleware to validate that the request comes from a game server.
pub async fn validate_request(
  game_servers: &business::game_servers::GameServers,
  db_pool: &deadpool_postgres::Pool,
  request: axum::extract::Request,
  next: axum::middleware::Next,
) -> axum::response::Response<axum::body::Body> {
  if let Some(header) = request.headers().get(axum::http::header::AUTHORIZATION)
    && let Ok(token_str) = header.to_str()
  {
    if let Ok(db) = db_pool.get().await
      && let Ok(valid) = game_servers.validate_token(&db, token_str).await
    {
      if valid {
        return next.run(request).await;
      }

      return (axum::http::StatusCode::UNAUTHORIZED).into_response();
    }

    return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
  }

  return (axum::http::StatusCode::UNAUTHORIZED).into_response();
}
