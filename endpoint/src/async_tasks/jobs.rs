use std::sync::Arc;

use chrono::Utc;
use remex_core::{
  codec,
  db::surreal::models::{
    Execution,
    Job,
    JobStatus,
    Log,
  },
};
use tokio::sync::Mutex;
use uuid::Uuid;

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
    let mut jobs_to_exec = Vec::new();

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
      let start_time = Utc::now().to_rfc3339();

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

      let end_time = Utc::now().to_rfc3339();
      let client_id = {
        let ctx_lock = ctx.lock().await;
        ctx_lock.id.clone().unwrap_or_else(|| "unknown".to_string())
      };

      let job_id = j.job.id.clone().unwrap_or_default();

      let execution = Execution {
        id: Some(execution_id.clone()),
        job_id: Some(job_id.clone()),
        client_id: client_id.clone(),
        executed_at: Some(start_time.clone()),
        execution_result: Some(output.clone()),
        created_at: Some(start_time.clone()),
        updated_at: Some(end_time.clone()),
      };

      let log = Log {
        id: Some(log_id),
        client_id: client_id.clone(),
        execution_id: execution_id.clone(),
        output: output.clone(),
        command: j.job.job_command.clone(),
        exit_code: exit_code.to_string(),
        start_time: start_time.clone(),
        end_time: end_time.clone(),
        created_at: Some(end_time.clone()),
        updated_at: Some(end_time),
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
          job_id,
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
