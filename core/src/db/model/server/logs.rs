use diesel::prelude::*;
use serde::{
  Deserialize,
  Serialize,
};
use utoipa::ToSchema;

#[allow(unused_imports)]
use crate::db::model::server as model;
use crate::db::schema::server as schema;

#[derive(Debug, Queryable, Selectable, Serialize, Identifiable, Deserialize, Clone, ToSchema)]
#[diesel(table_name = schema::logs)]
#[diesel(belongs_to(model::executions::ExecutionSRV, foreign_key = execution_id))]
pub struct LogSRV {
  pub id: String,
  pub client_id: String,
  pub execution_id: String,
  pub log: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = schema::logs)]
pub struct NewLogSRV {
  pub id: String,
  pub client_id: String,
  pub execution_id: String,
  pub log: String,
}

#[allow(dead_code)]
#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = schema::logs)]
pub struct UpdateLogSRV {
  id: String,
  client_id: String,
  execution_id: String,
  log: String,
}
