use clap::Parser;
use surrealdb::engine::local::SurrealKv;

mod async_tasks;
mod db;
mod db_connector;
mod utils;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
  #[clap(long, env = "REMEX_SECRET")]
  secret: Option<String>,
  #[clap(long, env = "REMEX_SERVER")]
  server: String,
  #[clap(long, env = "REMEX_PORT", default_value = "4269")]
  port: String,
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

#[tokio::main]
async fn main() -> Result<(), Error> {
  let args = Args::parse();
  init_logging(args.debug);
  println!("Running client");

  db::LOCAL_DB
    .connect::<SurrealKv>("endpoint.db")
    .await
    .unwrap();
  db::migrate(&db::LOCAL_DB).await.unwrap();

  let (db_token_tx, db_token_rx) = tokio::sync::mpsc::channel::<(remex_core::db::BearerGrantResponse, String)>(10);
  let (db_handle_tx, db_handle_rx) = tokio::sync::watch::channel(None::<surrealdb::Surreal<surrealdb::engine::remote::ws::Client>>);
  let (monitor_cmd_tx, monitor_cmd_rx) = tokio::sync::mpsc::channel::<async_tasks::jobs::monitor::MonitorCommand>(100);
  let (job_injection_tx, job_injection_rx) = tokio::sync::mpsc::channel::<async_tasks::jobs::JobQueueMessage>(1000);

  tokio::spawn(db_connector::run(db_token_rx, db_handle_tx));

  tokio::spawn(async_tasks::server_msg::server_msg_loop(
    args.secret,
    args.server,
    args.port,
    db_token_tx,
    monitor_cmd_tx,
  ));

  tokio::spawn(async_tasks::jobs::scheduler::run(job_injection_rx));

  tokio::spawn(async_tasks::jobs::monitor::run(
    monitor_cmd_rx,
    job_injection_tx,
    db_handle_rx.clone(),
  ));

  tokio::spawn(async_tasks::jobs::sync::execution_sync_loop(db_handle_rx));

  loop {
    tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
  }
}
