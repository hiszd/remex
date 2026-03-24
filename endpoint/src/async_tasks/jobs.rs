use std::sync::Arc;

use chrono::Utc;
use remex_core::{
  codec,
  db::dal::{
    executions::Execution,
    jobs::JobStatus,
    logs::Log,
  },
};
use tokio::sync::Mutex;
use uuid::Uuid;

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
    let mut jobs_to_exec = Vec::new();

    // Scoped lock to avoid holding the Mutex across an await point
    {
      let mut ctx_lock = ctx.lock().await;
      if ctx_lock.authenticated {
        for j in &mut ctx_lock.cache.jobs {
          if j.job.job_status == JobStatus::Pending {
            jobs_to_exec.push(j.clone());
            j.locked = true;
          } else {
            tracing::info!("Job {} is in {:?} state", j.job.job_name, j.job.job_status);
          }
        }
      }
    }

    for j in jobs_to_exec {
      let execution_id = Uuid::new_v4().to_string();
      let log_id = Uuid::new_v4().to_string();
      let start_time = Utc::now().naive_utc();

      tracing::info!(
        "Executing command: {}\n for Job {} because job is in {:?} state",
        j.job.job_command,
        j.job.job_name,
        j.job.job_status,
      );

      let command: Vec<&str> = j.job.job_command.split(' ').collect();
      let (output, exit_code) = match crate::utils::run_command(command[0], &command[1..]) {
        Ok(output) => {
          tracing::info!("Command {} output: {}", command[0], output);
          (output, 0)
        }
        Err(e) => {
          tracing::error!("Failed to execute command: {}", e);
          (e.to_string(), -1)
        }
      };

      let end_time = Utc::now().naive_utc();
      let client_id = {
        let ctx_lock = ctx.lock().await;
        ctx_lock.id.clone().unwrap_or_else(|| "unknown".to_string())
      };

      let execution = Execution {
        id: execution_id.clone(),
        job_id: Some(j.job.id.clone()),
        client_id: client_id.clone(),
        executed_at: Some(start_time),
        execution_result: Some(output.clone()),
        created_at: start_time,
        updated_at: end_time,
      };

      let log = Log {
        id: log_id,
        client_id: client_id.clone(),
        execution_id: execution_id.clone(),
        output: output.clone(),
        command: j.job.job_command.clone(),
        exit_code: exit_code.to_string(),
        start_time,
        end_time,
        created_at: end_time,
        updated_at: end_time,
      };

      {
        let mut ctx_lock = ctx.lock().await;
        ctx_lock.cache.jobs = ctx_lock
          .cache
          .jobs
          .iter()
          .map(|n| {
            let mut nj = n.clone();
            if n.job.id == j.job.id {
              nj.job.job_status = JobStatus::Completed;
            }
            nj
          })
          .collect();
      }

      if tx
        .send(codec::ClientRequest::JobsRequest(codec::JobsRequest::SendExecutions(
          j.job.id.clone(),
          vec![execution],
          vec![log],
        )))
        .await
        .is_err()
      {
        return;
      }
    }
  }
}
