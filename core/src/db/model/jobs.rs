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
  pub timeout: Option<surrealdb::types::Duration>,
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
  pub timeout: Option<surrealdb::types::Duration>,
  pub created_at: surrealdb::types::Datetime,
  pub updated_at: surrealdb::types::Datetime,
}

impl Job {
  pub async fn migrate(db: &Surreal<Any>) -> Result<(), DbError> {
    db.query(
      r"
        USE NS remex DB remex;
        DEFINE TABLE IF NOT EXISTS job SCHEMAFULL PERMISSIONS
          FOR select FULL,
          FOR create WHERE $auth.id INSIDE (SELECT VALUE id FROM user),
          FOR update WHERE $auth.id INSIDE (SELECT VALUE id FROM user),
          FOR delete WHERE $auth.id INSIDE (SELECT VALUE id FROM user);
        DEFINE FIELD IF NOT EXISTS job_name ON TABLE job TYPE string;
        DEFINE FIELD IF NOT EXISTS job_shell ON TABLE job TYPE string;
        DEFINE FIELD IF NOT EXISTS job_command ON TABLE job TYPE string;

        DEFINE FIELD IF NOT EXISTS timeout ON TABLE job TYPE option<duration>;

        DEFINE FIELD IF NOT EXISTS job_type ON TABLE job TYPE object FLEXIBLE;
        DEFINE FIELD IF NOT EXISTS execution_status ON TABLE job TYPE object COMPUTED
          IF count((SELECT id FROM execution WHERE job_id = $this.id)) = 0
            THEN { Pending: {} }
          ELSE IF count((SELECT id FROM execution WHERE job_id = $this.id AND status = { Failed: {} } AND execution_start = (SELECT VALUE math::max(execution_start) FROM execution WHERE job_id = $this.id AND client_id = e.client_id))) > 0
            THEN { Failed: {} }
          ELSE IF count((SELECT id FROM execution WHERE job_id = $this.id AND status = { TimedOut: {} } AND execution_start = (SELECT VALUE math::max(execution_start) FROM execution WHERE job_id = $this.id AND client_id = e.client_id))) = count((SELECT client_id FROM execution WHERE job_id = $this.id GROUP BY client_id))
            THEN { TimedOut: {} }
          ELSE IF count((SELECT id FROM execution WHERE job_id = $this.id AND status = { Completed: {} } AND execution_start = (SELECT VALUE math::max(execution_start) FROM execution WHERE job_id = $this.id AND client_id = e.client_id))) = count((SELECT client_id FROM execution WHERE job_id = $this.id GROUP BY client_id))
            THEN { Completed: {} }
          ELSE { Running: {} }
          END;
        DEFINE FIELD IF NOT EXISTS enabled ON TABLE job TYPE object FLEXIBLE DEFAULT { Draft: {} };

        DEFINE FIELD IF NOT EXISTS assignments ON TABLE job TYPE array<record<client | group>> DEFAULT [];

        DEFINE FIELD IF NOT EXISTS created_at ON TABLE job TYPE datetime DEFAULT time::now() READONLY;
        DEFINE FIELD IF NOT EXISTS updated_at ON TABLE job TYPE datetime VALUE time::now() READONLY;

        DEFINE EVENT IF NOT EXISTS audit_job ON TABLE job
        WHEN $event IN ['CREATE', 'UPDATE', 'DELETE']
        THEN {
          CREATE audit_log SET
            table_name = 'job',
            record_id = $after.id ?? $before.id,
            action = $event,
            before_snapshot = IF $event = 'CREATE' THEN {} ELSE $before END,
            after_snapshot = IF $event = 'DELETE' THEN {} ELSE $after END,
            changed_by = $auth.id;
        };
      ",
    )
    .await?
    .check()?;
    Ok(())
  }
}

impl From<(String, JobData)> for Job {
  fn from((id, data): (String, JobData)) -> Self {
    Job {
      id: surrealdb::types::RecordId::new("job", id.as_str()),
      job_name: data.job_name,
      job_shell: data.job_shell,
      job_command: data.job_command,
      job_type: data.job_type,
      execution_status: data.execution_status,
      enabled: data.enabled,
      timeout: data.timeout,
      assignments: Vec::new(),
      created_at: surrealdb::types::Datetime::default(),
      updated_at: surrealdb::types::Datetime::default(),
    }
  }
}

use crate::impl_surreal_db_operator;

impl_surreal_db_operator!(pub SurrealJobRepo, Job, JobData, "job", "remex", "remex");
