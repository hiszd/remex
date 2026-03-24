use diesel::prelude::*;
use serde::{
  Deserialize,
  Serialize,
};
use utoipa::ToSchema;

#[allow(unused_imports)]
use crate::db::model::server as model;
use crate::db::{
  dal::logs::Log,
  schema::server as schema,
};

#[derive(Debug, Queryable, Selectable, Serialize, Identifiable, Deserialize, Clone, ToSchema)]
#[diesel(table_name = schema::logs)]
#[diesel(belongs_to(model::executions::ExecutionSRV, foreign_key = execution_id))]
pub struct LogSRV {
  pub id: String,
  pub client_id: String,
  pub execution_id: String,
  pub output: String,
  pub command: String,
  pub exit_code: String,
  pub start_time: chrono::NaiveDateTime,
  pub end_time: chrono::NaiveDateTime,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = schema::logs)]
pub struct NewLogSRV {
  pub id: String,
  pub client_id: String,
  pub execution_id: String,
  pub output: String,
  pub command: String,
  pub exit_code: String,
  pub start_time: chrono::NaiveDateTime,
  pub end_time: chrono::NaiveDateTime,
  pub created_at: Option<chrono::NaiveDateTime>,
  pub updated_at: Option<chrono::NaiveDateTime>,
}

impl From<LogSRV> for NewLogSRV {
  fn from(log: LogSRV) -> Self {
    NewLogSRV {
      id: log.id,
      client_id: log.client_id,
      execution_id: log.execution_id,
      output: log.output,
      command: log.command,
      exit_code: log.exit_code,
      start_time: log.start_time,
      end_time: log.end_time,
      created_at: Some(log.created_at),
      updated_at: Some(log.updated_at),
    }
  }
}

impl From<Log> for NewLogSRV {
  fn from(log: Log) -> Self {
    NewLogSRV {
      id: log.id,
      client_id: log.client_id,
      execution_id: log.execution_id,
      output: log.output,
      command: log.command,
      exit_code: log.exit_code,
      start_time: log.start_time,
      end_time: log.end_time,
      created_at: Some(log.created_at),
      updated_at: Some(log.updated_at),
    }
  }
}

#[allow(dead_code)]
#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = schema::logs)]
pub struct UpdateLogSRV {
  id: Option<String>,
  client_id: Option<String>,
  execution_id: Option<String>,
  output: Option<String>,
  command: Option<String>,
  exit_code: Option<String>,
  start_time: Option<chrono::NaiveDateTime>,
  end_time: Option<chrono::NaiveDateTime>,
  created_at: Option<chrono::NaiveDateTime>,
  updated_at: Option<chrono::NaiveDateTime>,
}

impl From<LogSRV> for UpdateLogSRV {
  fn from(log: LogSRV) -> Self {
    UpdateLogSRV {
      id: Some(log.id),
      client_id: Some(log.client_id),
      execution_id: Some(log.execution_id),
      output: Some(log.output),
      command: Some(log.command),
      exit_code: Some(log.exit_code),
      start_time: Some(log.start_time),
      end_time: Some(log.end_time),
      created_at: Some(log.created_at),
      updated_at: Some(log.updated_at),
    }
  }
}

impl From<Log> for UpdateLogSRV {
  fn from(log: Log) -> Self {
    UpdateLogSRV {
      id: Some(log.id),
      client_id: Some(log.client_id),
      execution_id: Some(log.execution_id),
      output: Some(log.output),
      command: Some(log.command),
      exit_code: Some(log.exit_code),
      start_time: Some(log.start_time),
      end_time: Some(log.end_time),
      created_at: Some(log.created_at),
      updated_at: Some(log.updated_at),
    }
  }
}

#[allow(dead_code)]
#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = schema::logs)]
#[diesel(check_for_backend(diesel::postgres::Pg))]
pub struct UpsertLogSRV {
  id: Option<String>,
  client_id: Option<String>,
  execution_id: Option<String>,
  output: Option<String>,
  command: Option<String>,
  exit_code: Option<String>,
  start_time: Option<chrono::NaiveDateTime>,
  end_time: Option<chrono::NaiveDateTime>,
  created_at: Option<chrono::NaiveDateTime>,
  updated_at: Option<chrono::NaiveDateTime>,
}

impl From<LogSRV> for UpsertLogSRV {
  fn from(log: LogSRV) -> Self {
    UpsertLogSRV {
      id: Some(log.id),
      client_id: Some(log.client_id),
      execution_id: Some(log.execution_id),
      output: Some(log.output),
      command: Some(log.command),
      exit_code: Some(log.exit_code),
      start_time: Some(log.start_time),
      end_time: Some(log.end_time),
      created_at: Some(log.created_at),
      updated_at: Some(log.updated_at),
    }
  }
}

impl From<Log> for UpsertLogSRV {
  fn from(log: Log) -> Self {
    UpsertLogSRV {
      id: Some(log.id),
      client_id: Some(log.client_id),
      execution_id: Some(log.execution_id),
      output: Some(log.output),
      command: Some(log.command),
      exit_code: Some(log.exit_code),
      start_time: Some(log.start_time),
      end_time: Some(log.end_time),
      created_at: Some(log.created_at),
      updated_at: Some(log.updated_at),
    }
  }
}
