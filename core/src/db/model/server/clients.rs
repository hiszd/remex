use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::db::model::server as model;
use crate::db::schema::server as schema;

#[derive(Queryable, Selectable, Serialize)]
#[diesel(table_name = schema::clients)]
pub struct Client {
  pub id: String,
  pub secret: String,
  pub client_name: String,
  pub hardware_hash: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = schema::clients)]
pub struct NewClient {
  pub id: String,
  pub secret: String,
  pub client_name: String,
  pub hardware_hash: String,
}

#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = schema::clients)]
pub struct UpdateClient {
  secret: String,
  client_name: String,
  hardware_hash: String,
}
