use std::sync::Arc;

use remex_core::db::model;
use surrealdb::types::Action;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

use crate::ConnState;

pub async fn monitor_jobs(ctx: Arc<Mutex<crate::Context>>) -> Result<(), crate::Error> {
  let mut stream: surrealdb::Stream<Vec<model::jobs::Job>> =
    crate::REMOTE_DB.select("job").live().await?;
  loop {
    let ctx_lock = ctx.lock().await;
    if ctx_lock.state.remote_db_connected != ConnState::Connected {
      continue;
    }

    let id: surrealdb::types::RecordId =
      surrealdb::types::RecordId::parse_simple(&ctx_lock.session.client_id.clone().unwrap())
        .unwrap();

    tokio::select! {
      Some(notification) = stream.next() => {
        match notification {
          Ok(notification) => {
            if !notification.data.assignments.contains(&id) {
              continue;
            }
            match notification.action {
              Action::Create => {
                tracing::info!("Job created: {:#?}", notification.data);
              }
              Action::Update => {
                tracing::info!("Job updated: {:#?}", notification.data);
              }
              Action::Delete => {
                tracing::info!("Job deleted: {:#?}", notification.data);
              }
              Action::Killed => {
                tracing::info!("Job killed: {:#?}", notification.data);
              }
            }
          }
          Err(err) => {
            println!("Error: {:#?}", err);
          }
        }
      }
    }
  }
}
