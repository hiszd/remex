use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::db::model::endpoint as model;
use crate::db::schema::endpoint as schema;

#[derive(Debug, Queryable, Identifiable, Associations, Serialize, Deserialize, Clone)]
#[diesel(table_name = schema::executions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(belongs_to(model::jobs::JobCLT, foreign_key = job_id))]
pub struct ExecutionCLT {
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
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewExecutionCLT {
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
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UpdateExecutionCLT {
  job_id: Option<String>,
  client_id: String,
  executed_at: chrono::NaiveDateTime,
  execution_result: Option<String>,
}
