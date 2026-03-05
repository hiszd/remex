use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::db::model::endpoint as model;
use crate::db::schema::endpoint as schema;

#[derive(Debug, Queryable, Identifiable, Associations, Serialize, Deserialize, Clone)]
#[diesel(table_name = schema::logs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(belongs_to(model::executions::ExecutionCLT, foreign_key = execution_id))]
pub struct LogCLT {
  pub id: String,
  pub client_id: String,
  pub execution_id: String,
  pub log: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = schema::logs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewLogCLT {
  pub id: String,
  pub client_id: String,
  pub execution_id: String,
  pub log: String,
}

#[allow(dead_code)]
#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = schema::logs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UpdateLogCLT {
  id: String,
  client_id: String,
  execution_id: String,
  log: String,
}
