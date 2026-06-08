// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;
use crate::webapi::admin::auth;

#[derive(Clone)]
pub struct ServiceState {
  leaders: std::sync::Arc<business::leads::Leaders>,
  game_features: std::sync::Arc<business::game_features::Repository>,
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
 * Set the price of a game feature, creating the item if it does not exist.
 * This requires an administrator.
 *
 * Example:
 * [
 *   {"id": "feature-1", "coins": 200},
 *   {"id": "feature-2", "coins": 500}
 * ]
 */
async fn update(
  state_handle: axum::extract::State<ServiceState>,
  axum::Json(features): axum::Json<Vec<business::game_features::Feature>>,
) -> business::result::Result<()> {
  let game_features: &business::game_features::Repository =
    &state_handle.0.game_features;

  return game_features.batch_put(&features).await;
}

/// List all game features and their prices.
async fn list(
  state_handle: axum::extract::State<ServiceState>,
) -> business::result::Result<axum::Json<Vec<business::game_features::Feature>>>
{
  let game_features: &business::game_features::Repository =
    &state_handle.0.game_features;

  let mut features: Vec<business::game_features::Feature> =
    game_features.list().await?;

  features.sort_by(|lhs, rhs| lhs.id.cmp(&rhs.id));

  return Ok(axum::Json(features));
}

/// Configure all routes for this service.
pub fn route(
  leaders: std::sync::Arc<business::leads::Leaders>,
  game_features: std::sync::Arc<business::game_features::Repository>,
) -> axum::Router {
  let state = ServiceState {
    leaders,
    game_features,
  };

  return axum::Router::new()
    .route("/update", axum::routing::post(update))
    .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth))
    .route("/list", axum::routing::get(list))
    .with_state(state);
}
