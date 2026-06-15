// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

pub type Client = deadpool_postgres::Client;
pub type Transaction<'a> = deadpool_postgres::Transaction<'a>;

pub async fn execute_p(
  db: &impl deadpool_postgres::GenericClient,
  statement: &str,
  params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
) -> result::Result<u64> {
  match db.execute(statement, params).await {
    Ok(r) => Ok(r),
    Err(e) => {
      tracing::error!(statement = statement, ?e, "execute_p() failed.");
      return Err(e.into());
    }
  }
}

pub async fn query_p(
  db: &impl deadpool_postgres::GenericClient,
  statement: &str,
  params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
) -> result::Result<Vec<tokio_postgres::Row>> {
  match db.query(statement, params).await {
    Ok(r) => Ok(r),
    Err(e) => {
      tracing::error!(statement = statement, ?e, "query_p() failed.");
      return Err(e.into());
    }
  }
}

pub async fn query(
  db: &impl deadpool_postgres::GenericClient,
  statement: &str,
) -> result::Result<Vec<tokio_postgres::Row>> {
  return query_p(db, statement, &[]).await;
}

pub async fn query_one_p(
  db: &impl deadpool_postgres::GenericClient,
  statement: &str,
  params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
) -> result::Result<tokio_postgres::Row> {
  match db.query_one(statement, params).await {
    Ok(r) => Ok(r),
    Err(e) => {
      tracing::error!(statement = statement, ?e, "query_one_p() failed.");
      return Err(e.into());
    }
  }
}

pub async fn query_one(
  db: &impl deadpool_postgres::GenericClient,
  statement: &str,
) -> result::Result<tokio_postgres::Row> {
  return query_one_p(db, statement, &[]).await;
}

pub async fn query_opt_p(
  db: &impl deadpool_postgres::GenericClient,
  statement: &str,
  params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
) -> result::Result<Option<tokio_postgres::Row>> {
  match db.query_opt(statement, params).await {
    Ok(r) => Ok(r),
    Err(e) => {
      tracing::error!(statement = statement, ?e, "execute_p() failed.");
      return Err(e.into());
    }
  }
}

pub async fn exists_p(
  db: &impl deadpool_postgres::GenericClient,
  statement: &str,
  params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
) -> result::Result<bool> {
  return Ok(
    query_one_p(db, &format!("select exists ({statement})"), params)
      .await?
      .get(0),
  );
}

pub async fn exists(
  db: &impl deadpool_postgres::GenericClient,
  statement: &str,
) -> result::Result<bool> {
  return exists_p(db, statement, &[]).await;
}

pub async fn collect<B, F, R>(
  db: &impl deadpool_postgres::GenericClient,
  statement: &str,
  transform: F,
) -> result::Result<R>
where
  F: FnMut(tokio_postgres::Row) -> B,
  R: FromIterator<B>,
{
  return collect_p(db, statement, &[], transform).await;
}

pub async fn collect_p<B, F, R>(
  db: &impl deadpool_postgres::GenericClient,
  statement: &str,
  params: &[&(dyn tokio_postgres::types::ToSql + Sync)],
  transform: F,
) -> result::Result<R>
where
  F: FnMut(tokio_postgres::Row) -> B,
  R: FromIterator<B>,
{
  return Ok(
    query_p(db, statement, params)
      .await?
      .into_iter()
      .map(transform)
      .collect(),
  );
}
