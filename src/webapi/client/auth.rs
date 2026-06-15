// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;

use axum::response::IntoResponse;

/// Middleware to validate that the request comes from a valid user session.
pub async fn validate_request(
  db: &deadpool_postgres::Pool,
  mut request: axum::extract::Request,
  next: axum::middleware::Next,
) -> axum::response::Response<axum::body::Body> {
  if let Some(header) = request.headers().get(axum::http::header::AUTHORIZATION)
    && let Ok(token_str) = header.to_str()
  {
    if let Ok(mut client) = db.get().await
      && let Ok(t) = client.transaction().await
      && let Ok(user_id_opt) = business::sessions::refresh(&t, token_str).await
      && let Ok(_) = t.commit().await
    {
      if let Some(user_id) = user_id_opt {
        request.extensions_mut().insert(user_id);
        return next.run(request).await;
      }

      return (axum::http::StatusCode::UNAUTHORIZED).into_response();
    }

    return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
  }

  return (axum::http::StatusCode::UNAUTHORIZED).into_response();
}
