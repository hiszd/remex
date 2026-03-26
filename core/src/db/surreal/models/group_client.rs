use serde::{
  Deserialize,
  Serialize,
};
use surrealdb_types::SurrealValue;

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
pub struct GroupClient {
  pub id: Option<String>,
  pub group_id: Option<String>,
  pub client_id: Option<String>,
  pub created_at: Option<String>,
  pub updated_at: Option<String>,
}

impl GroupClient {
  pub fn new(group_id: Option<String>, client_id: Option<String>) -> Self {
    let now = chrono::Utc::now().to_rfc3339();
    Self {
      id: None,
      group_id,
      client_id,
      created_at: Some(now.clone()),
      updated_at: Some(now),
    }
  }
}
