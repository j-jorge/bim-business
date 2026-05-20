// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;
use crate::webapi;

#[derive(Clone)]
pub struct ServiceState {
  flat_config: std::sync::Arc<business::flat_client_config::FlatClientConfig>,
  game_features: std::sync::Arc<business::game_features::GameFeatures>,
  game_servers: std::sync::Arc<business::game_servers::GameServers>,
  shop: std::sync::Arc<business::shop::Shop>,
}

#[derive(serde::Deserialize)]
struct ConfigRequest {
  game_server_protocol_version: u64,
}

/// Config to be sent to the client at launch time.
async fn client_config(
  state_handle: axum::extract::State<ServiceState>,
  axum::response::Json(request): axum::response::Json<ConfigRequest>,
) -> business::result::Result<String> {
  let mut config: std::collections::HashMap<String, serde_json::value::Value> =
    std::collections::HashMap::new();

  {
    let mut misc: std::collections::HashMap<&str, serde_json::value::Value> =
      std::collections::HashMap::new();

    let flat_config: &business::flat_client_config::FlatClientConfig =
      &state_handle.0.flat_config;
    let entries: Vec<business::flat_client_config::Entry> =
      flat_config.all_entries().await?;

    webapi::flat_client_config::collect(&mut misc, &entries)?;
    config.insert("misc".to_string(), serde_json::to_value(misc)?);
  }

  {
    let game_features: &business::game_features::GameFeatures =
      &state_handle.0.game_features;

    config.insert(
      "game-feature-prices".to_string(),
      serde_json::to_value(game_features.list().await?)?,
    );
  }

  {
    let game_servers: &business::game_servers::GameServers =
      &state_handle.0.game_servers;

    config.insert(
      "game-servers".to_string(),
      serde_json::to_value(
        game_servers
          .online_hosts_for_protocol(request.game_server_protocol_version)?,
      )?,
    );
  }

  {
    let shop: &business::shop::Shop = &state_handle.0.shop;

    config.insert(
      "shop".to_string(),
      serde_json::to_value(shop.list().await?)?,
    );
  }

  return Ok(serde_json::to_string(&config)?);
}

/// Configure all routes for this service.
pub fn route(
  flat_config: std::sync::Arc<business::flat_client_config::FlatClientConfig>,
  game_features: std::sync::Arc<business::game_features::GameFeatures>,
  game_servers: std::sync::Arc<business::game_servers::GameServers>,
  shop: std::sync::Arc<business::shop::Shop>,
) -> axum::Router {
  let state = ServiceState {
    flat_config,
    game_features,
    game_servers,
    shop,
  };

  return axum::Router::new()
    .route("/", axum::routing::post(client_config))
    .with_state(state);
}
