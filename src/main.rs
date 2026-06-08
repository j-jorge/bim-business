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
}

#[derive(serde::Deserialize)]
struct Secrets {
  /// Password of the database user.
  db_password: String,
}

/// Update the tables to match the state required by the current code.
///
/// Many tools (e.g. Loco or Ruby on Rails) provide a rollback
/// mechanism but I don't see when it becomes useful. Some
/// transformations cannot be rollbacked (e.g. drop table) so it seems
/// that it is necessary to backup the database before any
/// migration. But then, why rollback if we can just restore the
/// backup?
async fn migrate_database(
  mut client: deadpool_postgres::Object,
) -> business::result::Result<()> {
  // We are keeping the current version of the schema into a
  // specific table which will have a single row (or none on
  // creation) with the version number.
  client
    .batch_execute("create table if not exists meta_version (value integer)")
    .await?;

  let version_row: Option<tokio_postgres::Row> = client
    .query_opt("select value from meta_version", &[])
    .await
    .unwrap();
  let mut table_version: i32 = match version_row {
    None => 0,
    Some(r) => r.get(0),
  };
  const CURRENT_VERSION: i32 = 2;

  if table_version != CURRENT_VERSION {
    // Wrap the operations in a transaction such that we can apply
    // them all at once, thus avoiding a partial modification if
    // something fails.
    let t: deadpool_postgres::Transaction<'_> = client.transaction().await?;

    if table_version == 0 {
      println!("Table version is 0, upgrading to 1.");
      table_version += 1;

      // Ensure each service has the tables it needs.
      business::flat_client_config::run_migration(&t, table_version).await?;
      business::leads::run_migration(&t, table_version).await?;
      business::game_features::run_migration(&t, table_version).await?;
      business::game_servers::run_migration(&t, table_version).await?;
      business::shop::run_migration(&t, table_version).await?;
    }

    if table_version == 1 {
      println!("Table version is 1, upgrading to 2.");
      table_version += 1;

      business::game_feature_slots::run_migration(&t, table_version).await?;
    }

    // Update the schema version too, in the same transaction.
    t.execute(
      "insert into meta_version (value) values ($1);",
      &[&table_version],
    )
    .await?;

    t.commit().await?;
  }

  println!("Migration done. Final version is {}.", table_version);

  return Ok(());
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

  migrate_database(pool.get().await?)
    .await
    .context("failed to migrate the database: {}")?;

  let leads = std::sync::Arc::new(business::leads::Leaders::new(pool.clone()));
  let flat_client_config = std::sync::Arc::new(
    business::flat_client_config::Repository::new(pool.clone()),
  );
  let game_feature_slots = std::sync::Arc::new(
    business::game_feature_slots::Repository::new(pool.clone()),
  );
  let game_features =
    std::sync::Arc::new(business::game_features::Repository::new(pool.clone()));
  let game_servers =
    std::sync::Arc::new(business::game_servers::GameServers::new(pool.clone()));
  let shop = std::sync::Arc::new(business::shop::Shop::new(pool));

  // Register the web services.
  let router = axum::Router::new()
    .nest(
      "/admin/flat-client-config",
      webapi::admin::flat_client_config::route(
        leads.clone(),
        flat_client_config.clone(),
      ),
    )
    .nest(
      "/admin/game-feature-slots",
      webapi::admin::game_feature_slots::route(
        leads.clone(),
        game_feature_slots.clone(),
      ),
    )
    .nest(
      "/admin/game-features",
      webapi::admin::game_features::route(leads.clone(), game_features.clone()),
    )
    .nest(
      "/admin/game-servers",
      webapi::admin::game_servers::route(leads.clone(), game_servers.clone()),
    )
    .nest("/admin/leads", webapi::admin::leads::route(leads.clone()))
    .nest(
      "/admin/shop",
      webapi::admin::shop::route(leads, shop.clone()),
    )
    .nest(
      "/client/config",
      webapi::client::config::route(
        flat_client_config,
        game_feature_slots,
        game_features,
        game_servers.clone(),
        shop,
      ),
    )
    .nest("/gs/hello", webapi::gs::hello::route(game_servers))
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
