// SPDX-License-Identifier: AGPL-3.0-only
use crate::business;
use crate::webapi::client::auth;

#[derive(Clone)]
struct ServiceState {
  session_service: std::sync::Arc<business::sessions::Service>,
  db: deadpool_postgres::Pool,
}

/// Middleware to validate that the request comes from known game server.
async fn auth(
  state: axum::extract::State<ServiceState>,
  request: axum::extract::Request,
  next: axum::middleware::Next,
) -> axum::response::Response<axum::body::Body> {
  return auth::validate_request(&state.0.db, request, next).await;
}

#[derive(serde::Deserialize)]
struct AuthenticationRequest {
  device_id: String,
}

#[axum::debug_handler]
async fn authenticate(
  state: axum::extract::State<ServiceState>,
  axum::Json(request): axum::Json<AuthenticationRequest>,
) -> business::result::Result<
  axum::Json<business::sessions::AuthenticationResponse>,
> {
  let mut client: business::db::Client = state.0.db.get().await?;
  let transaction: business::db::Transaction<'_> = client.transaction().await?;
  let session_service: &business::sessions::Service = &state.0.session_service;

  let response: business::sessions::AuthenticationResponse = session_service
    .authenticate(&transaction, request.device_id)
    .await?;

  transaction.commit().await?;

  return Ok(axum::Json(response));
}

#[derive(serde::Deserialize)]
struct UpdateNicknameRequest {
  pub nickname: String,
}

#[axum::debug_handler]
async fn update_nickname(
  user_id: axum::Extension<i64>,
  state: axum::extract::State<ServiceState>,
  axum::Json(request): axum::Json<UpdateNicknameRequest>,
) -> business::result::Result<()> {
  let mut client: business::db::Client = state.0.db.get().await?;
  let transaction: business::db::Transaction<'_> = client.transaction().await?;

  business::users::set_nickname(&transaction, user_id.0, &request.nickname)
    .await?;

  return Ok(transaction.commit().await?);
}

#[axum::debug_handler]
async fn profile(
  user_id: axum::Extension<i64>,
  state: axum::extract::State<ServiceState>,
  axum::Json(request): axum::Json<Vec<i64>>,
) -> business::result::Result<axum::Json<Vec<business::users::ProfileResponse>>>
{
  let mut profiles: Vec<business::users::ProfileResponse> =
    business::users::profile(&state.0.db.get().await?, user_id.0, &request)
      .await?;
  profiles.sort_by_key(|p| p.user_id);

  return Ok(axum::Json(profiles));
}

#[derive(serde::Deserialize)]
struct TransferLegacyRequest {
  coins: i64,
  game_features: Vec<String>,
  slots: Vec<i16>,
  game_feature_selection: Vec<business::inventory::GameFeatureSlotSelection>,
  arena_stats: business::legacy::GameStatistics,
}

#[derive(serde::Serialize)]
struct TransferLegacyResponse {
  transfer_state: i64,
}

async fn transfer_legacy(
  user_id: axum::Extension<i64>,
  state: axum::extract::State<ServiceState>,
  axum::Json(request): axum::Json<TransferLegacyRequest>,
) -> business::result::Result<axum::Json<TransferLegacyResponse>> {
  let mut client: business::db::Client = state.0.db.get().await?;
  let transaction: business::db::Transaction<'_> = client.transaction().await?;

  let transfer_state: i64 = match business::legacy::transfer(
    &transaction,
    user_id.0,
    request.coins,
    &request.game_features,
    &request.slots,
    &request.game_feature_selection,
    &request.arena_stats,
  )
  .await?
  {
    business::legacy::TransferResult::Disabled => 0,
    business::legacy::TransferResult::Done => 1,
    business::legacy::TransferResult::AlreadyDone => 2,
  };

  if transfer_state == 1 {
    transaction.commit().await?;
  }

  return Ok(axum::Json(TransferLegacyResponse { transfer_state }));
}

#[derive(serde::Serialize)]
struct GameFeatureInventoryResponse {
  slots: Vec<business::inventory::GameFeatureSlotState>,
  available_features: Vec<String>,
}

#[axum::debug_handler]
async fn game_feature_inventory(
  user_id: axum::Extension<i64>,
  state: axum::extract::State<ServiceState>,
) -> business::result::Result<axum::Json<GameFeatureInventoryResponse>> {
  let db: business::db::Client = state.0.db.get().await?;

  let mut r = GameFeatureInventoryResponse {
    slots: business::inventory::user_selected_game_features(&db, user_id.0)
      .await?,
    available_features: business::inventory::user_available_game_features(
      &db, user_id.0,
    )
    .await?,
  };

  r.slots.sort_by_key(|s| s.slot_index);
  r.available_features.sort();

  return Ok(axum::Json(r));
}

