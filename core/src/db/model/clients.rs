use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, FromRow, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct ClientModel {
  pub id: String,
  pub secret: String,
  pub client_name: String,
  #[serde(rename = "createdAt")]
  pub created_at: chrono::DateTime<chrono::Utc>,
  #[serde(rename = "updatedAt")]
  pub updated_at: chrono::DateTime<chrono::Utc>,
}
