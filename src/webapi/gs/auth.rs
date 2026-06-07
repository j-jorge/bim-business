// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;

use axum::response::IntoResponse;

/// Middleware to validate that the request comes from a game server.
pub async fn validate_request(
  game_servers: &business::game_servers::GameServers,
  request: axum::extract::Request,
  next: axum::middleware::Next,
) -> axum::response::Response<axum::body::Body> {
  if let Some(header) = request.headers().get(axum::http::header::AUTHORIZATION)
    && let Ok(token_str) = header.to_str()
    && game_servers.validate_token(token_str).await.is_ok()
  {
    return next.run(request).await;
  }

  return (axum::http::StatusCode::UNAUTHORIZED).into_response();
}
