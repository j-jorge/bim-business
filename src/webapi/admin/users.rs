// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;
use crate::webapi::admin::auth;

#[derive(Clone)]
pub struct ServiceState {
  db: deadpool_postgres::Pool,
}

/// Middleware to validate that the request comes from a leader.
async fn auth(
  state: axum::extract::State<ServiceState>,
  request: axum::extract::Request,
  next: axum::middleware::Next,
) -> axum::response::Response<axum::body::Body> {
  return auth::validate_request(&state.0.db, request, next).await;
}

#[derive(serde::Deserialize)]
struct OverrideNicknameRequest {
  user_id: i64,
  nickname: String,
}

async fn override_nickname(
  state: axum::extract::State<ServiceState>,
  axum::Json(request): axum::Json<OverrideNicknameRequest>,
) -> business::result::Result<()> {
  let mut client: business::db::Client = state.0.db.get().await?;
  let transaction: business::db::Transaction<'_> = client.transaction().await?;

  business::users::override_nickname(
    &transaction,
    request.user_id,
    &request.nickname,
  )
  .await?;

  return Ok(transaction.commit().await?);
}

#[derive(serde::Deserialize)]
struct CoinsTransactionRequest {
  user_id: i64,
  amount: i64,
  reason: String,
}

async fn coins_transaction(
  state: axum::extract::State<ServiceState>,
  axum::Json(request): axum::Json<CoinsTransactionRequest>,
) -> business::result::Result<()> {
  let mut client: business::db::Client = state.db.get().await?;
  let transaction: business::db::Transaction<'_> = client.transaction().await?;

  business::wallet::admin_coins_transaction(
    &transaction,
    request.user_id,
    &request.reason,
    request.amount,
  )
  .await?;

  transaction.commit().await?;

  return Ok(());
}

/// Configure all routes for this service.
pub fn route(db: deadpool_postgres::Pool) -> axum::Router {
  let state = ServiceState { db };

  return axum::Router::new()
    .route("/override-nickname", axum::routing::post(override_nickname))
    .route("/coins-transaction", axum::routing::post(coins_transaction))
    .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth))
    .with_state(state);
}
