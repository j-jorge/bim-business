// SPDX-License-Identifier: AGPL-3.0-only
use super::result::OrBadParameter;
use super::*;
use std::str::FromStr;

// The game servers can register to this business server with a token
// provided by an admin. They will be available for the clients as
// long as they keep notifying the business server about their
// availability.

pub async fn run_migration(
  transaction: &deadpool_postgres::Transaction<'_>,
  to_version: i32,
) -> result::Result<()> {
  if to_version == 1 {
    transaction
      .batch_execute(
        "create table game_server \
         (id text primary key, \
         token text unique, \
         description text, \
         registration_date timestamp, \
         last_seen timestamp)",
      )
      .await?;
  }

  return Ok(());
}

pub struct ServerDeclaredInfo {
  pub host: String,
  pub version: u64,
  pub protocol_version: u64,
}

pub struct GameServerInfo {
  pub id: String,
  pub token: String,
  pub description: String,
  pub registration_date: chrono::DateTime<chrono::Utc>,
  pub last_seen: chrono::DateTime<chrono::Utc>,
  pub info: Option<ServerDeclaredInfo>,
}

#[derive(Debug)]
struct OnlineServerInfo {
  pub id: String,
  pub host: String,
  pub version: u64,
  pub protocol_version: u64,
  pub removal_date: std::time::Instant,
}

struct Internals {
  m_clean_up_delay: std::time::Duration,
  m_date_for_next_clean_up: std::time::Instant,
  m_online_servers: Vec<OnlineServerInfo>,
}

impl Internals {
  pub fn remove_dead_servers(&mut self) {
    let now = std::time::Instant::now();

    if now < self.m_date_for_next_clean_up {
      return;
    }

    fn still_valid(s: &OnlineServerInfo, now: &std::time::Instant) -> bool {
      if s.removal_date <= *now {
        tracing::info!("Removing server {}", s.id);
        return false;
      }

      return true;
    }

    self.m_online_servers.retain(|s| still_valid(s, &now));

    self.m_date_for_next_clean_up = now + self.m_clean_up_delay;
  }
}

pub struct GameServers {
  m_db: db::Wrapper,
  m_internals: std::sync::Mutex<Internals>,
}

impl GameServers {
  pub fn new(db: deadpool_postgres::Pool) -> GameServers {
    let default_clean_up_delay = std::time::Duration::from_mins(5);

    return GameServers {
      m_db: db::Wrapper::new(db),
      m_internals: std::sync::Mutex::new(Internals {
        m_clean_up_delay: default_clean_up_delay,
        m_date_for_next_clean_up: std::time::Instant::now()
          + default_clean_up_delay,
        m_online_servers: vec![],
      }),
    };
  }

  pub fn set_clean_up_delay(&self, delay: std::time::Duration) {
    if let Ok(ref mut internals) = self.m_internals.lock() {
      internals.m_date_for_next_clean_up =
        internals.m_date_for_next_clean_up - internals.m_clean_up_delay + delay;
      internals.m_clean_up_delay = delay;
    }
  }

  /// Register a new game server, returns its token.
  pub async fn register(
    &self,
    id: &str,
    description: &str,
  ) -> result::Result<String> {
    if let Ok(ref mut internals) = self.m_internals.lock() {
      internals.remove_dead_servers();
    }

    if id
      .chars()
      .any(|c| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
    {
      return Err(error::Error::InvalidParameter);
    }

    let now: std::time::SystemTime = std::time::SystemTime::now();
    let token: String = token::generate_token(32)?;
    self
      .m_db
      .execute_p(
        "insert into game_server values ($1, $2, $3, $4, $5)",
        &[&id, &token, &description, &now, &std::time::UNIX_EPOCH],
      )
      .await?;

    return Ok(token);
  }

  pub async fn keep_alive(
    &self,
    token: &str,
    host: String,
    version: u64,
    protocol_version: u64,
  ) -> result::Result<std::time::Duration> {
    // Validate the syntax of the host string. It must be ip:port or
    // domain:port.
    let (host_str, port_str) = host
      .rsplit_once(':')
      .ok_or(error::Error::InvalidParameter)?;

    let _ = u16::from_str(port_str).or_bad_parameter()?;

    // The limit on the length of the host is arbitrary.
    if std::net::IpAddr::from_str(host_str).is_err() && host_str.len() > 255 {
      return Err(error::Error::InvalidParameter);
    }

    // Retrieve the server id from its token.
    let id: String = self
      .m_db
      .query_one_p("select id from game_server where token = $1", &[&token])
      .await?
      .get(0);

    // At this point we assume the server to be legit.

    // TODO: report the error if lock() fails.
    if let Ok(ref mut internals) = self.m_internals.lock() {
      internals.remove_dead_servers();

      let removal_delay = internals.m_clean_up_delay;
      let removal_date = std::time::Instant::now() + removal_delay;

      if let Some(ref mut info) =
        internals.m_online_servers.iter_mut().find(|e| e.id == id)
      {
        info.host = host;
        info.version = version;
        info.protocol_version = protocol_version;
        info.removal_date = removal_date;
      } else {
        internals.m_online_servers.push(OnlineServerInfo {
          id,
          host,
          version,
          protocol_version,
          removal_date,
        });
      }

      return Ok(removal_delay / 2);
    }

    // TODO: return an error.
    return Ok(std::time::Duration::from_mins(1));
  }

  /// List all game servers, with their availability.
  pub async fn all(&self) -> result::Result<Vec<GameServerInfo>> {
    let rows: Vec<tokio_postgres::Row> = self
      .m_db
      .query(
        "select id, token, description, registration_date, last_seen \
         from game_server",
      )
      .await?;

    // TODO: report the error if lock() fails.
    if let Ok(ref mut internals) = self.m_internals.lock() {
      internals.remove_dead_servers();

      let mut result = Vec::<GameServerInfo>::with_capacity(rows.len());

      for r in rows {
        let id: String = r.get(0);
        let mut info: Option<ServerDeclaredInfo> = None;

        if let Some(i) = internals.m_online_servers.iter().find(|e| e.id == id)
        {
          info = Some(ServerDeclaredInfo {
            host: i.host.clone(),
            version: i.version,
            protocol_version: i.protocol_version,
          });
        }

        let registration_date: std::time::SystemTime = r.get(3);
        let last_seen: std::time::SystemTime = r.get(4);

        result.push(GameServerInfo {
          id,
          token: r.get(1),
          description: r.get(2),
          registration_date: registration_date.into(),
          last_seen: last_seen.into(),
          info,
        });
      }
      return Ok(result);
    }

    // TODO: return error.
    return Ok(vec![]);
  }
}
