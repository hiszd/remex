use serde::{Deserialize, Serialize};
use sqlx::{types::Uuid, FromRow};

#[derive(Debug, FromRow, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct GroupModel {
  pub id: Uuid,
  pub group_name: String,
  #[serde(rename = "createdAt")]
  pub created_at: chrono::DateTime<chrono::Utc>,
  #[serde(rename = "updatedAt")]
  pub updated_at: chrono::DateTime<chrono::Utc>,
}
