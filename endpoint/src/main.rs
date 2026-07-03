use clap::Parser;
use surrealdb::engine::local::SurrealKv;

mod async_tasks;
mod db;
mod db_connector;

use actix::Actor;
use async_tasks::jobs::scheduler::SchedulerActor;
use async_tasks::db_heartbeat::HeartbeatActor;

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
  #[error("No Database Connection")]
  NoDatabaseConnection(String),
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

  let (db_handle_tx, db_handle_rx) = tokio::sync::watch::channel(None::<(surrealdb::Surreal<surrealdb::engine::any::Any>, String)>);
  let (monitor_cmd_tx, monitor_cmd_rx) = tokio::sync::mpsc::channel::<async_tasks::jobs::monitor::MonitorCommand>(100);
  let (job_injection_tx, mut job_injection_rx) = tokio::sync::mpsc::channel::<async_tasks::jobs::JobQueueMessage>(1000);
  let (heartbeat_client_id_tx, _heartbeat_client_id_rx) = tokio::sync::mpsc::channel::<String>(10);

  // Start SchedulerActor — migrate from old tokio task to Actix actor
  let scheduler_addr = SchedulerActor::new().start();
  // Bridge: old mpsc channel → actor address (until monitor is migrated too)
  tokio::spawn(async move {
    while let Some(msg) = job_injection_rx.recv().await {
      if scheduler_addr.send(async_tasks::jobs::scheduler::InjectJob(msg)).await.is_err() {
        tracing::error!("Scheduler actor mailbox closed");
        break;
      }
    }
  });

  tokio::spawn(db_connector::run(
    args.db_url,
    args.enrollment_token,
    db_handle_tx,
    monitor_cmd_tx,
    heartbeat_client_id_tx,
  ));

  tokio::spawn(async_tasks::jobs::monitor::run(
    monitor_cmd_rx,
    job_injection_tx,
    db_handle_rx.clone(),
  ));

  tokio::spawn(async_tasks::jobs::sync::execution_sync_loop(db_handle_rx.clone()));

  // Start HeartbeatActor — receives ConnectionReady from DbConnectorActor (once migrated)
  let _heartbeat_addr = HeartbeatActor::new().start();

  loop {
    tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
  }
}
