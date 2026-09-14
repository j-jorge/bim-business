// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

pub struct GoogleConfig
{
  pub package_id: String,
  pub credentials_path: std::path::PathBuf
}

// This is the response sent by Google when querying a purchase.
#[derive(serde::Deserialize)]
struct ProductPurchase
{
  #[serde(rename = "purchaseState")]
  purchase_state: i32,

  #[serde(rename = "consumptionState")]
  consumption_state: i32,

  #[serde(rename = "orderId")]
  order_id: String,

  quantity: Option<i32>
}

struct GoogleValidationResources
{
  package_id: String,
  credentials: google_cloud_auth::credentials::Credentials,
  http_client: reqwest::Client,
  headers: http::HeaderMap
}

async fn validate_with_google(
  google: &mut GoogleValidationResources,
  t: &db::Transaction<'_>,
  user_id: i64,
  sku: &str,
  token: &str
) -> result::Result<(String, i32)>
{
  if let google_cloud_auth::credentials::CacheableResource::New {
    entity_tag: _,
    data: r
  } = google.credentials.headers(http::Extensions::new()).await?
  {
    google.headers = r;
  }

  const ENDPOINT: &str =
    "https://androidpublisher.googleapis.com/androidpublisher/v3/applications";

  let google_validation_url: String = format!(
    "{}/{}/purchases/products/{}/tokens/{}",
    ENDPOINT, google.package_id, sku, token
  );

  // First, let's check the current state of the purchase according to Google.
  let response: reqwest::Response = google
    .http_client
    .get(&google_validation_url)
    .headers(google.headers.clone())
    .send()
    .await?
    .error_for_status()?;

  let body: String = response.text().await?;
  let purchase: ProductPurchase = serde_json::from_str(&body)?;

  const PURCHASE_STATE_PURCHASED: i32 = 0;
  const CONSUMPTION_STATE_CONSUMED: i32 = 1;

  // If it's not purchased, or if it's already consumed, it should not
  // be requested.
  if (purchase.purchase_state != PURCHASE_STATE_PURCHASED)
    || (purchase.consumption_state == CONSUMPTION_STATE_CONSUMED)
  {
    return Err(error::Error::BadParameter);
  }

  // Record the purchase.
  let user_opt: Option<tokio_postgres::Row> = db::query_opt_p(
    t,
    "insert into play_store_purchase
     values ($1, $2, $3)
     on conflict do nothing
     returning user_id",
    &[&purchase.order_id, &token, &user_id]
  )
  .await?;

  // If insert failed (the order is already in the table), it means
  // that something weird is going on. We would be applying a
  // purchase that is both non-consumed and yet is recorded in the
  // table.
  if user_opt.is_none()
  {
    return Err(error::Error::BadParameter);
  }

  let google_consume_url: String = format!(
    "{}/{}/purchases/products/{}/tokens/{}:consume",
    ENDPOINT, google.package_id, sku, token
  );

  google
    .http_client
    .post(&google_consume_url)
    .headers(google.headers.clone())
    .send()
    .await?
    .error_for_status()?;

  return Ok((purchase.order_id, purchase.quantity.unwrap_or(1)));
}

pub struct Billing
{
  m_google: Option<tokio::sync::RwLock<GoogleValidationResources>>
}

impl Billing
{
  pub fn new(google_config: Option<GoogleConfig>) -> result::Result<Billing>
  {
    if let Some(config) = google_config
    {
      let key: serde_json::Value = serde_json::from_reader(
        std::io::BufReader::new(std::fs::File::open(config.credentials_path)?)
      )?;

      use google_cloud_auth::credentials::service_account::AccessSpecifier;

      let access_specifier: AccessSpecifier = AccessSpecifier::from_scopes([
        "https://www.googleapis.com/auth/androidpublisher"
      ]);

      return Ok(Billing {
        m_google: Some(tokio::sync::RwLock::new(GoogleValidationResources {
          package_id: config.package_id,
          credentials:
            google_cloud_auth::credentials::service_account::Builder::new(key)
              .with_access_specifier(access_specifier)
              .build()?,
          http_client: reqwest::Client::new(),
          headers: http::HeaderMap::new()
        }))
      });
    }

    return Ok(Billing {
      m_google: None
    });
  }

  pub async fn validate_purchase(
    &self,
    t: &db::Transaction<'_>,
    user_id: i64,
    sku: &str,
    token: &str
  ) -> result::Result<i64>
  {
    let coins_unit: i32 =
      db::query_opt_p(t, "select coins from shop where id = $1", &[&sku])
        .await?
        .ok_or(error::Error::BadParameter)?
        .get(0);

    let (reason, quantity) = if let Some(ref google_lock) = self.m_google
    {
      let google = &mut google_lock.write().await;
      validate_with_google(google, t, user_id, sku, token).await?
    }
    else
    {
      (String::from("purchase"), 1)
    };

    let coins: i64 = coins_unit as i64 * quantity as i64;

    wallet::coins_transaction(t, user_id, &reason, coins).await?;

    return Ok(coins);
  }
}
