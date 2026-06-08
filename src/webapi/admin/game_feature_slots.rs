// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;
use crate::webapi::admin::auth;

#[derive(Clone)]
pub struct ServiceState {
  leaders: std::sync::Arc<business::leads::Leaders>,
  game_feature_slots: std::sync::Arc<business::game_feature_slots::Repository>,
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
 * Set the price of a game feature slot, creating the item if it does
 * not exist. This requires an administrator.
 *
 * Example:
 * [
 *   {"index": 1, "coins": 200},
 *   {"index": 0, "coins": 500}
 * ]
 */
async fn update(
  state_handle: axum::extract::State<ServiceState>,
  axum::Json(slots): axum::Json<Vec<business::game_feature_slots::Slot>>,
) -> business::result::Result<()> {
  let game_feature_slots: &business::game_feature_slots::Repository =
    &state_handle.0.game_feature_slots;

  return game_feature_slots.batch_put(&slots).await;
}

async fn list(
  state_handle: axum::extract::State<ServiceState>,
) -> business::result::Result<axum::Json<Vec<business::game_feature_slots::Slot>>>
{
  let game_feature_slots: &business::game_feature_slots::Repository =
    &state_handle.0.game_feature_slots;

  let mut slots: Vec<business::game_feature_slots::Slot> =
    game_feature_slots.list().await?;
  slots.sort_by_key(|v| v.index);

  return Ok(axum::Json(slots));
}

/// Configure all routes for this service.
pub fn route(
  leaders: std::sync::Arc<business::leads::Leaders>,
  game_feature_slots: std::sync::Arc<business::game_feature_slots::Repository>,
) -> axum::Router {
  let state = ServiceState {
    leaders,
    game_feature_slots,
  };

  return axum::Router::new()
    .route("/update", axum::routing::post(update))
    .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth))
    .route("/list", axum::routing::get(list))
    .with_state(state);
}
