// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;
use crate::webapi;
use crate::webapi::admin::auth;

use axum::response::IntoResponse;

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
 * Set the value associated with a given config key, creating the
 * entry if it does not exist. This requires an administrator.
 *
 * Example:
 * {
 *   "foo": 123,
 *   "bar": "baz"
 * }
 */
async fn update(
  state: axum::extract::State<ServiceState>,
  axum::Json(payload): axum::Json<serde_json::Value>,
) -> axum::response::Response<axum::body::Body> {
  // Iterate over payload keys and values.
  // Build a Vec<business::flat_client_config::Entry> from them.
  // Send everything to the business part.

  let serde_json::Value::Object(map) = payload else {
    return (axum::http::StatusCode::BAD_REQUEST).into_response();
  };

  let mut entries: Vec<business::flat_client_config::Entry> =
    Vec::with_capacity(map.len());

  for (key, value) in map {
    entries.push(business::flat_client_config::Entry {
      key,
      value: match value {
        serde_json::Value::String(s) => {
          business::flat_client_config::Value::Text(s)
        }
        serde_json::Value::Number(n) => match n.as_i64() {
          Some(v) => business::flat_client_config::Value::Int64(v),
          _ => {
            return (axum::http::StatusCode::BAD_REQUEST).into_response();
          }
        },
        _ => {
          return (axum::http::StatusCode::BAD_REQUEST).into_response();
        }
      },
    });
  }

  if let Ok(mut client) = state.0.db.get().await
    && let Ok(transaction) = client.transaction().await
    && let Ok(_) =
      business::flat_client_config::batch_put(&transaction, &entries).await
    && let Ok(_) = transaction.commit().await
  {
    return (axum::http::StatusCode::OK).into_response();
  }

  return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
}

/**
 * Delete the entries with the given keys. This requires an administrator.
 *
 * Example:
 * [
 *   "foo",
 *   "bar"
 * ]
 */
async fn erase(
  state: axum::extract::State<ServiceState>,
  axum::Json(keys): axum::Json<Vec<String>>,
) -> business::result::Result<()> {
  let mut client: business::db::Client = state.0.db.get().await?;
  let transaction: business::db::Transaction<'_> = client.transaction().await?;

  business::flat_client_config::batch_erase(&transaction, &keys).await?;

  return Ok(transaction.commit().await?);
}

/// List all config parameters.
async fn list(
  state: axum::extract::State<ServiceState>,
) -> business::result::Result<String> {
  let mut m: std::collections::HashMap<&str, serde_json::value::Value> =
    std::collections::HashMap::new();
  let entries: Vec<business::flat_client_config::Entry> =
    business::flat_client_config::all_entries(&state.0.db.get().await?).await?;

  webapi::flat_client_config::collect(&mut m, &entries)?;

  return Ok(serde_json::to_string(&m)?);
}

/// Configure all routes for this service.
pub fn route(db: deadpool_postgres::Pool) -> axum::Router {
  let state = ServiceState { db };

  return axum::Router::new()
    .route("/update", axum::routing::post(update))
    .route("/erase", axum::routing::post(erase))
    .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth))
    .route("/list", axum::routing::get(list))
    .with_state(state);
}
