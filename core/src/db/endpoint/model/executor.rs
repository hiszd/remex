use serde::{
  Deserialize,
  Serialize,
};
use sqlx::FromRow;

#[derive(Debug, FromRow, Deserialize, Serialize, PartialEq, Clone)]
#[allow(non_snake_case)]
pub struct ExecutorModel {
  pub id: String,
  pub name: String,
  pub command: String,
  pub status: String,
  pub active: bool,
  #[serde(rename = "createdAt")]
  pub created_at: chrono::NaiveDateTime,
  #[serde(rename = "updatedAt")]
  pub updated_at: chrono::NaiveDateTime,
}

impl ExecutorModel {
  pub fn dbvalues(&self) -> String {
    format!(
      "({}, '{}', '{}', '{}', {}, '{}', '{}')",
      &self.id,
      &self.name,
      &self.command,
      &self.status,
      &self.active,
      &self.created_at,
      &self.updated_at
    )
  }
}
