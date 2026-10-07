// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;
use crate::webapi::client::auth;

#[derive(Clone)]
struct ServiceState
{
  db: deadpool_postgres::Pool
}

/// Middleware to validate that the request comes from known user.
async fn auth(
  state: axum::extract::State<ServiceState>,
  request: axum::extract::Request,
  next: axum::middleware::Next
) -> axum::response::Response<axum::body::Body>
{
  return auth::validate_request(&state.0.db, request, next).await;
}

#[derive(serde::Deserialize)]
struct ProfileRequest
{
  user_ids: Vec<i64>
}

#[derive(serde::Serialize)]
struct ProfileResponse
{
  profiles: Vec<business::users::ProfileResponse>
}

#[axum::debug_handler]
async fn profiles(
  user_id: axum::Extension<i64>,
  state: axum::extract::State<ServiceState>,
  axum::Json(request): axum::Json<ProfileRequest>
) -> business::result::Result<axum::Json<ProfileResponse>>
{
  let mut profiles: Vec<business::users::ProfileResponse> =
    business::users::profile(
      &state.0.db.get().await?,
      user_id.0,
      &request.user_ids
    )
    .await?;
  profiles.sort_by_key(|p| p.user_id);

  return Ok(axum::Json(ProfileResponse {
    profiles
  }));
}

pub fn route(db: deadpool_postgres::Pool) -> axum::Router
{
  let state = ServiceState {
    db
  };

  return axum::Router::new()
    .route("/profiles", axum::routing::post(profiles))
    .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth))
    .with_state(state);
}
