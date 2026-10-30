// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;

use axum::response::IntoResponse;

async fn find_server(
  db_pool: &deadpool_postgres::Pool,
  token: &str
) -> business::result::Result<Option<i64>>
{
  let client: deadpool_postgres::Client = db_pool.get().await?;
  let server_id: Option<i64> =
    business::game_servers::validate_token(&client, token).await?;

  return Ok(server_id);
}

/// Middleware to validate that the request comes from a game server.
pub async fn validate_request(
  db_pool: &deadpool_postgres::Pool,
  mut request: axum::extract::Request,
  next: axum::middleware::Next
) -> axum::response::Response<axum::body::Body>
{
  if let Some(header) = request.headers().get(axum::http::header::AUTHORIZATION)
    && let Ok(token_str) = header.to_str()
  {
    if let Ok(server_id_opt) = find_server(db_pool, token_str).await
    {
      if let Some(server_id) = server_id_opt
      {
        request.extensions_mut().insert(server_id);
        return next.run(request).await;
      }

      return (axum::http::StatusCode::UNAUTHORIZED).into_response();
    }

    return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
  }

  return (axum::http::StatusCode::UNAUTHORIZED).into_response();
}
