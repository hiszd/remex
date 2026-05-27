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

use crate::db::DbError;

#[derive(
  Debug, Serialize, Deserialize, PartialEq, Clone, SurrealValue, remex_macros::SerdeIntoString,
)]
#[serde(rename_all = "snake_case")]
pub enum JobType {
  Instant,
  Scheduled(surrealdb::types::Datetime),
  Recurring(surrealdb::types::Datetime, std::time::Duration),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, SurrealValue, Default)]
#[serde(rename_all = "snake_case")]
pub enum Enabled {
  #[default]
  Draft,
  Enabled,
  Disabled,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, SurrealValue, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
  #[default]
  Pending,
  Running,
  Completed,
  Failed,
  TimedOut,
}

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
pub struct JobData {
  pub job_name: String,
  pub job_type: JobType,
  pub execution_status: ExecutionStatus,
  pub enabled: Enabled,
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
  pub execution_status: ExecutionStatus,
  pub enabled: Enabled,
  pub assignments: Vec<surrealdb::types::RecordId>,
  pub created_at: surrealdb::types::Datetime,
  pub updated_at: surrealdb::types::Datetime,
}

impl Job {
  pub async fn migrate(db: &Surreal<Any>) -> Result<(), DbError> {
    db.query(
      r"
        USE NS remex DB remex;
        DEFINE TABLE IF NOT EXISTS job SCHEMAFULL
          PERMISSIONS FULL;
        DEFINE FIELD IF NOT EXISTS job_name ON TABLE job TYPE string;
        DEFINE FIELD IF NOT EXISTS job_shell ON TABLE job TYPE string;
        DEFINE FIELD IF NOT EXISTS job_command ON TABLE job TYPE string;

        DEFINE FIELD IF NOT EXISTS job_type ON TABLE job FLEXIBLE TYPE object;
        DEFINE FIELD IF NOT EXISTS execution_status ON TABLE job FLEXIBLE TYPE object COMPUTED {
          LET $execs = (SELECT status FROM execution WHERE job_id = $this.id);
          IF array::len($execs) = 0 THEN
            RETURN { Pending: {} };
          END IF;
          IF (SELECT VALUE status FROM $execs WHERE status = { Failed: {} }) THEN
            RETURN { Failed: {} };
          END IF;
          IF array::len($execs) = (SELECT VALUE array::len((SELECT VALUE status FROM $execs WHERE status = { TimedOut: {} }))) THEN
            RETURN { TimedOut: {} };
          END IF;
          IF array::len($execs) = (SELECT VALUE array::len((SELECT VALUE status FROM $execs WHERE status = { Completed: {} }))) THEN
            RETURN { Completed: {} };
          END IF;
          RETURN { Running: {} };
        };
        DEFINE FIELD IF NOT EXISTS enabled ON TABLE job FLEXIBLE TYPE object DEFAULT { Draft: {} };

        DEFINE FIELD IF NOT EXISTS assignments ON TABLE job TYPE array<record<client | group>> DEFAULT [];

        DEFINE FIELD IF NOT EXISTS created_at ON TABLE job TYPE datetime DEFAULT time::now() READONLY;
        DEFINE FIELD IF NOT EXISTS updated_at ON TABLE job TYPE datetime VALUE time::now() READONLY;

        DEFINE EVENT audit_job ON TABLE job
        WHEN $event IN ['CREATE', 'UPDATE', 'DELETE']
        THEN {
          CREATE audit_log SET
            table_name = 'job',
            record_id = $after.id ?? $before.id,
            action = $event,
            before_snapshot = $before,
            after_snapshot = $after,
            changed_by = $auth.id;
        };
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
