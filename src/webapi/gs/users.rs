// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;
use crate::webapi::gs::auth;

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
  sessions: Vec<String>,
  tokens: Vec<i64>,
}

#[derive(serde::Serialize)]
struct UserIdResponse {
  tokens: Vec<i64>,
  user_ids: Vec<Option<i64>>,
}

async fn user_id(
  state: axum::extract::State<StateHandle>,
  axum::Json(request): axum::Json<UserIdRequest>,
) -> business::result::Result<axum::Json<UserIdResponse>> {
  let user_ids: Vec<Option<i64>> =
    business::sessions::to_user_id(&state.0.get().await?, &request.sessions)
      .await?;

  return Ok(axum::Json(UserIdResponse {
    tokens: request.tokens,
    user_ids,
  }));
}

/// Configure all routes for this service.
pub fn route(db: deadpool_postgres::Pool) -> axum::Router {
  return axum::Router::new()
    .route("/user-id", axum::routing::post(user_id))
    .route_layer(axum::middleware::from_fn_with_state(db.clone(), auth))
    .with_state(db);
}
