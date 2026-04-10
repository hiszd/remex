use std::sync::Arc;

use remex_core::db::model;
use surrealdb::types::Action;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

use crate::ConnState;

pub async fn monitor_jobs(ctx: Arc<Mutex<crate::Context>>) -> Result<(), crate::Error> {
  tracing::info!("Starting monitoring jobs");
  let mut stream = crate::REMOTE_DB
    .select::<Vec<model::jobs::Job>>("job")
    .live()
    .await?;
  tracing::info!("Monitoring jobs loop starting");
  loop {
    tracing::info!("Acquiring lock");
    loop {
      let ctx_lock = ctx.lock().await;
      if ctx_lock.state.remote_db_connected != ConnState::Connected {
        tracing::info!("Remote database not connected");
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        continue;
      } else {
        tracing::info!("Remote database is now connected!");
        break;
      }
    }
    let ctx_lock = ctx.lock().await;

    tracing::info!("Monitoring jobs");

    let id: surrealdb::types::RecordId =
      surrealdb::types::RecordId::parse_simple(&ctx_lock.session.client_id.clone().unwrap())
        .unwrap();

    tokio::select! {
      notification = stream.next() => {
        match notification.unwrap() {
          Ok(notification) => {
            if !notification.data.assignments.contains(&id) {
              continue;
            }
            match notification.action {
              Action::Create => {
                tracing::debug!("Job created: {:#?}", notification.data);
                println!("Executing job: {}", notification.data.job_name);
              }
              Action::Update => {
                tracing::debug!("Job updated: {:#?}", notification.data);
              }
              Action::Delete => {
                tracing::debug!("Job deleted: {:#?}", notification.data);
                println!("Job completed: {}", notification.data.job_name);
              }
              Action::Killed => {
                tracing::debug!("Job killed: {:#?}", notification.data);
                println!("Job completed: {}", notification.data.job_name);
              }
            }
          }
          Err(err) => {
            tracing::error!("Error: {:#?}", err);
          }
        }
      }
    }
  }
}
