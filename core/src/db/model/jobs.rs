use serde::{
  Deserialize,
  Serialize,
};
use surrealdb::{
  engine::{
    any::Any,
    local::Db,
  },
  types::{
    SurrealValue,
    ToSql,
  },
  Surreal,
};
use utoipa::ToSchema;

use crate::db::DbError;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, SurrealValue)]
#[serde(rename_all = "snake_case")]
pub enum JobType {
  Instant,
  Scheduled(surrealdb::types::Datetime),
  Recurring(surrealdb::types::Datetime, std::time::Duration),
}

impl From<String> for JobType {
  fn from(s: String) -> Self {
    match serde_json::from_str(&s) {
      Ok(v) => v,
      Err(e) => {
        tracing::info!("Failed to parse job type: {}", s);
        panic!("{}", e);
      }
    }
  }
}

impl From<JobType> for String {
  fn from(jt: JobType) -> Self { serde_json::to_string(&jt).unwrap() }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, ToSchema, SurrealValue)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
  /// The job is waiting to be picked up for execution.
  Pending,
  /// The job is currently being executed by a client.
  Running,
  /// The job has finished successfully.
  Completed,
  /// The job has failed during execution, includes an error message.
  Failed(String),
  /// The job was cancelled by a user or system administrator.
  Cancelled,
  /// The job execution exceeded the allocated time limit.
  TimedOut,
  /// The job is disabled and will not be picked up for execution.
  Disabled,
}

impl From<String> for JobStatus {
  fn from(status: String) -> Self {
    match serde_json::from_str(&status) {
      Ok(v) => v,
      Err(e) => {
        tracing::info!("Failed to parse job status: {}", status);
        panic!("{}", e);
      }
    }
  }
}
impl From<JobStatus> for String {
  fn from(val: JobStatus) -> Self { serde_json::to_string(&val).unwrap() }
}

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
pub struct JobData {
  pub job_name: String,
  pub job_type: JobType,
  pub job_status: JobStatus,
  pub job_shell: String,
  pub job_command: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
pub struct Job {
  pub id: surrealdb::types::RecordId,
  pub job_name: String,
  pub job_shell: String,
  pub job_command: String,
  pub job_type: JobType,
  pub job_status: JobStatus,
  pub assignments: Vec<surrealdb::types::RecordId>,
  pub created_at: surrealdb::types::Datetime,
  pub updated_at: surrealdb::types::Datetime,
}

impl Job {
  pub async fn migrate(db: &Surreal<Any>) -> Result<(), DbError> {
    // this will create the table in the database if it does not already exist
    db.query(
      r"
        USE NS remex DB remex;
        DEFINE TABLE IF NOT EXISTS job SCHEMAFULL
          PERMISSIONS FULL;
        DEFINE FIELD IF NOT EXISTS job_name ON TABLE job TYPE string;
        DEFINE FIELD IF NOT EXISTS job_shell ON TABLE job TYPE string;
        DEFINE FIELD IF NOT EXISTS job_command ON TABLE job TYPE string;

        DEFINE FIELD IF NOT EXISTS job_type ON TABLE job TYPE any;
        DEFINE FIELD IF NOT EXISTS job_status ON TABLE job TYPE any;

        DEFINE FIELD IF NOT EXISTS assignments ON TABLE job TYPE array<record<client | group>> DEFAULT [];

        DEFINE FIELD IF NOT EXISTS created_at ON TABLE job TYPE datetime DEFAULT time::now() READONLY;
        DEFINE FIELD IF NOT EXISTS updated_at ON TABLE job TYPE datetime VALUE time::now() READONLY;
      ",
    )
    .await?
    .check()?;
    Ok(())
  }
}

impl crate::db::DbOperator<Job, JobData> for Job {
  async fn create(obj: JobData, db: &Surreal<Db>) -> Result<Option<Job>, DbError> {
    let s: Option<Job> = db
      .query(
        r"
        USE NS remex DB remex;
        CREATE job CONTENT $data
      ",
      )
      .bind(("data", obj))
      .await?
      .check()?
      .take(1)?;
    if let Some(job) = s {
      Ok(Some(job.clone()))
    } else {
      Err(DbError::OperationFailed("Failed to create job".to_string()))
    }
  }
  async fn read(id: String, db: &Surreal<Db>) -> Result<Option<Job>, DbError> {
    Ok(
      db.query("USE NS remex DB remex; SELECT * FROM job:$id;")
        .bind(("id", id))
        .await?
        .check()?
        .take(1)?,
    )
  }
  async fn push(&mut self, db: &Surreal<Db>) -> Result<(), DbError> {
    let s: Option<Job> = db
      .query("USE NS remex DB remex; UPSERT $id CONTENT $data")
      .bind(("id", self.id.to_sql()))
      .bind(("data", self.clone()))
      .await?
      .check()?
      .take(1)?;
    if let Some(job) = s {
      *self = job;
      Ok(())
    } else {
      Err(DbError::OperationFailed("Failed to upsert job".to_string()))
    }
  }

  async fn pull(&self, db: &Surreal<Db>) -> Result<Option<Job>, DbError> {
    Ok(
      db.query("USE NS remex DB remex; SELECT * from $id;")
        .bind(("id", self.id.to_sql()))
        .await?
        .check()?
        .take(1)?,
    )
  }

  async fn delete(&self, db: &Surreal<Db>) -> Result<(), DbError> {
    db.query("USE NS remex DB remex; DELETE $id;")
      .bind(("id", self.id.to_sql()))
      .await?
      .check()?;
    Ok(())
  }
}
