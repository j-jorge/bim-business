// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;
use crate::webapi::client::auth;

#[derive(Clone)]
struct ServiceState
{
  billing_service: std::sync::Arc<business::billing::Billing>,
  db: deadpool_postgres::Pool
}

/// Middleware to validate that the request comes from known game server.
async fn auth(
  state: axum::extract::State<ServiceState>,
  request: axum::extract::Request,
  next: axum::middleware::Next
) -> axum::response::Response<axum::body::Body>
{
  return auth::validate_request(&state.0.db, request, next).await;
}

#[derive(serde::Deserialize)]
struct ValidatePurchaseRequest
{
  sku: String,
  token: String
}

#[derive(serde::Serialize)]
struct ValidatePurchaseResponse
{
  coins: i64
}

#[axum::debug_handler]
async fn validate_purchase(
  user_id: axum::Extension<i64>,
  state: axum::extract::State<ServiceState>,
  axum::Json(request): axum::Json<ValidatePurchaseRequest>
) -> business::result::Result<axum::Json<ValidatePurchaseResponse>>
{
  let mut client: business::db::Client = state.0.db.get().await?;
  let transaction: business::db::Transaction<'_> = client.transaction().await?;

  let coins: i64 = state
    .0
    .billing_service
    .validate_purchase(&transaction, user_id.0, &request.sku, &request.token)
    .await?;

  transaction.commit().await?;

  return Ok(axum::Json(ValidatePurchaseResponse {
    coins
  }));
}

pub fn route(
  billing_service: std::sync::Arc<business::billing::Billing>,
  db: deadpool_postgres::Pool
) -> axum::Router
{
  let state = ServiceState {
    billing_service,
    db
  };

  return axum::Router::new()
    .route(
      "/billing/validate-purchase",
      axum::routing::post(validate_purchase)
    )
    .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth))
    .with_state(state);
}
