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
