use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Queryable, Identifiable, Associations, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::db::schema::endpoint::executions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(belongs_to(crate::db::model::endpoint::jobs::Job, foreign_key = job_id))]
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
#[diesel(table_name = crate::db::schema::endpoint::executions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
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
#[diesel(table_name = crate::db::schema::endpoint::executions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UpdateExecution {
  job_id: Option<String>,
  client_id: String,
  executed_at: chrono::NaiveDateTime,
  execution_result: Option<String>,
}

impl From<crate::db::model::server::executions::Execution> for Execution {
  fn from(execution: crate::db::model::server::executions::Execution) -> Self {
    Self {
      id: execution.id,
      job_id: execution.job_id,
      client_id: execution.client_id,
      executed_at: execution.executed_at,
      execution_result: execution.execution_result,
      created_at: execution.created_at,
      updated_at: execution.updated_at,
    }
  }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExecutionLogs {
  pub execution: Execution,
  pub logs: Vec<crate::db::model::endpoint::logs::Log>,
}
