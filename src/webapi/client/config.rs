// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;
use crate::webapi;

#[derive(Clone)]
pub struct ServiceState {
  flat_config: std::sync::Arc<business::flat_client_config::Repository>,
  game_feature_slots: std::sync::Arc<business::game_feature_slots::Repository>,
  game_features: std::sync::Arc<business::game_features::Repository>,
  game_servers: std::sync::Arc<business::game_servers::GameServers>,
  shop: std::sync::Arc<business::shop::Shop>,
}

#[derive(serde::Deserialize)]
struct ConfigRequest {
  game_server_protocol_version: u64,
}

#[derive(serde::Serialize)]
struct ConfigResponse {
  pub misc: serde_json::value::Value,
  pub game_feature_slots: Vec<business::game_feature_slots::Slot>,
  pub game_features: Vec<business::game_features::Feature>,
  pub game_servers: Vec<String>,
  pub shop: Vec<business::shop::Product>,
}

/// Config to be sent to the client at launch time.
async fn client_config(
  state_handle: axum::extract::State<ServiceState>,
  axum::Json(request): axum::Json<ConfigRequest>,
) -> business::result::Result<axum::Json<ConfigResponse>> {
  let flat_config: &business::flat_client_config::Repository =
    &state_handle.0.flat_config;
  let flat_entries: Vec<business::flat_client_config::Entry> =
    flat_config.all_entries().await?;
  let mut misc: std::collections::HashMap<&str, serde_json::value::Value> =
    std::collections::HashMap::new();

  webapi::flat_client_config::collect(&mut misc, &flat_entries)?;

  let game_feature_slots: &business::game_feature_slots::Repository =
    &state_handle.0.game_feature_slots;
  let game_features: &business::game_features::Repository =
    &state_handle.0.game_features;
  let game_servers: &business::game_servers::GameServers =
    &state_handle.0.game_servers;
  let shop: &business::shop::Shop = &state_handle.0.shop;

  return Ok(axum::Json(ConfigResponse {
    misc: serde_json::to_value(misc)?,
    game_feature_slots: game_feature_slots.list().await?,
    game_features: game_features.list().await?,
    game_servers: game_servers
      .online_hosts_for_protocol(request.game_server_protocol_version)?,
    shop: shop.list().await?,
  }));
}

/// Configure all routes for this service.
pub fn route(
  flat_config: std::sync::Arc<business::flat_client_config::Repository>,
  game_feature_slots: std::sync::Arc<business::game_feature_slots::Repository>,
  game_features: std::sync::Arc<business::game_features::Repository>,
  game_servers: std::sync::Arc<business::game_servers::GameServers>,
  shop: std::sync::Arc<business::shop::Shop>,
) -> axum::Router {
  let state = ServiceState {
    flat_config,
    game_feature_slots,
    game_features,
    game_servers,
    shop,
  };

  return axum::Router::new()
    .route("/", axum::routing::post(client_config))
    .with_state(state);
}
