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
  model::endpoint as model,
  schema::endpoint as schema,
};

#[derive(Debug, Queryable, Identifiable, Selectable, Serialize, Deserialize, Clone, ToSchema)]
#[diesel(table_name = schema::jobs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct JobCLT {
  pub id: String,
  pub job_name: String,
  pub job_type: String,
  pub job_status: String,
  pub job_shell: String,
  pub job_command: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

impl From<Job> for JobCLT {
  fn from(job: Job) -> Self {
    JobCLT {
      id: job.id,
      job_name: job.job_name,
      job_type: job.job_type.into(),
      job_status: job.job_status.into(),
      job_shell: job.job_shell,
      job_command: job.job_command,
      created_at: job.created_at,
      updated_at: job.updated_at,
    }
  }
}

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = schema::jobs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewJobCLT {
  pub id: String,
  pub job_name: String,
  pub job_type: String,
  pub job_status: String,
  pub job_shell: String,
  pub job_command: String,
  pub created_at: Option<chrono::NaiveDateTime>,
  pub updated_at: Option<chrono::NaiveDateTime>,
}

impl From<JobCLT> for NewJobCLT {
  fn from(job: JobCLT) -> Self {
    NewJobCLT {
      id: job.id,
      job_name: job.job_name,
      job_type: job.job_type,
      job_status: job.job_status,
      job_shell: job.job_shell,
      job_command: job.job_command,
      created_at: Some(job.created_at),
      updated_at: Some(job.updated_at),
    }
  }
}

impl From<Job> for NewJobCLT {
  fn from(job: Job) -> Self {
    NewJobCLT {
      id: job.id,
      job_name: job.job_name,
      job_type: job.job_type.into(),
      job_status: job.job_status.into(),
      job_shell: job.job_shell,
      job_command: job.job_command,
      created_at: Some(job.created_at),
      updated_at: Some(job.updated_at),
    }
  }
}

#[allow(dead_code)]
#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = schema::jobs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UpdateJobCLT {
  pub job_name: Option<String>,
  pub job_type: Option<String>,
  pub job_status: Option<String>,
  pub job_shell: Option<String>,
  pub job_command: Option<String>,
  pub created_at: Option<chrono::NaiveDateTime>,
  pub updated_at: Option<chrono::NaiveDateTime>,
}

impl From<JobCLT> for UpdateJobCLT {
  fn from(job: JobCLT) -> Self {
    UpdateJobCLT {
      job_name: Some(job.job_name),
      job_type: Some(job.job_type),
      job_status: Some(job.job_status),
      job_shell: Some(job.job_shell),
      job_command: Some(job.job_command),
      created_at: Some(job.created_at),
      updated_at: Some(job.updated_at),
    }
  }
}

impl From<Job> for UpdateJobCLT {
  fn from(job: Job) -> Self {
    UpdateJobCLT {
      job_name: Some(job.job_name),
      job_type: Some(job.job_type.into()),
      job_status: Some(job.job_status.into()),
      job_shell: Some(job.job_shell),
      job_command: Some(job.job_command),
      created_at: Some(job.created_at),
      updated_at: Some(job.updated_at),
    }
  }
}

#[allow(dead_code)]
#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = schema::jobs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UpsertJobCLT {
  pub job_name: Option<String>,
  pub job_type: Option<String>,
  pub job_status: Option<String>,
  pub job_shell: Option<String>,
  pub job_command: Option<String>,
  pub created_at: Option<chrono::NaiveDateTime>,
  pub updated_at: Option<chrono::NaiveDateTime>,
}

impl From<Job> for UpsertJobCLT {
  fn from(job: Job) -> Self {
    UpsertJobCLT {
      job_name: Some(job.job_name),
      job_type: Some(job.job_type.into()),
      job_status: Some(job.job_status.into()),
      job_shell: Some(job.job_shell),
      job_command: Some(job.job_command),
      created_at: Some(job.created_at),
      updated_at: Some(job.updated_at),
    }
  }
}
