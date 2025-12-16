use serde::{Deserialize, Serialize};
use sqlx::{types::Uuid, FromRow};

#[derive(Debug, FromRow, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct JobsGroupsModel {
  pub job_id: Uuid,
  pub group_id: Uuid,
  #[serde(rename = "createdAt")]
  pub created_at: chrono::DateTime<chrono::Utc>,
  #[serde(rename = "updatedAt")]
  pub updated_at: chrono::DateTime<chrono::Utc>,
}
