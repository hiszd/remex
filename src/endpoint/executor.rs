use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(non_snake_case)]
pub struct Executor {
  pub id: String,
  pub name: String,
  pub command: String,
  #[serde(rename = "createdAt")]
  pub created_at: chrono::NaiveDateTime,
  #[serde(rename = "updatedAt")]
  pub updated_at: chrono::NaiveDateTime,
}
