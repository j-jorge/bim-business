// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;
use crate::webapi::admin::auth;

#[derive(Clone)]
pub struct ServiceState {
  leaders: std::sync::Arc<business::leads::Leaders>,
  shop: std::sync::Arc<business::shop::Shop>,
}

/// Middleware to validate that the request comes from a leader.
async fn auth(
  state_handle: axum::extract::State<ServiceState>,
  request: axum::extract::Request,
  next: axum::middleware::Next,
) -> axum::response::Response<axum::body::Body> {
  return auth::validate_request(&state_handle.0.leaders, request, next).await;
}

/**
 * Register a shop product and its reward in coins, creating the item if it
 * does not exist. This requires an administrator.
 *
 * Example:
 * [
 *   {"id": "product-1", coins: 200},
 *   {"id": "product-2", coins: 500}
 * ]
 */
async fn update(
  state_handle: axum::extract::State<ServiceState>,
  axum::Json(products): axum::Json<Vec<business::shop::Product>>,
) -> business::result::Result<()> {
  return state_handle.0.shop.batch_put(&products).await;
}

/// List all shop products.
async fn list(
  state_handle: axum::extract::State<ServiceState>,
) -> business::result::Result<axum::Json<Vec<business::shop::Product>>> {
  let mut products: Vec<business::shop::Product> =
    state_handle.0.shop.list().await?;

  products.sort_by_key(|v| v.coins);

  return Ok(axum::Json(products));
}

/// Configure all routes for this service.
pub fn route(
  leaders: std::sync::Arc<business::leads::Leaders>,
  shop: std::sync::Arc<business::shop::Shop>,
) -> axum::Router {
  let state = ServiceState { leaders, shop };

  return axum::Router::new()
    .route("/update", axum::routing::post(update))
    .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth))
    .route("/list", axum::routing::get(list))
    .with_state(state);
}
