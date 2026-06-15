// SPDX-License-Identifier: AGPL-3.0-only
use super::result::OrBadParameter;
use super::*;
use std::str::FromStr;

// The game servers can register to this business server with a token
// provided by an admin. They will be available for the clients as
// long as they keep notifying the business server about their
// availability.

#[derive(serde::Serialize)]
pub struct ServerDeclaredInfo {
  pub host: String,
  pub version: u64,
  pub protocol_version: u64,
}

#[derive(serde::Serialize)]
pub struct GameServerInfo {
  pub id: String,
  pub token: String,
  pub description: String,
  pub registration_date: chrono::DateTime<chrono::Utc>,
  pub last_seen: chrono::DateTime<chrono::Utc>,
  #[serde(skip_serializing_if = "Option::is_none")]
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
  m_date_of_last_clean_up: std::time::Instant,
  m_online_servers: Vec<OnlineServerInfo>,
}

async fn clean_up_delay(db: &db::Client) -> std::time::Duration {
  return std::time::Duration::from_mins(
    app_config::get_u64(db, "game_servers.clean_up_delay.minutes", 5).await,
  );
}

impl Internals {
  pub fn online_hosts_for_protocol(
    &self,
    protocol_version: u64,
  ) -> Vec<String> {
    return self
      .m_online_servers
      .iter()
      .filter(|s| s.protocol_version == protocol_version)
      .map(|s| s.host.clone())
      .collect();
  }

  pub fn server_clean_up(&mut self, now: &std::time::Instant) {
    fn still_valid(s: &OnlineServerInfo, now: &std::time::Instant) -> bool {
      if s.removal_date <= *now {
        tracing::info!("Removing server {}", s.id);
        return false;
      }

      return true;
    }

    self.m_online_servers.retain(|s| still_valid(s, now));
  }
}

pub struct GameServers {
  m_internals: tokio::sync::RwLock<Internals>,
}

impl GameServers {
  pub async fn new() -> GameServers {
    return GameServers {
      m_internals: tokio::sync::RwLock::new(Internals {
        m_date_of_last_clean_up: std::time::Instant::now(),
        m_online_servers: vec![],
      }),
    };
  }

  /// Register a new game server, returns its token.
  pub async fn register(
    &self,
    db: &db::Client,
    id: &str,
    description: &str,
  ) -> result::Result<String> {
    if id
      .chars()
      .any(|c| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
    {
      return Err(error::Error::BadParameter);
    }

    let now: std::time::SystemTime = std::time::SystemTime::now();
    let token: String = token::generate_token(32)?;
    db::execute_p(
      db,
      "insert into game_server values ($1, $2, $3, $4, $5)",
      &[&id, &token, &description, &now, &std::time::UNIX_EPOCH],
    )
    .await?;

    return Ok(token);
  }

  pub async fn validate_token(
    &self,
    db: &db::Client,
    token: &str,
  ) -> result::Result<bool> {
    return Ok(
      db::query_one_p(
        db,
        "select exists (select token from game_server where token = $1)",
        &[&token],
      )
      .await?
      .get(0),
    );
  }

  pub async fn hello(
    &self,
    db: &db::Client,
    token: &str,
    host: String,
    version: u64,
    protocol_version: u64,
  ) -> result::Result<std::time::Duration> {
    // Validate the syntax of the host string. It must be ip:port or
    // domain:port.
    let (host_str, port_str) =
      host.rsplit_once(':').ok_or(error::Error::BadParameter)?;

    let _ = u16::from_str(port_str).or_bad_parameter()?;

    // The limit on the length of the host is arbitrary.
    if std::net::IpAddr::from_str(host_str).is_err() && host_str.len() > 255 {
      return Err(error::Error::BadParameter);
    }

    // Retrieve the server id from its token.
    let id: String = db::query_one_p(
      db,
      "select id from game_server where token = $1",
      &[&token],
    )
    .await?
    .get(0);

    // At this point we assume the server is legit.

    let internals = &mut self.m_internals.write().await;

    let removal_delay = clean_up_delay(db).await;
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

  /// List all game servers, with their availability.
  pub async fn all(
    &self,
    db: &db::Client,
  ) -> result::Result<Vec<GameServerInfo>> {
    let rows: Vec<tokio_postgres::Row> = db::query(
      db,
      "select id, token, description, registration_date, last_seen \
         from game_server",
    )
    .await?;

    let internals = &mut self.m_internals.read().await;
    let mut result = Vec::<GameServerInfo>::with_capacity(rows.len());

    for r in rows {
      let id: String = r.get(0);
      let mut info: Option<ServerDeclaredInfo> = None;

      if let Some(i) = internals.m_online_servers.iter().find(|e| e.id == id) {
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

  /// List available game servers supporting the given protocol.
  pub async fn online_hosts_for_protocol(
    &self,
    db: &db::Client,
    protocol_version: u64,
  ) -> result::Result<Vec<String>> {
    let now = std::time::Instant::now();

    {
      let internals = &self.m_internals.read().await;
      let clean_up_delay = clean_up_delay(db).await;

      if internals.m_date_of_last_clean_up + clean_up_delay > now {
        return Ok(internals.online_hosts_for_protocol(protocol_version));
      }
    }

    let internals = &mut self.m_internals.write().await;
    internals.server_clean_up(&now);
    internals.m_date_of_last_clean_up = now;

    return Ok(internals.online_hosts_for_protocol(protocol_version));
  }
}
