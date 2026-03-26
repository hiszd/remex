#![allow(unused)]
use serde::{
  Deserialize,
  Serialize,
};
use surrealdb_types::SurrealValue;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue, ToSchema)]
pub struct Client {
  pub id: Option<String>,
  pub secret: String,
  pub client_name: String,
  pub hardware_hash: String,
  pub created_at: Option<String>,
  pub updated_at: Option<String>,
}

impl Client {
  pub fn new(secret: String, client_name: String, hardware_hash: String) -> Self {
    let now = chrono::Utc::now().to_rfc3339();
    Self {
      id: None,
      secret,
      client_name,
      hardware_hash,
      created_at: Some(now.clone()),
      updated_at: Some(now),
    }
  }
}
