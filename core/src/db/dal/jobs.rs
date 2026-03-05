use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::db::model;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
  Pending,
  Running,
  Completed,
  Failed(String),
  Cancelled,
  TimedOut,
  Disabled,
}

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Job {
  pub id: String,
  pub job_name: String,
  pub job_type: String,
  pub job_status: String,
  pub job_shell: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

impl From<model::server::jobs::JobSRV> for Job {
  fn from(job: model::server::jobs::JobSRV) -> Self {
    Job {
      id: job.id,
      job_name: job.job_name,
      job_type: job.job_type,
      job_status: job.job_status,
      job_shell: job.job_shell,
      created_at: job.created_at,
      updated_at: job.updated_at,
    }
  }
}
impl From<model::endpoint::jobs::JobCLT> for Job {
  fn from(job: model::endpoint::jobs::JobCLT) -> Self {
    Job {
      id: job.id,
      job_name: job.job_name,
      job_type: job.job_type,
      job_status: job.job_status,
      job_shell: job.job_shell,
      created_at: job.created_at,
      updated_at: job.updated_at,
    }
  }
}

impl From<Job> for model::server::jobs::JobSRV {
  fn from(job: Job) -> Self {
    model::server::jobs::JobSRV {
      id: job.id,
      job_name: job.job_name,
      job_type: job.job_type,
      job_status: job.job_status,
      job_shell: job.job_shell,
      created_at: job.created_at,
      updated_at: job.updated_at,
    }
  }
}
impl From<Job> for model::endpoint::jobs::JobCLT {
  fn from(job: Job) -> Self {
    model::endpoint::jobs::JobCLT {
      id: job.id,
      job_name: job.job_name,
      job_type: job.job_type,
      job_status: job.job_status,
      job_shell: job.job_shell,
      created_at: job.created_at,
      updated_at: job.updated_at,
    }
  }
}
