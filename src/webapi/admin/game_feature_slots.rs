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
  state: axum::extract::State<ServiceState>,
  axum::Json(slots): axum::Json<Vec<business::game_feature_slots::Slot>>,
) -> business::result::Result<()> {
  let mut client: business::db::Client = state.0.db.get().await?;
  let transaction: business::db::Transaction<'_> = client.transaction().await?;

  business::game_feature_slots::batch_put(&transaction, &slots).await?;

  return Ok(transaction.commit().await?);
}

async fn list(
  state: axum::extract::State<ServiceState>,
) -> business::result::Result<axum::Json<Vec<business::game_feature_slots::Slot>>>
{
  let mut slots: Vec<business::game_feature_slots::Slot> =
    business::game_feature_slots::list(&state.0.db.get().await?).await?;
  slots.sort_by_key(|v| v.index);

  return Ok(axum::Json(slots));
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
