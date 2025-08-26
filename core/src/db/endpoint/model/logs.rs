use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, FromRow, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct LogModel {
  pub id: String,
  pub client_id: String,
  pub client_name: String,
  pub log: String,
  #[serde(rename = "createdAt")]
  pub created_at: Option<chrono::NaiveDateTime>,
  #[serde(rename = "updatedAt")]
  pub updated_at: Option<chrono::NaiveDateTime>,
}
