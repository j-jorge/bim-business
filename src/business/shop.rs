// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

// The shop lists all products that can be purchased via the store.

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Product {
  pub id: String,
  pub coins: i32,
}

pub struct Shop {
  m_db: db::Wrapper,
}

impl Shop {
  pub fn new(db: deadpool_postgres::Pool) -> Shop {
    return Shop {
      m_db: db::Wrapper::new(db),
    };
  }

  /// Adds products with the given reward in coins, or update the
  /// reward of a product if the ID already exists.
  pub async fn batch_put(&self, products: &Vec<Product>) -> result::Result<()> {
    if products.is_empty() {
      return Ok(());
    }

    let mut client: deadpool_postgres::Object = self.m_db.client().await?;
    let transaction: deadpool_postgres::Transaction<'_> =
      db::transaction(&mut client).await?;

    for p in products {
      if p.coins < 0 {
        tracing::error!("Product coins reward '{}' cannot be negative", &p.id);
        return Err(error::Error::BadParameter);
      }
      transaction
        .execute(
          "insert into shop \
           values ($1, $2) \
           on conflict (id) \
           do update set coins = $2",
          &[&p.id, &p.coins],
        )
        .await?;
    }

    return Ok(transaction.commit().await?);
  }

  /// Returns a vector of shop products.
  pub async fn list(&self) -> result::Result<Vec<Product>> {
    return self
      .m_db
      .collect("select id, coins from shop", |row| Product {
        id: row.get(0),
        coins: row.get(1),
      })
      .await;
  }
}
