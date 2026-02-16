use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Serialize)]
#[diesel(table_name = crate::db::schema::executions)]
pub struct Execution {
  pub id: String,
  pub job_id: Option<String>,
  pub client_id: String,
  pub executed_at: chrono::DateTime<chrono::Utc>,
  pub execution_result: Option<String>,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::executions)]
pub struct NewExecution {
  pub job_id: Option<String>,
  pub client_id: String,
  pub executed_at: chrono::DateTime<chrono::Utc>,
  pub execution_result: Option<String>,
  #[serde(rename = "createdAt")]
  pub created_at: chrono::DateTime<chrono::Utc>,
  #[serde(rename = "updatedAt")]
  pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = crate::db::schema::executions)]
pub struct UpdateExecution {
  job_id: Option<String>,
  client_id: String,
  executed_at: chrono::DateTime<chrono::Utc>,
  execution_result: Option<String>,
}
