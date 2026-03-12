use diesel::prelude::*;
use serde::{
  Deserialize,
  Serialize,
};
use utoipa::ToSchema;

#[allow(unused_imports)]
use crate::db::model::server as model;
use crate::db::schema::server as schema;

#[derive(Queryable, Selectable, Identifiable, Serialize, Clone, ToSchema)]
#[diesel(table_name = schema::clients)]
pub struct ClientSRV {
  pub id: String,
  pub secret: String,
  pub client_name: String,
  pub hardware_hash: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Queryable, Insertable, Serialize, Deserialize, ToSchema)]
#[diesel(table_name = schema::clients)]
pub struct NewClientSRV {
  pub id: String,
  pub secret: String,
  pub client_name: String,
  pub hardware_hash: String,
}

#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = schema::clients)]
pub struct UpdateClientSRV {
  secret: String,
  client_name: String,
  hardware_hash: String,
}
