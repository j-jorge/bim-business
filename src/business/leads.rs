// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

// What is called a leader (or a lead) here is an administrator. They
// are allowed to edit everything.

pub async fn validate_token(
  db: &db::Client,
  token: &str,
) -> result::Result<bool> {
  return db::exists_p(
    db,
    "select token from leads where token = $1",
    &[&token],
  )
  .await;
}

/// In initialization state there is no leader (the table has just
/// been created, it's empty). The only allowed action is to create a
/// leader.
pub async fn is_in_initialization_state(
  db: &db::Client,
) -> result::Result<bool> {
  let has_any_lead: bool = db::exists(db, "select token from leads").await?;

  return Ok(!has_any_lead);
}

pub async fn create_token(db: &db::Client) -> result::Result<String> {
  let token: String = token::generate_token(32)?;
  db::execute_p(db, "insert into leads values ($1)", &[&token]).await?;

  return Ok(token);
}

pub async fn all_tokens(db: &db::Client) -> result::Result<Vec<String>> {
  return db::collect(db, "select token from leads", |r| r.get(0)).await;
}
