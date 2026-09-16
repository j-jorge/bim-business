// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;
use crate::webapi::admin::auth;

#[derive(Clone)]
pub struct ServiceState
{
  db: deadpool_postgres::Pool
}

/// Middleware to validate that the request comes from a leader.
async fn auth(
  state: axum::extract::State<ServiceState>,
  request: axum::extract::Request,
  next: axum::middleware::Next
) -> axum::response::Response<axum::body::Body>
{
  return auth::validate_request(&state.0.db, request, next).await;
}

async fn update(
  state: axum::extract::State<ServiceState>,
  axum::Json(payload): axum::Json<Vec<business::app_config::Entry>>
) -> business::result::Result<()>
{
  let mut client: business::db::Client = state.0.db.get().await?;
  let transaction: business::db::Transaction<'_> = client.transaction().await?;

  business::app_config::batch_put(&transaction, &payload).await?;

  return Ok(transaction.commit().await?);
}

async fn erase(
  state: axum::extract::State<ServiceState>,
  axum::Json(keys): axum::Json<Vec<String>>
) -> business::result::Result<()>
{
  let mut client: business::db::Client = state.0.db.get().await?;
  let transaction: business::db::Transaction<'_> = client.transaction().await?;

  business::app_config::batch_erase(&transaction, &keys).await?;

  return Ok(transaction.commit().await?);
}

async fn list(
  state: axum::extract::State<ServiceState>
) -> business::result::Result<
  axum::Json<std::collections::HashMap<String, String>>
>
{
  let result: std::collections::HashMap<String, String> =
    business::app_config::all_entries(&state.0.db.get().await?).await?;

  return Ok(axum::Json(result));
}

pub fn route(db: deadpool_postgres::Pool) -> axum::Router
{
  let state = ServiceState {
    db
  };

  return axum::Router::new()
    .route("/update", axum::routing::post(update))
    .route("/erase", axum::routing::post(erase))
    .route("/list", axum::routing::get(list))
    .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth))
    .with_state(state);
}
