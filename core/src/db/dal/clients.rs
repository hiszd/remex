use diesel::{
  QueryDsl,
  RunQueryDsl,
};
use serde::{
  Deserialize,
  Serialize,
};
use utoipa::ToSchema;

use crate::db::{
  model,
  schema,
};

#[derive(Serialize, Deserialize, ToSchema, Clone)]
pub struct Client {
  pub id: String,
  pub secret: String,
  pub client_name: String,
  pub hardware_hash: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

impl From<model::server::clients::ClientSRV> for Client {
  fn from(client: model::server::clients::ClientSRV) -> Self {
    Client {
      id: client.id,
      secret: client.secret,
      client_name: client.client_name,
      hardware_hash: client.hardware_hash,
      created_at: client.created_at,
      updated_at: client.updated_at,
    }
  }
}
