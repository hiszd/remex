use crate::db::SurrealValue;
use serde::{
  Deserialize,
  Serialize,
};
use surrealdb::{
  engine::any::Any,
  Surreal,
};

use crate::db::DbError;

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
pub struct AuditLogData {
  pub table_name: String,
  pub record_id: surrealdb::types::RecordId,
  pub action: String,
  pub before_snapshot: Option<serde_json::Value>,
  pub after_snapshot: Option<serde_json::Value>,
  pub changed_by: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
pub struct AuditLog {
  pub id: surrealdb::types::RecordId,
  pub table_name: String,
  pub record_id: surrealdb::types::RecordId,
  pub action: String,
  pub before_snapshot: Option<serde_json::Value>,
  pub after_snapshot: Option<serde_json::Value>,
  pub changed_at: surrealdb::types::Datetime,
  pub changed_by: Option<String>,
}

impl AuditLog {
  pub async fn migrate(db: &Surreal<Any>) -> Result<(), DbError> {
    db.query(
      r"
        USE NS remex DB remex;
        DEFINE TABLE IF NOT EXISTS audit_log SCHEMAFULL;

        DEFINE FIELD IF NOT EXISTS table_name ON TABLE audit_log TYPE string;
        DEFINE FIELD IF NOT EXISTS record_id ON TABLE audit_log TYPE record<job | client | group>;
        DEFINE FIELD IF NOT EXISTS action ON TABLE audit_log TYPE string;
        DEFINE FIELD IF NOT EXISTS before_snapshot ON TABLE audit_log FLEXIBLE TYPE object;
        DEFINE FIELD IF NOT EXISTS after_snapshot ON TABLE audit_log FLEXIBLE TYPE object;
        DEFINE FIELD IF NOT EXISTS changed_at ON TABLE audit_log TYPE datetime DEFAULT time::now() READONLY;
        DEFINE FIELD IF NOT EXISTS changed_by ON TABLE audit_log TYPE option<string>;
      ",
    )
    .await?
    .check()?;
    Ok(())
  }
}
