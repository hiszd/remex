use serde::{
  Deserialize,
  Serialize,
};
use surrealdb::{
  engine::any::Any,
  Surreal,
};

use crate::db::{
  DbError,
  SurrealValue,
};

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
pub struct UsageEntry {
  pub client_id: surrealdb::types::RecordId,
  pub used_at: surrealdb::types::Datetime,
}

#[derive(Serialize, Deserialize, Clone, SurrealValue)]
pub struct EnrollmentTokenData {
  pub token_hash: String,
  pub valid: bool,
  pub single_use: bool,
  pub expires_at: Option<surrealdb::types::Datetime>,
  pub issued_by: surrealdb::types::RecordId,
}

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
pub struct EnrollmentToken {
  pub id: surrealdb::types::RecordId,
  pub token_hash: String,
  pub valid: bool,
  pub single_use: bool,
  pub expires_at: Option<surrealdb::types::Datetime>,
  pub issued_by: surrealdb::types::RecordId,
  pub last_used: Option<surrealdb::types::Datetime>,
  pub last_used_by: Option<surrealdb::types::RecordId>,
  pub usage_history: Vec<UsageEntry>,
  pub created_at: surrealdb::types::Datetime,
}

impl EnrollmentToken {
  pub async fn migrate(db: &Surreal<Any>) -> Result<(), DbError> {
    db.query(
      r"
        USE NS remex DB remex;
        DEFINE TABLE IF NOT EXISTS enrollment_token SCHEMAFULL
          PERMISSIONS FOR select FULL,
                    FOR create FULL,
                    FOR update FULL,
                    FOR delete NONE;
        DEFINE FIELD IF NOT EXISTS token_hash ON TABLE enrollment_token TYPE string;
        DEFINE FIELD IF NOT EXISTS valid ON TABLE enrollment_token TYPE bool DEFAULT true;
        DEFINE FIELD IF NOT EXISTS single_use ON TABLE enrollment_token TYPE bool DEFAULT true;
        DEFINE FIELD IF NOT EXISTS expires_at ON TABLE enrollment_token TYPE option<datetime>;
        DEFINE FIELD IF NOT EXISTS issued_by ON TABLE enrollment_token TYPE record<user>;
        DEFINE FIELD IF NOT EXISTS usage_history ON TABLE enrollment_token TYPE array<object> DEFAULT [];
        DEFINE FIELD IF NOT EXISTS last_used ON TABLE enrollment_token TYPE option<datetime> COMPUTED {
          RETURN array::last($this.usage_history).used_at;
        };
        DEFINE FIELD IF NOT EXISTS last_used_by ON TABLE enrollment_token TYPE option<record<client>> COMPUTED {
          RETURN array::last($this.usage_history).client_id;
        };
        DEFINE FIELD IF NOT EXISTS created_at ON TABLE enrollment_token TYPE datetime DEFAULT time::now() READONLY;
        DEFINE INDEX IF NOT EXISTS idx_token_hash ON TABLE enrollment_token COLUMNS token_hash UNIQUE;
      ",
    )
    .await?
    .check()?;

    // Remove old field definitions — log warning if removal fails
    if let Err(e) = db.query("REMOVE FIELD used_at ON TABLE enrollment_token").await {
      tracing::warn!("Failed to remove old field used_at: {e}");
    }
    if let Err(e) = db.query("REMOVE FIELD used_by ON TABLE enrollment_token").await {
      tracing::warn!("Failed to remove old field used_by: {e}");
    }

    Ok(())
  }
}
