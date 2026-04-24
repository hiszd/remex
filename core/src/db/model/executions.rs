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

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, SurrealValue, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
  #[default]
  /// The job is currently being executed by a client.
  Running,
  /// The job has finished successfully.
  Completed,
  /// The job has failed during execution, includes an error message.
  Failed,
  /// The job was cancelled by a user or system administrator.
  Cancelled,
  /// The job execution exceeded the allocated time limit.
  TimedOut,
}

impl From<String> for ExecutionStatus {
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
impl From<ExecutionStatus> for String {
  fn from(val: ExecutionStatus) -> Self { serde_json::to_string(&val).unwrap() }
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct ExecutionData {
  pub job_id: Option<surrealdb::types::RecordId>,
  pub client_id: surrealdb::types::RecordId,
  pub status: ExecutionStatus,
  pub output: String,
  pub command: String,
  pub exit_code: String,
  pub execution_start: Option<surrealdb::types::Datetime>,
  pub execution_end: Option<surrealdb::types::Datetime>,
  pub created_at: Option<surrealdb::types::Datetime>,
  pub updated_at: Option<surrealdb::types::Datetime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SurrealValue)]
pub struct Execution {
  pub id: surrealdb::types::RecordId,
  pub job_id: Option<surrealdb::types::RecordId>,
  pub client_id: surrealdb::types::RecordId,
  pub status: ExecutionStatus,
  pub output: String,
  pub command: String,
  pub exit_code: String,
  pub execution_start: Option<surrealdb::types::Datetime>,
  pub execution_end: Option<surrealdb::types::Datetime>,
  pub created_at: surrealdb::types::Datetime,
  pub updated_at: surrealdb::types::Datetime,
}

impl Execution {
  pub async fn migrate(db: &Surreal<Any>) -> Result<(), DbError> {
    // this will create the table in the database if it does not already exist
    db.query(
      r"
        USE NS remex DB remex;
        DEFINE TABLE IF NOT EXISTS execution SCHEMAFULL;

        DEFINE FIELD IF NOT EXISTS job_id ON TABLE execution TYPE record<job>;
        DEFINE FIELD IF NOT EXISTS client_id ON TABLE execution TYPE record<client>;
        DEFINE FIELD IF NOT EXISTS status ON TABLE execution FLEXIBLE TYPE object;

        DEFINE FIELD IF NOT EXISTS output ON TABLE execution TYPE string;
        DEFINE FIELD IF NOT EXISTS command ON TABLE execution TYPE string;
        DEFINE FIELD IF NOT EXISTS exit_code ON TABLE execution TYPE string;

        DEFINE FIELD IF NOT EXISTS execution_start ON TABLE execution TYPE datetime;
        DEFINE FIELD IF NOT EXISTS execution_end ON TABLE execution TYPE datetime;

        DEFINE FIELD IF NOT EXISTS created_at ON TABLE execution TYPE datetime DEFAULT time::now() READONLY;
        DEFINE FIELD IF NOT EXISTS updated_at ON TABLE execution TYPE datetime VALUE time::now() READONLY;
      ",
    )
    .await?
    .check()?;
    Ok(())
  }
}

impl crate::db::DbOperator<Execution, ExecutionData> for Execution {
  async fn create(obj: ExecutionData, db: &Surreal<Db>) -> Result<Option<Execution>, DbError> {
    let s: Option<Execution> = db
      .query(
        r"
        USE NS remex DB remex;
        CREATE execution CONTENT $data
      ",
      )
      .bind(("data", obj))
      .await?
      .check()?
      .take(1)?;
    if let Some(execution) = s {
      Ok(Some(execution.clone()))
    } else {
      Err(DbError::OperationFailed("Failed to create execution".to_string()))
    }
  }
  async fn read(id: String, db: &Surreal<Db>) -> Result<Option<Execution>, DbError> {
    Ok(
      db.query("USE NS remex DB remex; SELECT * FROM execution WHERE id = $id;")
        .bind(("id", id))
        .await?
        .check()?
        .take(1)?,
    )
  }
  async fn push(&mut self, db: &Surreal<Db>) -> Result<(), DbError> {
    let s: Option<Execution> = db
      .query("USE NS remex DB remex; UPSERT $id CONTENT $data")
      .bind(("id", self.id.to_sql()))
      .bind(("data", self.clone()))
      .await?
      .check()?
      .take(1)?;
    if let Some(execution) = s {
      *self = execution;
      Ok(())
    } else {
      Err(DbError::OperationFailed("Failed to upsert execution".to_string()))
    }
  }

  async fn pull(&self, db: &Surreal<Db>) -> Result<Option<Execution>, DbError> {
    Ok(
      db.query("USE NS remex DB remex; SELECT * FROM $id;")
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
