use diesel::prelude::*;
use serde::{
  Deserialize,
  Serialize,
};
use utoipa::ToSchema;

#[allow(unused_imports)]
use crate::db::model::server as model;
use crate::db::{
  dal::executions::Execution,
  schema::server as schema,
};

#[derive(Debug, Queryable, Selectable, Serialize, Identifiable, Deserialize, Clone, ToSchema)]
#[diesel(table_name = schema::executions)]
#[diesel(belongs_to(model::jobs::JobSRV, foreign_key = job_id))]
pub struct ExecutionSRV {
  pub id: String,
  pub job_id: Option<String>,
  pub client_id: String,
  pub executed_at: Option<chrono::NaiveDateTime>,
  pub execution_result: Option<String>,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

impl From<Execution> for ExecutionSRV {
  fn from(execution: Execution) -> Self {
    ExecutionSRV {
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

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = schema::executions)]
pub struct NewExecutionSRV {
  pub job_id: Option<String>,
  pub client_id: Option<String>,
  pub executed_at: Option<chrono::NaiveDateTime>,
  pub execution_result: Option<String>,
  pub created_at: Option<chrono::NaiveDateTime>,
  pub updated_at: Option<chrono::NaiveDateTime>,
}

impl From<ExecutionSRV> for NewExecutionSRV {
  fn from(execution: ExecutionSRV) -> Self {
    NewExecutionSRV {
      job_id: execution.job_id,
      client_id: Some(execution.client_id),
      executed_at: execution.executed_at,
      execution_result: execution.execution_result,
      created_at: Some(execution.created_at),
      updated_at: Some(execution.updated_at),
    }
  }
}

impl From<Execution> for NewExecutionSRV {
  fn from(execution: Execution) -> Self {
    NewExecutionSRV {
      job_id: execution.job_id,
      client_id: Some(execution.client_id),
      executed_at: execution.executed_at,
      execution_result: execution.execution_result,
      created_at: Some(execution.created_at),
      updated_at: Some(execution.updated_at),
    }
  }
}

#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = schema::executions)]
pub struct UpdateExecutionSRV {
  job_id: Option<String>,
  client_id: Option<String>,
  executed_at: Option<chrono::NaiveDateTime>,
  execution_result: Option<String>,
}

impl From<ExecutionSRV> for UpdateExecutionSRV {
  fn from(execution: ExecutionSRV) -> Self {
    UpdateExecutionSRV {
      job_id: execution.job_id,
      client_id: Some(execution.client_id),
      executed_at: execution.executed_at,
      execution_result: execution.execution_result,
    }
  }
}

impl From<Execution> for UpdateExecutionSRV {
  fn from(execution: Execution) -> Self {
    UpdateExecutionSRV {
      job_id: execution.job_id,
      client_id: Some(execution.client_id),
      executed_at: execution.executed_at,
      execution_result: execution.execution_result,
    }
  }
}

#[allow(dead_code)]
#[derive(Deserialize, AsChangeset, Clone)]
#[diesel(table_name = schema::executions)]
#[diesel(check_for_backend(diesel::postgres::Pg))]
pub struct UpsertExecutionSRV {
  job_id: Option<String>,
  client_id: Option<String>,
  executed_at: Option<chrono::NaiveDateTime>,
  execution_result: Option<String>,
  created_at: Option<chrono::NaiveDateTime>,
  updated_at: Option<chrono::NaiveDateTime>,
}

impl From<ExecutionSRV> for UpsertExecutionSRV {
  fn from(execution: ExecutionSRV) -> Self {
    UpsertExecutionSRV {
      job_id: execution.job_id,
      client_id: Some(execution.client_id),
      executed_at: execution.executed_at,
      execution_result: execution.execution_result,
      created_at: Some(execution.created_at),
      updated_at: Some(execution.updated_at),
    }
  }
}

impl From<Execution> for UpsertExecutionSRV {
  fn from(execution: Execution) -> Self {
    UpsertExecutionSRV {
      job_id: execution.job_id,
      client_id: Some(execution.client_id),
      executed_at: execution.executed_at,
      execution_result: execution.execution_result,
      created_at: Some(execution.created_at),
      updated_at: Some(execution.updated_at),
    }
  }
}
