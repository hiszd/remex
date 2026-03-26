use std::sync::Arc;

use chrono::Utc;
use remex_core::codec;
use tokio::sync::Mutex;

pub async fn jobs_check(
  ctx: Arc<Mutex<crate::Context>>,
  tx: tokio::sync::mpsc::Sender<codec::ClientRequest>,
) {
  let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
  loop {
    interval.tick().await;
    let mut should_request = false;

    {
      let mut ctx_lock = ctx.lock().await;
      if ctx_lock.authenticated {
        if let Some(last_requested) = ctx_lock.jobs_last_requested {
          if last_requested.elapsed().as_secs() >= 30 {
            should_request = true;
          }
        } else {
          should_request = true;
        }

        if should_request {
          ctx_lock.jobs_last_requested = Some(std::time::Instant::now());
        }
      }
    }

    if should_request
      && tx
        .send(codec::ClientRequest::JobsRequest(codec::JobsRequest::All))
        .await
        .is_err()
    {
      break;
    }
  }
}

pub async fn jobs_exec(
  ctx: Arc<Mutex<crate::Context>>,
  tx: tokio::sync::mpsc::Sender<codec::ClientRequest>,
) {
  tracing::info!("Starting jobs executor");
  let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
  loop {
    interval.tick().await;

    let jobs_to_exec: Vec<String> = {
      let ctx_lock = ctx.lock().await;
      if ctx_lock.authenticated {
        ctx_lock.cache.jobs.iter().map(|j| j.job_name.clone()).collect()
      } else {
        vec![]
      }
    };

    for job_name in jobs_to_exec {
      // TODO: Implement job execution when database is re-added
      tracing::info!("Job {} is in TODO state", job_name);

      if tx
        .send(codec::ClientRequest::JobsRequest(codec::JobsRequest::SendExecutions(
          job_name,
          (),
          (),
        )))
        .await
        .is_err()
      {
        return;
      }
    }
  }
}