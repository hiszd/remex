use async_trait::async_trait;
use remex_core::db::model::jobs::{
  Job,
  JobType,
};
use surrealdb::types::RecordId;
use tokio::time::Instant;

pub mod execution;
pub mod scheduler;
pub mod sync;

/// Abstraction over sending JobQueueMessages to the scheduler.
/// Supports both mpsc channels (for tests, backward compat)
/// and direct Addr<SchedulerActor> for production.
#[async_trait]
pub trait JobSender: Send + Sync {
  async fn send_job(&self, msg: JobQueueMessage) -> Result<(), ()>;
}

#[async_trait]
impl JobSender for tokio::sync::mpsc::Sender<JobQueueMessage> {
  async fn send_job(&self, msg: JobQueueMessage) -> Result<(), ()> {
    self.send(msg).await.map_err(|_| ())
  }
}

#[async_trait]
impl JobSender for actix::Addr<scheduler::SchedulerActor> {
  async fn send_job(&self, msg: JobQueueMessage) -> Result<(), ()> {
    self.send(scheduler::InjectJob(msg)).await.map_err(|_| ())?;
    Ok(())
  }
}

/// Abstraction over job execution for testability.
/// Production uses RealJobExecutor which runs shell commands.
/// Tests use a mock that records calls.
#[async_trait]
pub trait JobExecutor: Send + Sync {
  async fn execute(
    &self,
    job: Job,
    client_id: &str,
  ) -> Result<Option<execution::ExecutionResult>, crate::Error>;
}

pub struct RealJobExecutor;

#[async_trait]
impl JobExecutor for RealJobExecutor {
  async fn execute(
    &self,
    job: Job,
    client_id: &str,
  ) -> Result<Option<execution::ExecutionResult>, crate::Error> {
    execution::execute_job(job, client_id).await
  }
}

#[derive(Debug, Clone)]
pub enum JobQueueMessage {
  Immediate {
    job: Job,
    client_id: String,
  },
  Scheduled {
    job: Job,
    execution_time: Instant,
    client_id: String,
  },
  Remove {
    id: RecordId,
  },
}

pub fn calculate_execution_time(job_type: &JobType) -> Option<Instant> {
  match job_type {
    JobType::Instant => None,
    JobType::Scheduled(dt) => {
      let datetime: chrono::DateTime<chrono::Utc> = (*dt).into();
      let duration = datetime.signed_duration_since(chrono::Utc::now());
      let millis = duration.num_milliseconds();
      if millis > 0 {
        Some(Instant::now() + std::time::Duration::from_millis(millis as u64))
      } else {
        Some(Instant::now())
      }
    }
    JobType::Recurring(dt, _interval) => {
      let datetime: chrono::DateTime<chrono::Utc> = (*dt).into();
      let duration = datetime.signed_duration_since(chrono::Utc::now());
      let millis = duration.num_milliseconds();
      if millis > 0 {
        Some(Instant::now() + std::time::Duration::from_millis(millis as u64))
      } else {
        Some(Instant::now())
      }
    }
  }
}
