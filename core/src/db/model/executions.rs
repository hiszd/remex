use serde::{
  Deserialize,
  Serialize,
};
use surrealdb::{
  engine::any::Any,
  types::SurrealValue,
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
        DEFINE TABLE IF NOT EXISTS execution SCHEMAFULL
          PERMISSIONS FOR select FULL FOR create FULL FOR update FULL;

        DEFINE FIELD IF NOT EXISTS job_id ON TABLE execution TYPE record<job>;
        DEFINE FIELD IF NOT EXISTS client_id ON TABLE execution TYPE record<client>;
        DEFINE FIELD IF NOT EXISTS status ON TABLE execution TYPE object FLEXIBLE;

        DEFINE FIELD IF NOT EXISTS output ON TABLE execution TYPE string;
        DEFINE FIELD IF NOT EXISTS command ON TABLE execution TYPE string;
        DEFINE FIELD IF NOT EXISTS exit_code ON TABLE execution TYPE string;

        DEFINE FIELD IF NOT EXISTS execution_start ON TABLE execution TYPE datetime;
        DEFINE FIELD IF NOT EXISTS execution_end ON TABLE execution TYPE datetime;

        DEFINE INDEX IF NOT EXISTS idx_job_id ON TABLE execution COLUMNS job_id;
        DEFINE INDEX IF NOT EXISTS idx_client_id ON TABLE execution COLUMNS client_id;

        DEFINE FIELD IF NOT EXISTS created_at ON TABLE execution TYPE datetime DEFAULT time::now() READONLY;
        DEFINE FIELD IF NOT EXISTS updated_at ON TABLE execution TYPE datetime VALUE time::now() READONLY;
      ",
    )
    .await?
    .check()?;
    Ok(())
  }
}

impl From<(String, ExecutionData)> for Execution {
  fn from((id, data): (String, ExecutionData)) -> Self {
    Execution {
      id: surrealdb::types::RecordId::new("execution", id.as_str()),
      job_id: data.job_id,
      client_id: data.client_id,
      status: data.status,
      output: data.output,
      command: data.command,
      exit_code: data.exit_code,
      execution_start: data.execution_start,
      execution_end: data.execution_end,
      created_at: data.created_at.unwrap_or_default(),
      updated_at: data.updated_at.unwrap_or_default(),
    }
  }
}

use crate::impl_surreal_db_operator;

impl_surreal_db_operator!(pub SurrealExecutionRepo, Execution, ExecutionData, "execution", "remex", "remex");
