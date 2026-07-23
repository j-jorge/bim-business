// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;
use crate::webapi::client::auth;

type StateHandle = deadpool_postgres::Pool;

/// Middleware to validate that the request comes from known game server.
async fn auth(
  state: axum::extract::State<StateHandle>,
  request: axum::extract::Request,
  next: axum::middleware::Next,
) -> axum::response::Response<axum::body::Body> {
  return auth::validate_request(&state.0, request, next).await;
}

#[derive(serde::Deserialize)]
struct ConsumeRewardRequest {
  game_id: i64,
}

#[derive(serde::Serialize)]
struct ConsumeRewardResponse {
  coins: i64,
}

#[axum::debug_handler]
async fn consume_reward(
  user_id: axum::Extension<i64>,
  state: axum::extract::State<StateHandle>,
  axum::Json(request): axum::Json<ConsumeRewardRequest>,
) -> business::result::Result<axum::Json<ConsumeRewardResponse>> {
  let mut client: business::db::Client = state.0.get().await?;
  let transaction: business::db::Transaction<'_> = client.transaction().await?;

  let coins: i64 =
    business::games::consume_reward(&transaction, request.game_id, user_id.0)
      .await?;

  transaction.commit().await?;

  return Ok(axum::Json(ConsumeRewardResponse { coins }));
}

pub fn route(db: deadpool_postgres::Pool) -> axum::Router {
  let state = db;

  return axum::Router::new()
    .route("/consume-reward", axum::routing::post(consume_reward))
    .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth))
    .with_state(state);
}
