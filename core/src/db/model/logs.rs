use diesel::prelude::*;
use serde::{Deserialize, Serialize};
#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::db::schema::logs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Log {
  pub id: String,
  pub client_id: String,
  pub execution_id: String,
  pub log: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::logs)]
pub struct NewLog {
  pub id: String,
  pub client_id: String,
  pub execution_id: String,
  pub log: String,
}

#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = crate::db::schema::logs)]
pub struct UpdateLog {
  id: String,
  client_id: String,
  execution_id: String,
  log: String,
}
