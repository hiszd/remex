use serde::{
  Deserialize,
  Serialize,
};
use surrealdb_types::SurrealValue;

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
pub struct Log {
  pub id: Option<String>,
  pub client_id: String,
  pub execution_id: String,
  pub output: String,
  pub command: String,
  pub exit_code: String,
  pub start_time: String,
  pub end_time: String,
  pub created_at: Option<String>,
  pub updated_at: Option<String>,
}

impl Log {
  pub fn new(
    client_id: String,
    execution_id: String,
    output: String,
    command: String,
    exit_code: String,
    start_time: String,
    end_time: String,
  ) -> Self {
    let now = chrono::Utc::now().to_rfc3339();
    Self {
      id: None,
      client_id,
      execution_id,
      output,
      command,
      exit_code,
      start_time,
      end_time,
      created_at: Some(now.clone()),
      updated_at: Some(now),
    }
  }
}
