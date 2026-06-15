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
  state: axum::extract::State<ServiceState>,
  axum::Json(features): axum::Json<Vec<business::game_features::Feature>>,
) -> business::result::Result<()> {
  let mut client: business::db::Client = state.0.db.get().await?;
  let transaction: business::db::Transaction<'_> = client.transaction().await?;

  business::game_features::batch_put(&transaction, &features).await?;

  return Ok(transaction.commit().await?);
}

/// List all game features and their prices.
async fn list(
  state: axum::extract::State<ServiceState>,
) -> business::result::Result<axum::Json<Vec<business::game_features::Feature>>>
{
  let mut features: Vec<business::game_features::Feature> =
    business::game_features::list(&state.0.db.get().await?).await?;

  features.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));

  return Ok(axum::Json(features));
}

/// Configure all routes for this service.
pub fn route(db: deadpool_postgres::Pool) -> axum::Router {
  let state = ServiceState { db };

  return axum::Router::new()
    .route("/update", axum::routing::post(update))
    .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth))
    .route("/list", axum::routing::get(list))
    .with_state(state);
}
