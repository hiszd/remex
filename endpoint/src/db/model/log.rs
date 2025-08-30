use serde::{
  Deserialize,
  Serialize,
};
use sqlx::FromRow;

#[derive(Debug, FromRow, Deserialize, Serialize, PartialEq, Clone)]
#[allow(non_snake_case)]
pub struct LogModel {
  pub id: u32,
  pub uuid: String,
  pub logtype: String,
  pub message: String,
  #[serde(rename = "createdAt")]
  pub created_at: chrono::NaiveDateTime,
  #[serde(rename = "updatedAt")]
  pub updated_at: chrono::NaiveDateTime,
}

impl LogModel {
  pub fn dbvalues(&self) -> String {
    format!(
      "('{}', '{}', '{}', '{}', '{}', '{}')",
      &self.id, &self.uuid, &self.logtype, &self.message, &self.created_at, &self.updated_at
    )
  }
  pub fn dbkeys(&self) -> String {
    format!("('id', 'uuid', 'logtype', 'message', 'created_at', 'updated_at')")
  }
}
