use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::db::model::server as model;
use crate::db::schema::server as schema;

#[derive(Queryable, Selectable)]
#[diesel(table_name = schema::logs)]
pub struct Log {
  pub id: String,
  pub client_id: String,
  pub execution_id: String,
  pub log: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = schema::logs)]
pub struct NewLog {
  pub id: String,
  pub client_id: String,
  pub execution_id: String,
  pub log: String,
}

#[allow(dead_code)]
#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = schema::logs)]
pub struct UpdateLog {
  id: String,
  client_id: String,
  execution_id: String,
  log: String,
}
