use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, FromRow, Deserialize, Serialize)]
#[allow(non_snake_case)]
pub struct ExecutorModel {
  pub id: String,
  pub name: String,
  pub command: String,
  #[serde(rename = "createdAt")]
  pub created_at: chrono::NaiveDateTime,
  #[serde(rename = "updatedAt")]
  pub updated_at: chrono::NaiveDateTime,
}
