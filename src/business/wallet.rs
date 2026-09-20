// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

#[derive(
  Debug,
  tokio_postgres::types::FromSql,
  tokio_postgres::types::ToSql,
  serde::Serialize,
)]
#[postgres(name = "transaction_origin", rename_all = "snake_case")]
enum TransactionOrigin
{
  Admin,
  App
}

pub async fn coins_transaction(
  db: &db::Transaction<'_>,
  user_id: i64,
  reason: &str,
  amount: i64
) -> result::Result<()>
{
  return internal_coins_transaction(
    db,
    user_id,
    TransactionOrigin::App,
    reason,
    amount
  )
  .await;
}

pub async fn admin_coins_transaction(
  db: &db::Transaction<'_>,
  user_id: i64,
  reason: &str,
  amount: i64
) -> result::Result<()>
{
  return internal_coins_transaction(
    db,
    user_id,
    TransactionOrigin::Admin,
    reason,
    amount
  )
  .await;
}

async fn internal_coins_transaction(
  db: &db::Transaction<'_>,
  user_id: i64,
  origin: TransactionOrigin,
  reason: &str,
  amount: i64
) -> result::Result<()>
{
  let initial_balance_row: Option<tokio_postgres::Row> = db
    .query_opt(
      r"select coins from user_wallet where user_id = $1 for update",
      &[&user_id]
    )
    .await?;

  let initial_balance: i64 = if let Some(row) = initial_balance_row
  {
    row.get(0)
  }
  else
  {
    0
  };

  if (amount < 0) && (-amount > initial_balance)
  {
    tracing::warn!(
      r"Can't process transaction, amount is {}, balance is {}.",
      amount,
      initial_balance
    );
    return Err(error::Error::Unprocessable);
  }

  db.execute(
    r"insert into user_wallet
          values ($1, $2)
          on conflict (user_id) do update set coins = $2",
    &[&user_id, &(initial_balance + amount)]
  )
  .await?;

  db.execute(
    r"insert into currency_transaction values ($1, $2, $3, $4, $5, $6)",
    &[
      &user_id,
      &time::OffsetDateTime::now_utc(),
      &origin,
      &reason,
      &initial_balance,
      &amount
    ]
  )
  .await?;

  return Ok(());
}

pub async fn coins_balance(db: &db::Client, user_id: i64)
-> result::Result<i64>
{
  let opt: Option<tokio_postgres::Row> = db::query_opt_p(
    db,
    r"select coins from user_wallet where user_id = $1",
    &[&user_id]
  )
  .await?;

  if let Some(row) = opt
  {
    return Ok(row.get(0));
  }

  return Ok(0);
}

#[derive(serde::Serialize)]
pub struct HistoryEntry
{
  #[serde(with = "time::serde::rfc3339")]
  date: time::OffsetDateTime,
  origin: TransactionOrigin,
  reason: String,
  initial_balance: i64,
  amount: i64
}

pub async fn history(
  db: &db::Client,
  user_id: i64
) -> result::Result<Vec<HistoryEntry>>
{
  return db::collect_p(
    db,
    r"select date,
               origin,
               reason,
               initial_balance,
               amount
        from currency_transaction
        where user_id = $1
        order by date desc
        limit 100",
    &[&user_id],
    |r| HistoryEntry {
      date: r.get::<usize, time::OffsetDateTime>(0),
      origin: r.get(1),
      reason: r.get(2),
      initial_balance: r.get(3),
      amount: r.get(4)
    }
  )
  .await;
}
