use serde::{
  Deserialize,
  Serialize,
};
use surrealdb_types::SurrealValue;

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
pub struct Execution {
  pub id: Option<String>,
  pub job_id: Option<String>,
  pub client_id: String,
  pub executed_at: Option<String>,
  pub execution_result: Option<String>,
  pub created_at: Option<String>,
  pub updated_at: Option<String>,
}

impl Execution {
  pub fn new(job_id: Option<String>, client_id: String) -> Self {
    let now = chrono::Utc::now().to_rfc3339();
    Self {
      id: None,
      job_id,
      client_id,
      executed_at: None,
      execution_result: None,
      created_at: Some(now.clone()),
      updated_at: Some(now),
    }
  }
}
