use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Queryable, Identifiable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::db::schema::endpoint::jobs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Job {
  pub id: String,
  pub job_name: String,
  pub job_type: String,
  pub job_status: String,
  pub job_shell: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::endpoint::jobs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewJob {
  pub id: String,
  pub job_name: String,
  pub job_type: String,
  pub job_status: String,
  pub job_shell: String,
}

#[allow(dead_code)]
#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = crate::db::schema::endpoint::jobs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UpdateJob {
  id: String,
  job_name: String,
  job_type: String,
  job_status: String,
  job_shell: String,
}

impl From<crate::db::model::server::jobs::Job> for Job {
  fn from(job: crate::db::model::server::jobs::Job) -> Self {
    Self {
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JobExecutions {
  pub job: Job,
  pub executions: Vec<crate::db::model::endpoint::executions::ExecutionLogs>,
}
