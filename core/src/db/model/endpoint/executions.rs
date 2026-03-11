use diesel::prelude::*;
use serde::{
  Deserialize,
  Serialize,
};

use crate::db::{
  model::endpoint as model,
  schema::endpoint as schema,
};

#[derive(
  Debug, Queryable, Identifiable, Selectable, Associations, Serialize, Deserialize, Clone,
)]
#[diesel(table_name = schema::executions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[diesel(belongs_to(model::jobs::JobCLT, foreign_key = job_id))]
pub struct ExecutionCLT {
  pub id: String,
  pub job_id: Option<String>,
  pub client_id: String,
  pub executed_at: Option<chrono::NaiveDateTime>,
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
  pub executed_at: Option<chrono::NaiveDateTime>,
  pub execution_result: Option<String>,
  pub created_at: Option<chrono::NaiveDateTime>,
  pub updated_at: Option<chrono::NaiveDateTime>,
}

impl From<ExecutionCLT> for NewExecutionCLT {
  fn from(execution: ExecutionCLT) -> Self {
    NewExecutionCLT {
      job_id: execution.job_id,
      client_id: execution.client_id,
      executed_at: execution.executed_at,
      execution_result: execution.execution_result,
      created_at: Some(execution.created_at),
      updated_at: Some(execution.updated_at),
    }
  }
}

#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = schema::executions)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UpdateExecutionCLT {
  job_id: Option<String>,
  client_id: Option<String>,
  executed_at: Option<chrono::NaiveDateTime>,
  execution_result: Option<String>,
}

impl From<ExecutionCLT> for UpdateExecutionCLT {
  fn from(job: ExecutionCLT) -> Self {
    UpdateExecutionCLT {
      job_id: job.job_id,
      client_id: Some(job.client_id),
      executed_at: job.executed_at,
      execution_result: job.execution_result,
    }
  }
}
