// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;
use crate::webapi::admin::auth;

use axum::response::IntoResponse;

#[derive(Clone)]
pub struct ServiceState {
  leaders: std::sync::Arc<business::leads::Leaders>,
  flat_config: std::sync::Arc<business::flat_client_config::FlatClientConfig>,
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
  state_handle: axum::extract::State<ServiceState>,
  axum::response::Json(payload): axum::response::Json<serde_json::Value>,
) -> axum::response::Response<axum::body::Body> {
  let flat_config: &business::flat_client_config::FlatClientConfig =
    &state_handle.0.flat_config;

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

  return flat_config.batch_put(&entries).await.into_response();
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
  state_handle: axum::extract::State<ServiceState>,
  axum::response::Json(keys): axum::response::Json<Vec<String>>,
) -> business::result::Result<()> {
  let flat_config: &business::flat_client_config::FlatClientConfig =
    &state_handle.0.flat_config;

  return flat_config.batch_erase(&keys).await;
}

/// List all config parameters.
async fn list(
  state_handle: axum::extract::State<ServiceState>,
) -> business::result::Result<String> {
  let flat_config: &business::flat_client_config::FlatClientConfig =
    &state_handle.0.flat_config;

  let mut m: std::collections::HashMap<&str, serde_json::value::Value> =
    std::collections::HashMap::new();
  let entries: Vec<business::flat_client_config::Entry> =
    flat_config.all_entries().await?;

  for entry in &entries {
    match &entry.value {
      business::flat_client_config::Value::Int64(v) => {
        m.insert(&entry.key, serde_json::to_value(v)?)
      }
      business::flat_client_config::Value::Text(v) => {
        m.insert(&entry.key, serde_json::to_value(v)?)
      }
    };
  }

  return Ok(serde_json::to_string(&m)?);
}

/// Configure all routes for this service.
pub fn route(
  leaders: std::sync::Arc<business::leads::Leaders>,
  flat_config: std::sync::Arc<business::flat_client_config::FlatClientConfig>,
) -> axum::Router {
  let state = ServiceState {
    leaders,
    flat_config,
  };

  return axum::Router::new()
    .route("/update", axum::routing::post(update))
    .route("/erase", axum::routing::post(erase))
    .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth))
    .route("/list", axum::routing::get(list))
    .with_state(state);
}
