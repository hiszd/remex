use std::sync::Arc;

use remex_core::codec;
use tokio::sync::Mutex;

pub async fn jobs_check(
  ctx: Arc<Mutex<crate::Context>>,
  tx: tokio::sync::mpsc::Sender<codec::ClientRequest>,
) {
  // spawn a new thread that will monitor the ctx variable and check for new jobs every 5
  // minutes
  let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
  loop {
    interval.tick().await;
    let mut should_request = false;

    // Scoped lock to avoid holding the Mutex across an await point
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
      // Channel closed, graceful exit
      break;
    }
  }
}

pub async fn jobs_exec(
  ctx: Arc<Mutex<crate::Context>>,
  tx: tokio::sync::mpsc::Sender<codec::ClientRequest>,
) {
  // spawn a new thread that will monitor the ctx variable and check for new jobs every 5
  // minutes
  tracing::info!("Starting jobs executor");
  let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
  loop {
    interval.tick().await;
    let mut requestqueue: Vec<codec::ClientRequest> = Vec::new();

    // Scoped lock to avoid holding the Mutex across an await point
    {
      let ctx_lock = ctx.lock().await;
      if ctx_lock.authenticated {
        ctx_lock.cache.jobs.iter().for_each(|j| {
          if j.job_status == "ready_for_execution" {
            tracing::info!(
              "Executing command: {}\n for Job {} because job is in {} state",
              j.job_command,
              j.job_name,
              j.job_status
            );
            let command: Vec<&str> = j.job_command.split(' ').collect();
            match crate::utils::run_command(command[0], &command[1..]) {
              Ok(output) => {
                tracing::info!("Command {} output: {}", command[0], output);
                requestqueue.push(codec::ClientRequest::JobsRequest(
                  codec::JobsRequest::UpdateJob(j.clone()),
                ));
              }
              Err(e) => {
                tracing::error!("Failed to execute command: {}", e);
              }
            }
          } else {
            tracing::info!("Job {} is in {} state", j.job_name, j.job_status);
          }
        })
      }
    }

    if requestqueue.is_empty() {
      for r in requestqueue {
        if tx.send(r).await.is_err() {
          // Channel closed, graceful exit
          break;
        }
      }
    }
  }
}
