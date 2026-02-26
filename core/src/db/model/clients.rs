use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Serialize)]
#[diesel(table_name = crate::db::schema::clients)]
pub struct Client {
  pub id: String,
  pub secret: String,
  pub client_name: String,
  pub hardware_hash: String,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::clients)]
pub struct NewClient {
  pub id: String,
  pub secret: String,
  pub client_name: String,
  pub hardware_hash: String,
}

#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = crate::db::schema::clients)]
pub struct UpdateClient {
  secret: String,
  client_name: String,
  hardware_hash: String,
}
