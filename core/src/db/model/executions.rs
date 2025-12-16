use serde::{Deserialize, Serialize};
use sqlx::{types::Uuid, FromRow};

#[derive(Debug, FromRow, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct ExecutionModel {
  pub id: Uuid,
  pub job_id: Option<String>,
  pub client_id: String,
  pub executed_at: chrono::DateTime<chrono::Utc>,
  pub execution_result: Option<String>,
  #[serde(rename = "createdAt")]
  pub created_at: chrono::DateTime<chrono::Utc>,
  #[serde(rename = "updatedAt")]
  pub updated_at: chrono::DateTime<chrono::Utc>,
}
