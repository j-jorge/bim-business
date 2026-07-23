// SPDX-License-Identifier: AGPL-3.0-only
mod business;
mod webapi;

use anyhow::{Context, Result};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(argh::FromArgs)]
/// Bim! business server.
#[argh(help_triggers("-h", "--help"))]
struct Arguments {
  /// port to listen to.
  #[argh(option)]
  port: u16,

  /// host of the database.
  #[argh(option, default = "String::from(\"localhost\")")]
  db_host: String,

  /// port of the database.
  #[argh(option)]
  db_port: u16,

  /// name of the database.
  #[argh(option)]
  db_name: String,

  /// user to log in the database.
  #[argh(option)]
  db_user: String,

  /// the file from which to read the secrets.
  #[argh(option)]
  secrets: std::path::PathBuf,

  /// the directory with the assets.
  #[argh(option)]
  assets: std::path::PathBuf,
}

#[derive(serde::Deserialize)]
struct Secrets {
  /// Password of the database user.
  db_password: String,
}

#[tokio::main]
async fn main() -> Result<()> {
  let arguments: Arguments = argh::from_env();

  let secrets: Secrets = serde_json::from_reader(std::io::BufReader::new(
    std::fs::File::open(arguments.secrets.clone())?,
  ))?;

  // Tracing at app level. Use debug level for tower_http in order
  // to have a trace of all requests and their status codes.
  tracing_subscriber::registry()
    .with(
      tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(
        |_| {
          format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
        },
      ),
    )
    .with(tracing_subscriber::fmt::layer().with_ansi(false))
    .init();

  let mut deadpool_config = deadpool_postgres::Config::new();
  deadpool_config.host = Some(arguments.db_host.clone());
  deadpool_config.port = Some(arguments.db_port);
  deadpool_config.dbname = Some(arguments.db_name.clone());
  deadpool_config.user = Some(arguments.db_user.clone());
  deadpool_config.password = Some(secrets.db_password);

  // Keep a pool of connections to the database. I wanted to share the
  // tokio_postgres::Client with the services but it would not pass
  // the borrow checker, and the client is not clonable.
  //
  // Moreover, there is the question of losing the connection to the
  // database, even if it is on the same host. The pool will handle
  // that.
  let pool = deadpool_config
    .create_pool(
      Some(deadpool_postgres::Runtime::Tokio1),
      tokio_postgres::NoTls,
    )
    .context("failed to create Postgres connection pool")?;

  business::schema::migrate_database(&mut pool.get().await?, &arguments.assets)
    .await
    .context("failed to migrate the database.")?;

  let game_servers =
    std::sync::Arc::new(business::game_servers::GameServers::new());
  let session_service = std::sync::Arc::new(business::sessions::Service::new());
  let games = std::sync::Arc::new(business::games::Service::new());

  // Register the web services.
  let router = axum::Router::new()
    .nest(
      "/admin/flat-client-config",
      webapi::admin::flat_client_config::route(pool.clone()),
    )
    .nest(
      "/admin/app-config",
      webapi::admin::app_config::route(pool.clone()),
    )
    .nest("/admin/users", webapi::admin::users::route(pool.clone()))
    .nest(
      "/admin/game-feature-slots",
      webapi::admin::game_feature_slots::route(pool.clone()),
    )
    .nest(
      "/admin/game-features",
      webapi::admin::game_features::route(pool.clone()),
    )
    .nest(
      "/admin/game-servers",
      webapi::admin::game_servers::route(game_servers.clone(), pool.clone()),
    )
    .nest("/admin/leads", webapi::admin::leads::route(pool.clone()))
    .nest("/admin/shop", webapi::admin::shop::route(pool.clone()))
    .nest(
      "/client/config",
      webapi::client::config::route(game_servers.clone(), pool.clone()),
    )
    .nest(
      "/client",
      webapi::client::account::route(session_service.clone(), pool.clone()),
    )
    .nest("/client/game", webapi::client::game::route(pool.clone()))
    .nest(
      "/gs/",
      webapi::gs::games::route(games.clone(), pool.clone()),
    )
    .nest(
      "/gs/",
      webapi::gs::hello::route(game_servers.clone(), pool.clone()),
    )
    .nest("/gs/", webapi::gs::users::route(pool.clone()))
    .layer(tower_http::trace::TraceLayer::new_for_http());

  // And finally, launch the server.
  let address = std::net::SocketAddr::new(
    std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
    arguments.port,
  );

  let server = axum_server::bind(address).serve(router.into_make_service());

  println!("Starting the web services.");

  return server.await.context("error during server run");
}
