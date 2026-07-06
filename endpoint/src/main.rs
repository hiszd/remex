use clap::Parser;
use surrealdb::engine::local::SurrealKv;

mod async_tasks;
mod db;
mod db_connector;

use actix::Supervisor;
use async_tasks::ConnectionReady;
use async_tasks::jobs::monitor::MonitorActor;
use async_tasks::jobs::scheduler::SchedulerActor;
use async_tasks::jobs::sync::SyncActor;
use async_tasks::db_heartbeat::HeartbeatActor;
use db_connector::{DbConnectorActor, Subscribe};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
  #[clap(long, env = "REMEX_DB_URL")]
  db_url: String,
  #[clap(long, env = "REMEX_ENROLLMENT_TOKEN")]
  enrollment_token: Option<String>,
  #[clap(short, long, env = "REMEX_DEBUG")]
  debug: bool,
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
  #[error("Shell not found: {0}")]
  ShellNotFound(String),
  #[error("Command timed out")]
  CommandTimeout,
  #[error("Invalid client ID: {0}")]
  InvalidClientId(String),
}

#[actix::main]
async fn main() -> Result<(), Error> {
  let args = Args::parse();
  init_logging(args.debug);
  println!("Running client");

  db::LOCAL_DB
    .connect::<SurrealKv>("endpoint.db")
    .await
    .unwrap();
  db::migrate(&db::LOCAL_DB).await.unwrap();

  // Start SchedulerActor — migrate from old tokio task to Actix actor
  let scheduler_addr = Supervisor::start(|_| SchedulerActor::new());

  // Start HeartbeatActor — receives ConnectionReady from DbConnectorActor (once migrated)
  let heartbeat_addr = Supervisor::start(|_| HeartbeatActor::new());

  // Start SyncActor — pushes unsynced executions to remote every 30s
  let sync_addr = Supervisor::start(|_| SyncActor::new());

  // Start DbConnectorActor — owns the remote connection, broadcasts ConnectionReady
  let db_connector_addr = Supervisor::start(|_| DbConnectorActor::new(args.db_url, args.enrollment_token));

  // Subscribe downstream actors to ConnectionReady broadcasts
  db_connector_addr.do_send(Subscribe(heartbeat_addr.recipient::<ConnectionReady>()));
  db_connector_addr.do_send(Subscribe(sync_addr.recipient::<ConnectionReady>()));

  // Start MonitorActor — LIVE SELECT streams on job/group tables
  let monitor_addr = Supervisor::start(move |_| MonitorActor::new(scheduler_addr.clone()));
  db_connector_addr.do_send(Subscribe(monitor_addr.recipient::<ConnectionReady>()));

  loop {
    tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
  }
}
