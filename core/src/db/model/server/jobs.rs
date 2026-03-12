use diesel::prelude::*;
use serde::{
  Deserialize,
  Serialize,
};
use utoipa::ToSchema;

#[allow(unused_imports)]
use crate::db::{
  dal::{
    self,
    jobs::Job,
  },
  model::server as model,
  schema::server as schema,
};

#[derive(Debug, Queryable, Selectable, Serialize, Identifiable, Deserialize, Clone, ToSchema)]
#[diesel(table_name = schema::jobs)]
pub struct JobSRV {
  pub id: String,
  pub job_name: String,
  pub job_type: String,
  pub job_status: String,
  pub job_status_message: Option<String>,
  pub job_shell: String,
  pub job_command: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Queryable, Insertable, Serialize, Deserialize, ToSchema)]
#[diesel(table_name = schema::jobs)]
pub struct NewJobSRV {
  pub id: String,
  pub job_name: String,
  pub job_type: String,
  pub job_status: String,
  pub job_status_message: Option<String>,
  pub job_shell: String,
  pub job_command: String,
  pub created_at: Option<chrono::NaiveDateTime>,
  pub updated_at: Option<chrono::NaiveDateTime>,
}

impl From<JobSRV> for NewJobSRV {
  fn from(job: JobSRV) -> Self {
    NewJobSRV {
      id: job.id,
      job_name: job.job_name,
      job_type: job.job_type,
      job_status: job.job_status,
      job_status_message: job.job_status_message,
      job_shell: job.job_shell,
      job_command: job.job_command,
      created_at: Some(job.created_at),
      updated_at: Some(job.updated_at),
    }
  }
}

impl From<Job> for NewJobSRV {
  fn from(job: Job) -> Self {
    NewJobSRV {
      id: job.id,
      job_name: job.job_name,
      job_type: job.job_type,
      job_status: job.job_status,
      job_status_message: job.job_status_message,
      job_shell: job.job_shell,
      job_command: job.job_command,
      created_at: Some(job.created_at),
      updated_at: Some(job.updated_at),
    }
  }
}

#[derive(Deserialize, AsChangeset, Identifiable, ToSchema)]
#[diesel(table_name = schema::jobs)]
pub struct UpdateJobSRV {
  pub id: String,
  pub job_name: Option<String>,
  pub job_type: Option<String>,
  pub job_status: Option<String>,
  pub job_status_message: Option<String>,
  pub job_shell: Option<String>,
  pub job_command: Option<String>,
  pub created_at: Option<chrono::NaiveDateTime>,
  pub updated_at: Option<chrono::NaiveDateTime>,
}

impl From<JobSRV> for UpdateJobSRV {
  fn from(job: JobSRV) -> Self {
    UpdateJobSRV {
      id: job.id,
      job_name: Some(job.job_name),
      job_type: Some(job.job_type),
      job_status: Some(job.job_status),
      job_status_message: job.job_status_message,
      job_shell: Some(job.job_shell),
      job_command: Some(job.job_command),
      created_at: Some(job.created_at),
      updated_at: Some(job.updated_at),
    }
  }
}

impl From<Job> for UpdateJobSRV {
  fn from(job: Job) -> Self {
    UpdateJobSRV {
      id: job.id,
      job_name: Some(job.job_name),
      job_type: Some(job.job_type),
      job_status: Some(job.job_status),
      job_status_message: job.job_status_message,
      job_shell: Some(job.job_shell),
      job_command: Some(job.job_command),
      created_at: Some(job.created_at),
      updated_at: Some(job.updated_at),
    }
  }
}
