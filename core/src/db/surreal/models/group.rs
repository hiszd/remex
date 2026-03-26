use serde::{
  Deserialize,
  Serialize,
};
use surrealdb_types::SurrealValue;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue, ToSchema)]
pub struct Group {
  pub id: Option<String>,
  pub group_name: String,
  pub created_at: Option<String>,
  pub updated_at: Option<String>,
}

impl Group {
  pub fn new(group_name: String) -> Self {
    let now = chrono::Utc::now().to_rfc3339();
    Self {
      id: None,
      group_name,
      created_at: Some(now.clone()),
      updated_at: Some(now),
    }
  }
}