#[derive(serde::Deserialize)]
struct GameFeatureSlotPurchaseRequest {
  slot_index: i16,
}

#[axum::debug_handler]
async fn buy_game_feature_slot(
  user_id: axum::Extension<i64>,
  state: axum::extract::State<ServiceState>,
  axum::Json(request): axum::Json<GameFeatureSlotPurchaseRequest>,
) -> business::result::Result<()> {
  tracing::info!(
    "User {} buys game feature slot {}.",
    user_id.0,
    request.slot_index
  );

  let mut client: business::db::Client = state.0.db.get().await?;
  let transaction: business::db::Transaction<'_> = client.transaction().await?;

  business::inventory::user_buy_game_feature_slot(
    &transaction,
    user_id.0,
    request.slot_index,
  )
  .await?;

  transaction.commit().await?;

  return Ok(());
}

#[derive(serde::Deserialize)]
struct GameFeaturePurchaseRequest {
  feature_name: String,
}

#[axum::debug_handler]
async fn buy_game_feature(
  user_id: axum::Extension<i64>,
  state: axum::extract::State<ServiceState>,
  axum::Json(request): axum::Json<GameFeaturePurchaseRequest>,
) -> business::result::Result<()> {
  tracing::info!(
    "User {} buys game feature {}.",
    user_id.0,
    request.feature_name
  );

  let mut client: business::db::Client = state.0.db.get().await?;
  let transaction: business::db::Transaction<'_> = client.transaction().await?;

  business::inventory::user_buy_game_feature(
    &transaction,
    user_id.0,
    &request.feature_name,
  )
  .await?;

  transaction.commit().await?;

  return Ok(());
}

#[derive(serde::Deserialize)]
struct GameFeatureSlotSelectionRequest {
  selection: Vec<business::inventory::GameFeatureSlotSelection>,
}

#[axum::debug_handler]
async fn assign_game_feature_slots(
  user_id: axum::Extension<i64>,
  state: axum::extract::State<ServiceState>,
  axum::Json(request): axum::Json<GameFeatureSlotSelectionRequest>,
) -> business::result::Result<()> {
  let mut client: business::db::Client = state.0.db.get().await?;
  let transaction: business::db::Transaction<'_> = client.transaction().await?;

  business::inventory::user_select_game_features(
    &transaction,
    user_id.0,
    &request.selection,
  )
  .await?;

  transaction.commit().await?;

  return Ok(());
}

#[axum::debug_handler]
async fn clear_game_feature_slot(
  user_id: axum::Extension<i64>,
  state: axum::extract::State<ServiceState>,
  axum::Json(slot_index): axum::Json<i16>,
) -> business::result::Result<()> {
  let mut client: business::db::Client = state.0.db.get().await?;
  let transaction: business::db::Transaction<'_> = client.transaction().await?;

  business::inventory::user_clear_game_feature_slot(
    &transaction,
    user_id.0,
    slot_index,
  )
  .await?;

  transaction.commit().await?;

  return Ok(());
}

#[derive(serde::Serialize)]
struct WalletResponse {
  coins: i64,
}

#[axum::debug_handler]
async fn wallet(
  user_id: axum::Extension<i64>,
  state: axum::extract::State<ServiceState>,
) -> business::result::Result<axum::Json<WalletResponse>> {
  return Ok(axum::Json(WalletResponse {
    coins: business::wallet::coins_balance(&state.0.db.get().await?, user_id.0)
      .await?,
  }));
}

pub fn route(
  session_service: std::sync::Arc<business::sessions::Service>,
  db: deadpool_postgres::Pool,
) -> axum::Router {
  let state = ServiceState {
    session_service,
    db,
  };

  return axum::Router::new()
    .route(
      "/account/update-nickname",
      axum::routing::post(update_nickname),
    )
    .route("/profile", axum::routing::post(profile))
    .route(
      "/transfer-legacy-inventory",
      axum::routing::post(transfer_legacy),
    )
    .route(
      "/game-feature/inventory",
      axum::routing::post(game_feature_inventory),
    )
    .route(
      "/game-feature/buy-slot",
      axum::routing::post(buy_game_feature_slot),
    )
    .route(
      "/game-feature/buy-feature",
      axum::routing::post(buy_game_feature),
    )
    .route(
      "/game-feature/assign-slots",
      axum::routing::post(assign_game_feature_slots),
    )
    .route(
      "/game-feature/clear-slot",
      axum::routing::post(clear_game_feature_slot),
    )
    .route("/wallet", axum::routing::post(wallet))
    .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth))
    .route("/authenticate", axum::routing::post(authenticate))
    .with_state(state);
}
