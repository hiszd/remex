use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::db::model::server as model;
use crate::db::schema::server as schema;

#[derive(Queryable, Serialize)]
#[diesel(table_name = schema::executions)]
pub struct Execution {
  pub id: String,
  pub job_id: Option<String>,
  pub client_id: String,
  pub executed_at: chrono::NaiveDateTime,
  pub execution_result: Option<String>,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = schema::executions)]
pub struct NewExecution {
  pub job_id: Option<String>,
  pub client_id: String,
  pub executed_at: chrono::NaiveDateTime,
  pub execution_result: Option<String>,
  #[serde(rename = "createdAt")]
  pub created_at: chrono::NaiveDateTime,
  #[serde(rename = "updatedAt")]
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = schema::executions)]
pub struct UpdateExecution {
  job_id: Option<String>,
  client_id: String,
  executed_at: chrono::NaiveDateTime,
  execution_result: Option<String>,
}
