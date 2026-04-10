//ENDPOINT

use std::sync::LazyLock;

use clap::Parser;
use gethostname::gethostname;
use remex_core::{
  codec,
  db::DbOperator,
};
use surrealdb::engine::{
  local::{
    Db,
    SurrealKv,
  },
  remote::ws::Client,
};
use tokio::sync::Mutex;

mod async_tasks;
mod db;
mod utils;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
  /// Secret to use for authentication
  #[clap(long, env = "REMEX_SECRET")]
  secret: Option<String>,
  /// Server IP to connect to
  #[clap(long, env = "REMEX_SERVER")]
  server: String,
  /// Server IP to connect to
  #[clap(long, env = "REMEX_PORT", default_value = "4269")]
  port: String,
}

#[derive(Debug, Clone)]
struct Context {
  session: db::endpoint::Session,
  state: State,
  server_secret: Option<String>,
  authenticated: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConnState {
  Initializing,
  Connecting,
  Connected,
  Disconnected,
  Reconnecting,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct State {
  pub server_connected: ConnState,
  pub remote_db_connected: ConnState,
}

static LOCAL_DB: LazyLock<surrealdb::Surreal<Db>> = LazyLock::new(surrealdb::Surreal::init);
static REMOTE_DB: LazyLock<surrealdb::Surreal<Client>> = LazyLock::new(surrealdb::Surreal::init);

#[derive(thiserror::Error, Debug)]
enum Error {
  #[error(transparent)]
  Surreal(#[from] surrealdb::Error),
  #[error(transparent)]
  DbError(#[from] remex_core::db::DbError),
}

#[actix_web::main]
async fn main() -> Result<(), Error> {
  tracing_subscriber::fmt::init();
  tracing::info!("Running client");

  let args = Args::parse();

  LOCAL_DB.connect::<SurrealKv>("endpoint.db").await.unwrap();
  db::migrate(&LOCAL_DB).await.unwrap();

  // Setup the initial context for the application
  let ctx_data = Context {
    session: match LOCAL_DB
      .query("USE NS remex DB endpoint; SELECT * FROM session ORDER BY updated_at DESC LIMIT 1;")
      .await
    {
      Ok(s) => match s.check() {
        Ok(mut s) => match s.take(1)? {
          Some(s) => s,
          None => db::endpoint::Session::create(
            db::endpoint::SessionData {
              client_id: None,
              hardware_hash: Some(machine_uid::get().unwrap()),
              client_name: Some(gethostname().to_string_lossy().to_string()),
              db_addr: None,
              tkn: None,
              secret: None,
            },
            &LOCAL_DB,
          )
          .await?
          .unwrap(),
        },
        Err(e) => panic!("Failed to check session: {}", e),
      },
      Err(e) => {
        tracing::error!("Failed to query session: {}\n Creating a new one instead", e);
        db::endpoint::Session::create(
          db::endpoint::SessionData {
            client_id: None,
            hardware_hash: Some(machine_uid::get().unwrap()),
            client_name: Some(gethostname().to_string_lossy().to_string()),
            db_addr: None,
            tkn: None,
            secret: None,
          },
          &LOCAL_DB,
        )
        .await?
        .unwrap()
      }
    },
    state: State {
      server_connected: ConnState::Initializing,
      remote_db_connected: ConnState::Initializing,
    },
    server_secret: args.secret.clone(),
    authenticated: false,
  };
  let ctx = std::sync::Arc::new(Mutex::new(ctx_data));

  // Create bounded channel for outgoing requests with backpressure
  let (_client_request_tx, client_request_rx) =
    tokio::sync::mpsc::channel::<codec::ClientRequest>(1000);

  // Spawn task to process server messages
  tokio::spawn(async_tasks::server_msg::server_msg_loop(
    ctx.clone(),
    args.secret.clone(),
    args.server.clone(),
    args.port.clone(),
    client_request_rx,
  ));

  // Spawn task to process server messages
  tokio::spawn(async_tasks::jobs::monitor_jobs(ctx.clone()));

  // spawn threads to request new jobs and execute them outside of the reconnection loop
  // so they keep generating messages even when the connection is down.
  // tokio::spawn(async_tasks::jobs::jobs_check(ctx.clone(), client_request_tx.clone()));
  // tokio::spawn(async_tasks::jobs::jobs_exec(ctx.clone(), client_request_tx.clone()));

  loop {
    let mut ctx_lock = ctx.lock().await;
    // if the server is connected and has provided the URL for the remote database, setup the connection and start processing messages
    if (ctx_lock.state.server_connected == ConnState::Connected)
      && ctx_lock.session.db_addr.is_some()
      && !(ctx_lock.state.remote_db_connected == ConnState::Connected)
    {
      ctx_lock.state.server_connected = ConnState::Connecting;
      REMOTE_DB
        .connect::<surrealdb::engine::remote::ws::Ws>(ctx_lock.session.db_addr.clone().unwrap())
        .await
        .unwrap();
      REMOTE_DB.use_ns("remex").use_db("remex").await.unwrap();
      ctx_lock.state.server_connected = ConnState::Connected;
    }
  }
}
