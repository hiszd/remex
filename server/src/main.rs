use std::sync::{
  Arc,
  LazyLock,
};

use actix::Actor;
use clap::Parser;
use remex_core::{
  actors::server::RemexServer,
  utils::generate_secret,
};
use surrealdb::engine::any::Any;
use tokio::sync::Mutex;

mod secret;

const ADDRESS: &str = "127.0.0.1:4269";

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
  /// Enable debug logging
  #[clap(short, long, env = "REMEX_DEBUG")]
  debug: bool,
}

fn get_or_generate_secret() -> String {
  match secret::get_secret("server") {
    Ok(Some(secret_val)) => secret_val,
    Err(e) => {
      tracing::error!("Failed to get secret: {}", e);
      let secret_val = generate_secret(true);
      secret::save_secret("server", secret_val.clone()).expect("Failed to save secret");
      secret_val
    }
    _ => {
      let secret_val = generate_secret(true);
      secret::save_secret("server", secret_val.clone()).expect("Failed to save secret");
      secret_val
    }
  }
}

static REMOTE_DB: LazyLock<surrealdb::Surreal<Any>> = LazyLock::new(surrealdb::Surreal::init);

fn init_logging(debug: bool) {
  if debug {
    tracing_subscriber::fmt()
      .with_max_level(tracing::Level::DEBUG)
      .init();
  } else {
    tracing_subscriber::fmt::init();
  }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
  let args = Args::parse();
  init_logging(args.debug);

  tokio::task::LocalSet::new()
    .run_until(async {
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
          tracing::info!("Migrated SurrealDB");
          REMOTE_DB.query("USE NS remex DB remex; CREATE user SET email = 'hiszd1@gmail.com', username = 'Battl3Ax3', password = 'H@ck3r345';").await.unwrap().check().unwrap();
          tracing::info!("Seed user created");
        }
        Err(e) => {
          tracing::error!("Failed to migrate SurrealDB: {}", e);
        }
      }

      tokio::select! {
        _ = tokio::signal::ctrl_c() => {
          println!("Ctrl-C received, shutting down gracefully...");
        }
      }
      Ok::<_, anyhow::Error>(())
    })
    .await
}
