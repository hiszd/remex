use remex_core::db::{
  model,
  DbError,
};
use serde::{
  Deserialize,
  Serialize,
};
use surrealdb::{
  engine::local::Db,
  types::SurrealValue,
  Surreal,
};

use remex_core::impl_surreal_db_operator;

#[derive(Debug, Serialize, Deserialize, SurrealValue, Clone)]
pub struct ExecutionCacheData {
  pub execution_id: String,
  pub execution_info: model::executions::Execution,
  pub synced: bool,
}

#[derive(Debug, Serialize, Deserialize, SurrealValue, Clone)]
pub struct ExecutionCache {
  pub id: surrealdb::types::RecordId,
  pub execution_id: String,
  pub execution_info: model::executions::Execution,
  pub synced: bool,
}

impl From<(String, ExecutionCacheData)> for ExecutionCache {
  fn from((id, data): (String, ExecutionCacheData)) -> Self {
    ExecutionCache {
      id: surrealdb::types::RecordId::new("execution", id.as_str()),
      execution_id: data.execution_id,
      execution_info: data.execution_info,
      synced: data.synced,
    }
  }
}

impl ExecutionCache {
  pub fn cache_id(&self) -> String {
    match &self.id.key {
      surrealdb::types::RecordIdKey::String(s) => s.clone(),
      _ => panic!("expected string key"),
    }
  }

  pub async fn migrate(db: &Surreal<Db>) -> Result<(), DbError> {
    db.query(
      r"
        USE NS remex DB remex;
        DEFINE TABLE IF NOT EXISTS execution SCHEMAFULL;
        DEFINE FIELD IF NOT EXISTS execution_id ON TABLE execution TYPE string;
        DEFINE FIELD IF NOT EXISTS execution_info ON TABLE execution TYPE object FLEXIBLE;
        DEFINE FIELD IF NOT EXISTS synced ON TABLE execution TYPE bool DEFAULT false;
      ",
    )
    .await?
    .check()?;
    Ok(())
  }
}

impl_surreal_db_operator!(pub SurrealExecutionCacheRepo, ExecutionCache, ExecutionCacheData, "execution", "remex", "remex");
