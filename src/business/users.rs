// SPDX-License-Identifier: AGPL-3.0-only
use super::*;

#[derive(serde::Serialize)]
pub struct ProfileResponse {
  pub user_id: i64,
  pub nickname: String,
}

pub async fn profile(
  db: &db::Client,
  from_user_id: i64,
  user_ids: &[i64],
) -> result::Result<Vec<ProfileResponse>> {
  let mut query = String::from(
    r"select u.user_id,
             case when u.user_id = $1
                  then u.nickname
                  else coalesce(o.nickname, u.nickname)
             end
      from user_account u
      left join nickname_override o
      on u.user_id = o.user_id
      where u.user_id in (",
  );
  let mut separator = ' ';
  let mut parameters =
    Vec::<&(dyn tokio_postgres::types::ToSql + Sync)>::with_capacity(
      user_ids.len() + 1,
    );

  parameters.push(&from_user_id);

  for (i, id) in user_ids.iter().enumerate() {
    query += &format!("{}${}", separator, i + 2);
    separator = ',';
    parameters.push(id);
  }

  query += ")";

  let rows: Vec<tokio_postgres::Row> =
    db::query_p(db, &query, &parameters).await?;

  let result: Vec<ProfileResponse> = rows
    .iter()
    .map(|r| ProfileResponse {
      user_id: r.get(0),
      nickname: r.get(1),
    })
    .collect();

  return Ok(result);
}

pub async fn set_nickname(
  t: &db::Transaction<'_>,
  user_id: i64,
  nickname: &str,
) -> result::Result<()> {
  db::execute_p(
    t,
    r"update user_account
      set nickname = $1
      where user_id = $2",
    &[&nickname, &user_id],
  )
  .await?;

  return Ok(());
}

pub async fn override_nickname(
  t: &db::Transaction<'_>,
  user_id: i64,
  nickname: &str,
) -> result::Result<()> {
  db::execute_p(
    t,
    r"insert into nickname_override
      values ($1, $2)
      on conflict (user_id) do update set nickname = $2",
    &[&user_id, &nickname],
  )
  .await?;

  return Ok(());
}
