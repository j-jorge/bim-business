// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;
use crate::webapi;

#[derive(Clone)]
struct ServiceState {
  game_servers: std::sync::Arc<business::game_servers::GameServers>,
  db: deadpool_postgres::Pool,
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
#[axum::debug_handler]
async fn client_config(
  state: axum::extract::State<ServiceState>,
  axum::Json(request): axum::Json<ConfigRequest>,
) -> business::result::Result<axum::Json<ConfigResponse>> {
  let db: business::db::Client = state.0.db.get().await?;
  let flat_entries: Vec<business::flat_client_config::Entry> =
    business::flat_client_config::all_entries(&db).await?;
  let mut misc: std::collections::HashMap<&str, serde_json::value::Value> =
    std::collections::HashMap::new();

  webapi::flat_client_config::collect(&mut misc, &flat_entries)?;

  let game_servers: &business::game_servers::GameServers =
    &state.0.game_servers;

  return Ok(axum::Json(ConfigResponse {
    misc: serde_json::to_value(misc)?,
    game_feature_slots: business::game_feature_slots::list(&db).await?,
    game_features: business::game_features::list(&db).await?,
    game_servers: game_servers
      .online_hosts_for_protocol(&db, request.game_server_protocol_version)
      .await?,
    shop: business::shop::list(&db).await?,
  }));
}

/// Configure all routes for this service.
pub fn route(
  game_servers: std::sync::Arc<business::game_servers::GameServers>,
  db: deadpool_postgres::Pool,
) -> axum::Router {
  let state = ServiceState { game_servers, db };

  return axum::Router::new()
    .route("/", axum::routing::post(client_config))
    .with_state(state);
}
