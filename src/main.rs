// SPDX-License-Identifier: AGPL-3.0-only
mod business;
mod webapi;

use anyhow::{Context, Result};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

  if table_version == 0 {
    println!("Table version is {}, upgrading.", table_version);
    table_version += 1;

    // Wrap the operations in a transaction such that we can apply
    // them all at once, thus avoiding a partial modification if
    // something fails.
    let t: deadpool_postgres::Transaction<'_> = client.transaction().await?;

    // Ensure each service has the tables it needs.
    business::flat_client_config::run_migration(&t, table_version).await?;
    business::leads::run_migration(&t, table_version).await?;
    business::game_features::run_migration(&t, table_version).await?;
    business::game_servers::run_migration(&t, table_version).await?;
    business::shop::run_migration(&t, table_version).await?;

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
    .with(tracing_subscriber::fmt::layer())
    .init();

  let mut deadpool_config = deadpool_postgres::Config::new();
  deadpool_config.host = Some(String::from("localhost"));
  deadpool_config.user = Some(String::from("postgres"));
  deadpool_config.dbname = Some(String::from("postgres"));
  deadpool_config.password = Some(String::from("postgres"));

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

  migrate_database(pool.get().await.unwrap())
    .await
    .context("failed to migrate the database: {}")?;

  // I wish I could avoid ARC here as it models stuff floating around
  // in memory until it becomes unreferenced. By definition I can't
  // control when it will be destroyed nor the order of the
  // destruction :( Ideally I would have wanted to instantiate the
  // services here and have them destroyed at the end of main.
  //
  // Unfortunately I could not find any solution to pass the business
  // services to the web services otherwise, so there we go.
  let leads: std::sync::Arc<business::leads::Leaders> =
    std::sync::Arc::new(business::leads::Leaders::new(pool.clone()));
  let flat_client_config: std::sync::Arc<
    business::flat_client_config::FlatClientConfig,
  > = std::sync::Arc::new(business::flat_client_config::FlatClientConfig::new(
    pool.clone(),
  ));
  let game_features: std::sync::Arc<business::game_features::GameFeatures> =
    std::sync::Arc::new(business::game_features::GameFeatures::new(
      pool.clone(),
    ));
  let game_servers: std::sync::Arc<business::game_servers::GameServers> =
    std::sync::Arc::new(business::game_servers::GameServers::new(pool.clone()));
  let shop: std::sync::Arc<business::shop::Shop> =
    std::sync::Arc::new(business::shop::Shop::new(pool));

  // The certificates, to handle HTTPS. There will be no support for HTTP.
  let certificates_dir = std::path::PathBuf::from("certificates");

  let certificates = axum_server::tls_rustls::RustlsConfig::from_pem_file(
    certificates_dir.join("localhost.crt"),
    certificates_dir.join("localhost.key"),
  )
  .await
  .context("failed to init RustlsConfig")?;

  // Register the web services.
  let router = axum::Router::new()
    .nest(
      "/flat-client-config",
      webapi::flat_client_config::route(leads.clone(), flat_client_config),
    )
    .nest(
      "/game-features",
      webapi::game_features::route(leads.clone(), game_features),
    )
    .nest(
      "/game-servers",
      webapi::game_servers::route(leads.clone(), game_servers),
    )
    .nest("/leads", webapi::leads::route(leads.clone()))
    .nest("/shop", webapi::shop::route(leads, shop))
    .layer(tower_http::trace::TraceLayer::new_for_http());

  // And finally, launch the server.
  let address: std::net::SocketAddr = "127.0.0.1:3000".parse().unwrap();
  let service_future = axum_server::bind_rustls(address, certificates)
    .serve(router.into_make_service());

  println!("Starting the web services.");

  service_future.await.context("error during server run")?;

  return Ok(());
}
