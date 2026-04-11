use std::sync::Arc;

use remex_core::db::model;
use surrealdb::types::Action;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

use crate::ConnState;

pub async fn create_job() {
  let b: Option<model::jobs::Job> = crate::REMOTE_DB
    .query("UPSERT job CONTENT $data;")
    .bind(("data", model::jobs::JobData {
      job_name: "test".to_string(),
      job_shell: "bash".to_string(),
      job_command: "echo 'hello world!'".to_string(),
      job_type: model::jobs::JobType::Instant,
      job_status: model::jobs::JobStatus::Pending,
    }))
    .await
    .unwrap()
    .check()
    .unwrap()
    .take(0)
    .unwrap();
  tracing::info!("Job created: {:?}", b);
}

pub async fn monitor_jobs(ctx: Arc<Mutex<crate::Context>>) -> Result<(), crate::Error> {
  tracing::info!("Starting monitoring jobs");
  loop {
    let ctx_lock = ctx.lock().await;
    let conn_state = ctx_lock.state.remote_db_connected.clone();
    drop(ctx_lock);
    if conn_state != ConnState::Connected {
      tracing::info!("Remote database not connected");
      tokio::time::sleep(std::time::Duration::from_secs(2)).await;
      continue;
    } else {
      tracing::info!("Remote database is now connected!");
      break;
    }
  }
  // tokio::spawn(create_job());
  let mut stream = crate::REMOTE_DB
    .select::<Vec<model::jobs::Job>>("job")
    .live()
    .await?;
  tracing::info!("Acquiring lock");
  let ctx_lock = ctx.lock().await;
  let id: surrealdb::types::RecordId =
    surrealdb::types::RecordId::parse_simple(&ctx_lock.session.client_id.clone().unwrap()).unwrap();
  drop(ctx_lock);

  tracing::info!("Monitoring jobs loop starting");
  loop {
    tracing::info!("Monitoring jobs");

    tokio::select! {
      notification = stream.next() => {
        tracing::info!("Job notification received");
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
