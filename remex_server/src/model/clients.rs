use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, FromRow, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct ClientModel {
  pub id: i64,
  pub client_id: String,
  pub clientname: String,
  #[serde(rename = "createdAt")]
  pub created_at: Option<chrono::DateTime<chrono::Utc>>,
  #[serde(rename = "updatedAt")]
  pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}
