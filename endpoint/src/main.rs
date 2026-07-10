use std::sync::Arc;

use clap::Parser;
use surrealdb::engine::local::SurrealKv;

mod async_tasks;
mod db;

use actix::Supervisor;
use async_tasks::{
  jobs::{
    scheduler::SchedulerActor,
    RealJobExecutor,
  },
  local_db::LocalDbActor,
  remote_db::RemoteDbActor,
  SetRemoteDbAddr,
};

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

  let hardware_hash = machine_uid::get().unwrap_or_default();

  // Start LocalDbActor — owns local SurrealKV, session, execution cache
  let local_db_addr = Supervisor::start(|_| LocalDbActor::new());

  // Start SchedulerActor — job queue, spawns execute_job tasks, sends RecordExecution to LocalDbActor
  let local_db_for_scheduler = local_db_addr.clone();
  let scheduler_addr = Supervisor::start(move |_| {
    SchedulerActor::new(
      Arc::new(RealJobExecutor),
      local_db_for_scheduler.recipient(),
    )
  });

  // Start RemoteDbActor — owns remote connection, auth, heartbeat, execution push, LIVE SELECT
  let local_db_for_remote = local_db_addr.clone();
  let remote_db_addr = Supervisor::start(move |_| {
    RemoteDbActor::new(
      args.db_url.clone(),
      args.enrollment_token.clone(),
      hardware_hash.clone(),
      local_db_for_remote,
      scheduler_addr.clone(),
    )
  });

  // Wire up RemoteDbActor address to LocalDbActor (for execution sync)
  local_db_addr.do_send(SetRemoteDbAddr(remote_db_addr.clone()));

  loop {
    tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
  }
}
