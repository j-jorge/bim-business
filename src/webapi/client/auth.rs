// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;

use axum::response::IntoResponse;

async fn refresh_session(
  db: &deadpool_postgres::Pool,
  token: &str
) -> business::result::Result<Option<i64>>
{
  let mut client: deadpool_postgres::Client = db.get().await?;
  let t: deadpool_postgres::Transaction<'_> = client.transaction().await?;
  let user_id: Option<i64> = business::sessions::refresh(&t, token).await?;
  t.commit().await?;

  return Ok(user_id);
}

/// Middleware to validate that the request comes from a valid user session.
pub async fn validate_request(
  db: &deadpool_postgres::Pool,
  mut request: axum::extract::Request,
  next: axum::middleware::Next
) -> axum::response::Response<axum::body::Body>
{
  if let Some(header) = request.headers().get(axum::http::header::AUTHORIZATION)
    && let Ok(token_str) = header.to_str()
  {
    if let Ok(user_id_opt) = refresh_session(db, token_str).await
    {
      if let Some(user_id) = user_id_opt
      {
        request.extensions_mut().insert(user_id);
        return next.run(request).await;
      }

      return (axum::http::StatusCode::UNAUTHORIZED).into_response();
    }

    return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
  }

  return (axum::http::StatusCode::UNAUTHORIZED).into_response();
}
