// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

// The shop lists all products that can be purchased via the store.

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Product {
  pub id: String,
  pub coins: i32,
}

/// Adds products with the given reward in coins, or update the
/// reward of a product if the ID already exists.
pub async fn batch_put(
  t: &db::Transaction<'_>,
  products: &Vec<Product>,
) -> result::Result<()> {
  if products.is_empty() {
    return Ok(());
  }

  for p in products {
    if p.coins < 0 {
      tracing::error!("Product coins reward '{}' cannot be negative", &p.id);
      return Err(error::Error::BadParameter);
    }
    db::execute_p(
      t,
      "insert into shop \
                 values ($1, $2) \
                 on conflict (id) \
                 do update set coins = $2",
      &[&p.id, &p.coins],
    )
    .await?;
  }

  return Ok(());
}

/// Returns a vector of shop products.
pub async fn list(db: &db::Client) -> result::Result<Vec<Product>> {
  return db::collect(db, "select id, coins from shop", |row| Product {
    id: row.get(0),
    coins: row.get(1),
  })
  .await;
}
