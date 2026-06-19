use std::{cmp::Ordering, collections::BinaryHeap, time::Duration};

use remex_core::db::model::jobs::Job;
use surrealdb::types::ToSql;
use tokio::{sync::mpsc, time::Instant};

use super::JobQueueMessage;

#[derive(Debug, Clone)]
struct ScheduledJob {
  execution_time: Instant,
  job: Job,
  client_id: String,
}

impl Ord for ScheduledJob {
  fn cmp(&self, other: &Self) -> Ordering {
    other.execution_time.cmp(&self.execution_time)
  }
}

impl PartialOrd for ScheduledJob {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl PartialEq for ScheduledJob {
  fn eq(&self, other: &Self) -> bool {
    self.execution_time == other.execution_time && self.job.id == other.job.id
  }
}

impl Eq for ScheduledJob {}

pub async fn run(mut rx: mpsc::Receiver<JobQueueMessage>) -> Result<(), crate::Error> {
  let mut heap: BinaryHeap<ScheduledJob> = BinaryHeap::new();

  loop {
    let now = Instant::now();

    let next_fire = heap
      .peek()
      .map(|t| t.execution_time.saturating_duration_since(now));

    let sleep_duration = next_fire.unwrap_or(Duration::from_secs(3600 * 24 * 365));
    tracing::debug!("Queue state: heap_size={}, next_fire={:?}", heap.len(), next_fire);
    tracing::debug!("Sleeping for {:?}", sleep_duration);
    let sleep_fut = tokio::time::sleep(sleep_duration);

    tokio::select! {
      Some(injection) = rx.recv() => {
        match injection {
          JobQueueMessage::Immediate { job, client_id } => {
            tracing::debug!("Immediate job received: {}", job.job_name);
            let job_clone = job.clone();
            let client_id_clone = client_id.clone();
            tokio::spawn(async move {
              if let Err(e) = super::execution::execute_job(job_clone, &client_id_clone).await {
                tracing::error!("Job execution failed Immediate: {}", e);
              }
            });
          }
          JobQueueMessage::Scheduled { job, execution_time, client_id } => {
            tracing::debug!("Scheduled job received: {} at {:?}", job.job_name, execution_time);
            if execution_time > now {
              tracing::debug!("Job queued: {} at {:?}", job.job_name, execution_time);
              heap.push(ScheduledJob {
                execution_time,
                job,
                client_id,
              });
            } else {
              let job_clone = job.clone();
              let client_id_clone = client_id.clone();
              tokio::spawn(async move {
                if let Err(e) = super::execution::execute_job(job_clone, &client_id_clone).await {
                  tracing::error!("Job execution failed: {}", e);
                }
              });
            }
          }
          JobQueueMessage::Remove { id } => {
            println!("Removing job from queue: {}", id.to_sql());
            heap.retain(|j| j.job.id != id);
          }
          JobQueueMessage::SyncFromRemote => {
            println!("Clearing job queue for sync...");
            tracing::info!("Sync from remote requested, clearing job queue");
            heap.clear();
          }
        }
      }
      _ = sleep_fut, if next_fire.is_some() => {
        if let Some(scheduled) = heap.pop() {
          tracing::debug!("Scheduled job firing: {}", scheduled.job.job_name);
          let job_clone = scheduled.job.clone();
          let client_id_clone = scheduled.client_id.clone();
          tokio::spawn(async move {
            if let Err(e) = super::execution::execute_job(job_clone, &client_id_clone).await {
              tracing::error!("Job execution failed: {}", e);
            }
          });
        }
      }
    }
  }
}
