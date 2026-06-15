// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

#[derive(serde::Serialize)]
pub struct AuthenticationResponse {
  session_token: String,
  user_id: i64,
}

struct Internals {
  m_date_of_last_clean_up: std::time::SystemTime,
}

async fn session_validity_duration(
  db: &impl deadpool_postgres::GenericClient,
) -> std::time::Duration {
  return std::time::Duration::from_mins(
    app_config::get_u64(db, "sessions.validity.minutes", 60).await,
  );
}

pub struct Service {
  m_internals: tokio::sync::RwLock<Internals>,
}

impl Service {
  pub fn new() -> Service {
    return Service {
      m_internals: tokio::sync::RwLock::new(Internals {
        m_date_of_last_clean_up: std::time::SystemTime::now(),
      }),
    };
  }

  pub async fn authenticate(
    &self,
    db: &db::Transaction<'_>,
    device_id: String,
  ) -> result::Result<AuthenticationResponse> {
    let now = std::time::SystemTime::now();
    self.maybe_run_session_removal_job(db, &now).await?;

    let rows: Vec<tokio_postgres::Row> = db::query_p(
      db,
      r"select token, user_id
        from sessions
        where device_id = $1
        and expires_at > $2",
      &[&device_id, &now],
    )
    .await?;

    let expires_at = now + session_validity_duration(db).await;

    if !rows.is_empty() {
      let row = &rows[0];
      let session_token: String = row.get(0);
      let user_id: i64 = row.get(1);

      db::execute_p(
        db,
        r"update sessions
          set last_used_at = $1, expires_at = $2
          where token = $3",
        &[&now, &expires_at, &session_token],
      )
      .await?;

      return Ok(AuthenticationResponse {
        session_token,
        user_id,
      });
    }

    let user_id: i64 = self.get_or_create_user_id(db, &device_id).await?;
    let session_token: String = token::generate_token(32)?;

    db::execute_p(
      db,
      r"insert into sessions values ($1, $2, $3, $4, $5, $6)",
      &[
        &session_token,
        &user_id,
        &device_id,
        &now,
        &expires_at,
        &now,
      ],
    )
    .await?;

    return Ok(AuthenticationResponse {
      session_token,
      user_id,
    });
  }

  async fn get_or_create_user_id(
    &self,
    t: &db::Transaction<'_>,
    device_id: &String,
  ) -> result::Result<i64> {
    let existing_user_id: Option<tokio_postgres::Row> = db::query_opt_p(
      t,
      r"select user_id from user_device where device_id = $1",
      &[&device_id],
    )
    .await?;

    if let Some(user_id) = existing_user_id {
      return Ok(user_id.get(0));
    }

    // Starting from here, we create a new user.

    let user_id: i64 = db::query_one(
      t,
      r"insert into user_account values (default, '') returning user_id",
    )
    .await?
    .get(0);

    db::execute_p(
      t,
      r"insert into user_available_game_feature_slots values ($1, 0)",
      &[&user_id],
    )
    .await?;

    db::execute_p(
      t,
      "update user_account set nickname = $1 where user_id = $2",
      &[&format!("user_{user_id}"), &user_id],
    )
    .await?;

    db::execute_p(
      t,
      "insert into user_device values ($1, $2)",
      &[&device_id, &user_id],
    )
    .await?;

    return Ok(user_id);
  }

  async fn maybe_run_session_removal_job(
    &self,
    db: &db::Transaction<'_>,
    now: &std::time::SystemTime,
  ) -> result::Result<()> {
    {
      let internals = self.m_internals.read().await;
      let clean_up_delay = std::time::Duration::from_mins(
        app_config::get_u64(db, "sessions.clean_up_delay.minutes", 60).await,
      );

      if internals.m_date_of_last_clean_up + clean_up_delay < *now {
        return Ok(());
      }
    }

    let mut internals = self.m_internals.write().await;

    db::execute_p(db, "delete from sessions where expires_at <= $1", &[&now])
      .await?;

    internals.m_date_of_last_clean_up = *now;

    return Ok(());
  }
}

pub async fn refresh(
  db: &db::Transaction<'_>,
  token: &str,
) -> result::Result<Option<i64>> {
  let now = std::time::SystemTime::now();
  let expires_at = now + session_validity_duration(db).await;

  let row: Option<tokio_postgres::Row> = db::query_opt_p(
    db,
    r"update sessions
        set expires_at = $3
        where token = $1
        and expires_at > $2
        returning user_id",
    &[&token, &now, &expires_at],
  )
  .await?;

  if let Some(r) = row {
    return Ok(Some(r.get(0)));
  }

  return Ok(None);
}

pub async fn to_user_id(
  db: &db::Client,
  session_token: &str,
) -> result::Result<Option<i64>> {
  let now = std::time::SystemTime::now();

  let existing_user_id: Option<tokio_postgres::Row> = db::query_opt_p(
    db,
    r"select user_id from sessions where token = $1 and expires_at > $2",
    &[&session_token, &now],
  )
  .await?;

  if let Some(user_id) = existing_user_id {
    return Ok(user_id.get(0));
  }

  return Ok(None);
}
