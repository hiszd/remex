use serde::{
  Deserialize,
  Serialize,
};
use surrealdb_types::SurrealValue;

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
pub struct JobGroup {
  pub id: Option<String>,
  pub job_id: Option<String>,
  pub group_id: Option<String>,
  pub created_at: Option<String>,
  pub updated_at: Option<String>,
}

impl JobGroup {
  pub fn new(job_id: Option<String>, group_id: Option<String>) -> Self {
    let now = chrono::Utc::now().to_rfc3339();
    Self {
      id: None,
      job_id,
      group_id,
      created_at: Some(now.clone()),
      updated_at: Some(now),
    }
  }
}
