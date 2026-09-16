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

#[derive(serde::Deserialize)]
struct OverrideNicknameRequest
{
  user_id: i64,
  nickname: String
}

async fn override_nickname(
  state: axum::extract::State<ServiceState>,
  axum::Json(request): axum::Json<OverrideNicknameRequest>
) -> business::result::Result<()>
{
  let mut client: business::db::Client = state.0.db.get().await?;
  let transaction: business::db::Transaction<'_> = client.transaction().await?;

  business::users::override_nickname(
    &transaction,
    request.user_id,
    &request.nickname
  )
  .await?;

  return Ok(transaction.commit().await?);
}

#[derive(serde::Deserialize)]
struct RestoreNicknameRequest
{
  user_id: i64
}

async fn restore_nickname(
  state: axum::extract::State<ServiceState>,
  axum::Json(request): axum::Json<RestoreNicknameRequest>
) -> business::result::Result<()>
{
  let mut client: business::db::Client = state.0.db.get().await?;
  let transaction: business::db::Transaction<'_> = client.transaction().await?;

  business::users::restore_nickname(&transaction, request.user_id).await?;

  return Ok(transaction.commit().await?);
}

#[derive(serde::Deserialize)]
struct InfoRequest
{
  user_id: i64
}

#[derive(serde::Serialize)]
struct InfoResponse
{
  profile: business::users::ProfileResponse,
  public_nickname: String,
  coins: i64,
  feature_slots: Vec<business::inventory::GameFeatureSlotState>,
  available_features: Vec<String>,
  arena_stats: business::users::ArenaStatsResponse,
  game_history: Vec<business::games::HistoryEntry>,
  transactions: Vec<business::wallet::HistoryEntry>,
  devices: Vec<String>
}

async fn info(
  state: axum::extract::State<ServiceState>,
  axum::Json(request): axum::Json<InfoRequest>
) -> business::result::Result<axum::Json<InfoResponse>>
{
  let db: business::db::Client = state.0.db.get().await?;
  let mut profiles: Vec<business::users::ProfileResponse> =
    business::users::profile(&db, request.user_id, &[request.user_id]).await?;

  if profiles.len() != 1
  {
    return Err(business::error::Error::BadParameter);
  }

  let profile: business::users::ProfileResponse = profiles.remove(0);
  let public_nickname: String =
    business::users::overridden_nickname(&db, request.user_id)
      .await?
      .unwrap_or(profile.nickname.clone());

  let r = InfoResponse {
    profile,
    public_nickname,
    coins: business::wallet::coins_balance(&db, request.user_id).await?,
    arena_stats: business::users::arena_stats(&db, request.user_id).await?,
    feature_slots: business::inventory::user_selected_game_features(
      &db,
      request.user_id
    )
    .await?,
    available_features: business::inventory::user_available_game_features(
      &db,
      request.user_id
    )
    .await?,
    game_history: business::games::history(&db, request.user_id).await?,
    transactions: business::wallet::history(&db, request.user_id).await?,
    devices: business::sessions::devices(&db, request.user_id).await?
  };

  return Ok(axum::Json(r));
}

#[derive(serde::Deserialize)]
struct CoinsTransactionRequest
{
  user_id: i64,
  amount: i64,
  reason: String
}

async fn coins_transaction(
  state: axum::extract::State<ServiceState>,
  axum::Json(request): axum::Json<CoinsTransactionRequest>
) -> business::result::Result<()>
{
  let mut client: business::db::Client = state.db.get().await?;
  let transaction: business::db::Transaction<'_> = client.transaction().await?;

  business::wallet::admin_coins_transaction(
    &transaction,
    request.user_id,
    &request.reason,
    request.amount
  )
  .await?;

  transaction.commit().await?;

  return Ok(());
}

/// Configure all routes for this service.
pub fn route(db: deadpool_postgres::Pool) -> axum::Router
{
  let state = ServiceState {
    db
  };

  return axum::Router::new()
    .route("/override-nickname", axum::routing::post(override_nickname))
    .route("/restore-nickname", axum::routing::post(restore_nickname))
    .route("/coins-transaction", axum::routing::post(coins_transaction))
    .route("/info", axum::routing::post(info))
    .route_layer(axum::middleware::from_fn_with_state(state.clone(), auth))
    .with_state(state);
}
