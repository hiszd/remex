use remex_core::db::model::jobs::{Job, JobType};
use surrealdb::types::RecordId;
use tokio::time::Instant;

pub mod execution;
pub mod monitor;
pub mod scheduler;
pub mod sync;

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
  SyncFromRemote,
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
