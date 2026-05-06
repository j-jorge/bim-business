// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

// What is called a leader (or a lead) here is an administrator. They
// are allowed to edit anything.

pub async fn run_migration(
  transaction: &deadpool_postgres::Transaction<'_>,
  to_version: i32,
) -> result::Result<()> {
  if to_version == 1 {
    // The leaders are just identified by a random token they will
    // have to pass in the authorization header.
    transaction
      .batch_execute("create table leads (token text unique)")
      .await?;
  }

  return Ok(());
}

pub struct Leaders {
  m_db: deadpool_postgres::Pool,
}

impl Leaders {
  pub fn new(db: deadpool_postgres::Pool) -> Leaders {
    let result = Leaders { m_db: db };

    return result;
  }

  pub async fn validate_token(&self, token: &str) -> result::Result<bool> {
    return Ok(
      self
        .m_db
        .get()
        .await?
        .query(
          "select exists (select token from leads where token = $1)",
          &[&token],
        )
        .await?[0]
        .get(0),
    );
  }

  /// In initialization state there is no leader (the table has just
  /// been created, it's empty). The only allowed action is to create a
  /// leader.
  pub async fn is_in_initialization_state(&self) -> result::Result<bool> {
    let has_any_lead: bool = self
      .m_db
      .get()
      .await?
      .query("select exists (select token from leads)", &[])
      .await?[0]
      .get(0);

    return Ok(!has_any_lead);
  }

  pub async fn create_token(&self) -> result::Result<String> {
    let token: String = token::generate_token(32)?;
    self
      .m_db
      .get()
      .await?
      .execute("insert into leads values ($1)", &[&token])
      .await?;

    return Ok(token);
  }

  pub async fn all_tokens(&self) -> result::Result<Vec<String>> {
    return Ok(
      self
        .m_db
        .get()
        .await?
        .query("select token from leads", &[])
        .await?
        .into_iter()
        .map(|r| r.get(0))
        .collect(),
    );
  }
}
