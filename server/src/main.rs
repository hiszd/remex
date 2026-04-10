use std::sync::{
  Arc,
  LazyLock,
};

use actix::Actor;
use remex_core::{
  actors::server::RemexServer,
  utils::generate_secret,
};
use surrealdb::engine::any::Any;
use tokio::sync::Mutex;

mod secret;

const ADDRESS: &str = "127.0.0.1:4269";

fn get_or_generate_secret() -> String {
  match secret::get_secret("server") {
    Ok(Some(secret_val)) => {
      println!("Using existing secret from file");
      secret_val
    }
    Err(e) => {
      tracing::error!("Failed to get secret: {}", e);
      let secret_val = generate_secret(true);
      secret::save_secret("server", secret_val.clone()).expect("Failed to save secret");
      secret_val
    }
    _ => {
      println!("No secret found, generating new secret");
      let secret_val = generate_secret(true);
      secret::save_secret("server", secret_val.clone()).expect("Failed to save secret");
      secret_val
    }
  }
}

static REMOTE_DB: LazyLock<surrealdb::Surreal<Any>> = LazyLock::new(surrealdb::Surreal::init);

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
  tracing_subscriber::fmt::init();

  let secret_string = get_or_generate_secret();
  println!("Full secret (for copying to endpoint): {}", secret_string);

  let endpoint = std::env::var("DB_ENDPOINT").unwrap_or_else(|_| "mem://".to_owned());
  REMOTE_DB.connect(endpoint).await?;
  REMOTE_DB
    .signin(surrealdb::opt::auth::Root {
      username: "root".to_string(),
      password: std::env::var("DB_PASSWORD").unwrap_or_else(|_| "remex".to_owned()),
    })
    .await?;

  tracing::info!("Connected to SurrealDB");
  match remex_core::db::migrate(&REMOTE_DB).await {
    Ok(()) => {
      tracing::info!("SurrealDB migrated");
    }
    Err(e) => {
      tracing::error!("Failed to migrate SurrealDB: {}", e);
    }
  }

  let client_sessions = Arc::new(Mutex::new(std::collections::HashMap::new()));

  let server = RemexServer {
    sessions: remex_core::sessionmap::SessionMap::default(),
    migrated: false,
    secret: Some(secret_string.clone()),
    client_sessions,
    db: Some(REMOTE_DB.clone()),
  }
  .start();

  let tcp_fut = remex_core::actors::session::tcp_server(
    ADDRESS,
    &secret_string,
    server,
    Some(REMOTE_DB.clone()),
  );

  tokio::select! {
    _ = tokio::signal::ctrl_c() => {
      println!("Ctrl-C received, shutting down gracefully...");
    }
    _ = tcp_fut => {
      println!("TCP server exited unexpectedly.");
    }
  }
  Ok(())
}
