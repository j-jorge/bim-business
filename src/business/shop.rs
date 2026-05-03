// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

// The shop lists all products that can be purchased via the store.

pub async fn run_migration(
  transaction: &deadpool_postgres::Transaction<'_>,
  to_version: i32,
) -> result::Result<()> {
  if to_version == 1 {
    // id: should match a product ID from the store.
    // coins: the amount of coins acquired by this purchase.
    transaction
      .batch_execute(
        "create table shop \
         (id text primary key, coins integer)",
      )
      .await?;
  }

  return Ok(());
}

pub struct Shop {
  m_db: deadpool_postgres::Pool,
}

impl Shop {
  pub fn new(db: deadpool_postgres::Pool) -> Shop {
    let result = Shop { m_db: db };

    return result;
  }

  /// Adds products with the given reward in coins, or update the
  /// reward of a product if the ID already exists.
  pub async fn batch_put(
    &self,
    products: &std::collections::HashMap<String, i32>,
  ) -> result::Result<()> {
    if products.is_empty() {
      return Ok(());
    }

    let mut client: deadpool_postgres::Object = self.m_db.get().await?;
    let transaction: deadpool_postgres::Transaction<'_> =
      client.transaction().await?;

    for (id, coins) in products {
      if *coins < 0 {
        tracing::error!("Product coins reward '{}' cannot be negative", &id);
        return Err(error::Error::InvalidParameter);
      }
      transaction
        .execute(
          "insert into shop \
           values ($1, $2) \
           on conflict (id) \
           do update set coins = $2",
          &[&id, &coins],
        )
        .await?;
    }

    return Ok(transaction.commit().await?);
  }

  /// Returns a vector of shop products.
  pub async fn list(
    &self,
  ) -> result::Result<std::collections::HashMap<String, i32>> {
    return Ok(
      self
        .m_db
        .get()
        .await?
        .query("select id, coins from shop", &[])
        .await?
        .into_iter()
        .map(|row| (row.get(0), row.get(1)))
        .collect(),
    );
  }
}
