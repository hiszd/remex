use std::sync::Arc;

use remex_core::db::model;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

use crate::ConnState;

pub async fn job_tasks(ctx: Arc<Mutex<crate::Context>>, _client_id: String) {
  tokio::spawn(monitor_jobs(ctx, _client_id));
}

pub async fn monitor_jobs(
  ctx: Arc<Mutex<crate::Context>>,
  _client_id: String,
) -> Result<(), crate::Error> {
  let mut stream: surrealdb::Stream<Vec<model::jobs::Job>> =
    crate::REMOTE_DB.select("job").live().await?;
  loop {
    let ctx_lock = ctx.lock().await;
    if ctx_lock.state.remote_db_connected != ConnState::Connected {
      continue;
    }

    tokio::select! {
      Some(job) = stream.next() => {
        println!("Job: {:#?}", job);
      }
    }
  }
}
