// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

pub struct Wrapper {
  m_db: deadpool_postgres::Pool,
}

pub async fn transaction(
  client: &mut deadpool_postgres::Object,
) -> result::Result<deadpool_postgres::Transaction<'_>> {
  return Ok(client.transaction().await?);
}

impl Wrapper {
  pub fn new(db: deadpool_postgres::Pool) -> Wrapper {
    return Wrapper { m_db: db };
  }

  pub async fn client(&self) -> result::Result<deadpool_postgres::Object> {
    return Ok(self.m_db.get().await?);
  }

  pub async fn execute_p(
    &self,
    statement: &str,
    params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
  ) -> result::Result<u64> {
    return Ok(self.m_db.get().await?.execute(statement, params).await?);
  }

  pub async fn query_p(
    &self,
    statement: &str,
    params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
  ) -> result::Result<Vec<tokio_postgres::Row>> {
    return Ok(self.m_db.get().await?.query(statement, params).await?);
  }

  pub async fn query(
    &self,
    statement: &str,
  ) -> result::Result<Vec<tokio_postgres::Row>> {
    return self.query_p(statement, &[]).await;
  }

  pub async fn query_one_p(
    &self,
    statement: &str,
    params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
  ) -> result::Result<tokio_postgres::Row> {
    return Ok(self.m_db.get().await?.query_one(statement, params).await?);
  }

  pub async fn query_one(
    &self,
    statement: &str,
  ) -> result::Result<tokio_postgres::Row> {
    return self.query_one_p(statement, &[]).await;
  }

  pub async fn collect<B, F, R>(
    &self,
    statement: &str,
    transform: F,
  ) -> result::Result<R>
  where
    F: FnMut(tokio_postgres::Row) -> B,
    R: FromIterator<B>,
  {
    return Ok(
      self
        .query(statement)
        .await?
        .into_iter()
        .map(transform)
        .collect(),
    );
  }
}
