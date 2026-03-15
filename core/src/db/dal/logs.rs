use serde::{
  Deserialize,
  Serialize,
};

use crate::db::model;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
  pub id: String,
  pub client_id: String,
  pub execution_id: String,
  pub log: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

impl From<model::endpoint::logs::LogCLT> for Log {
  fn from(log: model::endpoint::logs::LogCLT) -> Self {
    Log {
      id: log.id,
      client_id: log.client_id,
      execution_id: log.execution_id,
      log: log.log,
      created_at: log.created_at,
      updated_at: log.updated_at,
    }
  }
}

impl From<model::server::logs::LogSRV> for Log {
  fn from(log: model::server::logs::LogSRV) -> Self {
    Log {
      id: log.id,
      client_id: log.client_id,
      execution_id: log.execution_id,
      log: log.log,
      created_at: log.created_at,
      updated_at: log.updated_at,
    }
  }
}
