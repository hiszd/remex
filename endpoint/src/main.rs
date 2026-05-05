//ENDPOINT

use clap::Parser;
use gethostname::gethostname;
use remex_core::{
  codec,
  db::DbOperator,
};
use surrealdb::engine::local::SurrealKv;
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
  /// Enable debug logging
  #[clap(short, long, env = "REMEX_DEBUG")]
  debug: bool,
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

fn init_logging(debug: bool) {
  if debug {
    tracing_subscriber::fmt()
      .with_max_level(tracing::Level::DEBUG)
      .init();
  } else {
    tracing_subscriber::fmt::init();
  }
}

#[derive(thiserror::Error, Debug)]
enum Error {
  #[error(transparent)]
  Surreal(#[from] surrealdb::Error),
  #[error(transparent)]
  DbError(#[from] remex_core::db::DbError),
  #[error(transparent)]
  StdIo(#[from] std::io::Error),
  #[error("No Database Connection")]
  NoDatabaseConnection(String),
}

#[actix_web::main]
async fn main() -> Result<(), Error> {
  let args = Args::parse();
  init_logging(args.debug);
  println!("Running client");

  db::LOCAL_DB
    .connect::<SurrealKv>("endpoint.db")
    .await
    .unwrap();
  db::migrate(&db::LOCAL_DB).await.unwrap();

  // Setup the initial context for the application
  let ctx_data = Context {
    session: match db::get_local_endpoint()
      .await?
      .query("SELECT * FROM session ORDER BY updated_at DESC LIMIT 1;")
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
            &db::get_local_endpoint().await?,
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
          &db::get_local_endpoint().await?,
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

  let (job_injection_tx, job_injection_rx) =
    tokio::sync::mpsc::channel::<async_tasks::jobs::JobQueueMessage>(1000);

  tracing::info!("Spawning jobs scheduler loop");
  tokio::spawn(async_tasks::jobs::job_scheduler_loop(job_injection_rx));

  tracing::info!("Spawning jobs monitor task");
  tokio::spawn(async_tasks::jobs::monitor_jobs(ctx.clone(), job_injection_tx.clone()));

  // spawn threads to request new jobs and execute them outside of the reconnection loop
  // so they keep generating messages even when the connection is down.
  // tokio::spawn(async_tasks::jobs::jobs_check(ctx.clone(), client_request_tx.clone()));
  // tokio::spawn(async_tasks::jobs::jobs_exec(ctx.clone(), client_request_tx.clone()));

  loop {
    let mut ctx_lock1 = ctx.lock().await;
    // if the server is connected and has provided the URL for the remote database, setup the connection and start processing messages
    if (ctx_lock1.state.server_connected == ConnState::Connected)
      && ctx_lock1.session.db_addr.is_some()
      && !(ctx_lock1.state.remote_db_connected == ConnState::Connected)
    {
      tracing::info!("Connecting to remote database");
      ctx_lock1.state.remote_db_connected = ConnState::Connecting;
      let db_url = ctx_lock1.session.db_addr.clone().unwrap();
      let token = ctx_lock1.session.tkn.clone().unwrap();
      drop(ctx_lock1);
      db::REMOTE_DB
        .connect::<surrealdb::engine::remote::ws::Ws>(db_url)
        .await
        .unwrap();
      tracing::info!("Authenticating to remote database");
      db::REMOTE_DB
        .signin(surrealdb::opt::auth::Record {
          namespace: "remex".into(),
          database: "remex".into(),
          access: "endpoint".into(),
          params: remex_core::db::BearerToken {
            key: token.grant.key.clone(),
          },
        })
        .await?;
      db::REMOTE_DB.use_ns("remex").use_db("remex").await.unwrap();
      println!("Connected to remote database");
      let mut ctx_lock2 = ctx.lock().await;
      ctx_lock2.state.remote_db_connected = ConnState::Connected;
      drop(ctx_lock2);
    }
  }
}
