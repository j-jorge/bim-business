// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;
use crate::webapi::gs::auth;
use axum::response::IntoResponse;

type StateHandle = deadpool_postgres::Pool;

/// Middleware to validate that the request comes from a known game server.
async fn auth(
  state: axum::extract::State<StateHandle>,
  request: axum::extract::Request,
  next: axum::middleware::Next,
) -> axum::response::Response<axum::body::Body> {
  return auth::validate_request(&state.0, request, next).await;
}

#[derive(serde::Deserialize)]
struct UserIdRequest {
  session_token: String,
}

#[derive(serde::Serialize)]
struct UserIdResponse {
  user_id: i64,
}

async fn user_id(
  state: axum::extract::State<StateHandle>,
  axum::Json(request): axum::Json<UserIdRequest>,
) -> axum::response::Response<axum::body::Body> {
  if let Ok(db) = state.0.get().await
    && let Ok(option) =
      business::sessions::to_user_id(&db, &request.session_token).await
  {
    if let Some(user_id) = option {
      if let Ok(json) = serde_json::to_value(UserIdResponse { user_id })
        && let Ok(string) = serde_json::to_string(&json)
      {
        return string.into_response();
      }

      return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    return axum::http::StatusCode::NOT_FOUND.into_response();
  }

  return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
}

/// Configure all routes for this service.
pub fn route(db: deadpool_postgres::Pool) -> axum::Router {
  return axum::Router::new()
    .route("/user-id", axum::routing::post(user_id))
    .route_layer(axum::middleware::from_fn_with_state(db.clone(), auth))
    .with_state(db);
}
