use serde::{
  Deserialize,
  Serialize,
};

use crate::db::model;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Execution {
  pub id: String,
  pub job_id: Option<String>,
  pub client_id: String,
  pub executed_at: Option<chrono::NaiveDateTime>,
  pub execution_result: Option<String>,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

impl From<model::server::executions::ExecutionSRV> for Execution {
  fn from(execution: model::server::executions::ExecutionSRV) -> Self {
    Execution {
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

impl From<model::endpoint::executions::ExecutionCLT> for Execution {
  fn from(execution: model::endpoint::executions::ExecutionCLT) -> Self {
    Execution {
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
