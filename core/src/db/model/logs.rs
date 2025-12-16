use serde::{Deserialize, Serialize};
use sqlx::{types::Uuid, FromRow};

#[derive(Debug, FromRow, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct LogModel {
  pub id: Uuid,
  pub client_id: Uuid,
  pub execution_id: Uuid,
  pub log: String,
  #[serde(rename = "createdAt")]
  pub created_at: chrono::DateTime<chrono::Utc>,
  #[serde(rename = "updatedAt")]
  pub updated_at: chrono::DateTime<chrono::Utc>,
}
