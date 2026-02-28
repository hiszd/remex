use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Queryable, Identifiable, Associations, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::db::schema::endpoint::logs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(belongs_to(crate::db::model::endpoint::executions::Execution, foreign_key = execution_id))]
pub struct Log {
  pub id: String,
  pub client_id: String,
  pub execution_id: String,
  pub log: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::endpoint::logs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewLog {
  pub id: String,
  pub client_id: String,
  pub execution_id: String,
  pub log: String,
}

#[allow(dead_code)]
#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = crate::db::schema::endpoint::logs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UpdateLog {
  id: String,
  client_id: String,
  execution_id: String,
  log: String,
}

impl From<crate::db::model::server::logs::Log> for Log {
  fn from(log: crate::db::model::server::logs::Log) -> Self {
    Self {
      id: log.id,
      client_id: log.client_id,
      execution_id: log.execution_id,
      log: log.log,
      created_at: log.created_at,
      updated_at: log.updated_at,
    }
  }
}
