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
  types::{
    SurrealValue,
    ToSql,
  },
  Surreal,
};

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

impl ExecutionCache {
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

impl remex_core::db::DbOperator<ExecutionCache, ExecutionCacheData> for ExecutionCache {
  async fn create(
    obj: ExecutionCacheData,
    db: &Surreal<Db>,
  ) -> Result<Option<ExecutionCache>, DbError> {
    let s: Option<ExecutionCache> = db
      .query(
        r"
        USE NS remex DB remex;
        CREATE execution CONTENT $data;
      ",
      )
      .bind(("data", obj))
      .await?
      .check()?
      .take(1)?;
    if let Some(execution) = s {
      Ok(Some(execution.clone()))
    } else {
      Err(DbError::OperationFailed("Failed to create execution".to_string()))
    }
  }
  async fn read(id: String, db: &Surreal<Db>) -> Result<Option<ExecutionCache>, DbError> {
    Ok(
      db.query("USE NS remex DB remex; SELECT * FROM execution WHERE id = $id;")
        .bind(("id", id))
        .await?
        .check()?
        .take(1)?,
    )
  }
  async fn push(&mut self, db: &Surreal<Db>) -> Result<(), DbError> {
    tracing::debug!("Pushing execution: {}", serde_json::to_string_pretty(self).unwrap());
    let s: Option<ExecutionCache> = db
      .query(format!(
        "USE NS remex DB remex; UPSERT execution:{} CONTENT $data;",
        self.id.key.to_sql()
      ))
      .bind(("data", self.clone()))
      .await?
      .check()?
      .take(1)?;
    if let Some(execution) = s {
      *self = execution.clone();
      Ok(())
    } else {
      Err(DbError::OperationFailed("Failed to upsert execution".to_string()))
    }
  }

  async fn pull(&self, db: &Surreal<Db>) -> Result<Option<ExecutionCache>, DbError> {
    Ok(
      db.query("USE NS remex DB remex; SELECT * FROM execution WHERE id = $id;")
        .bind(("id", self.id.key.clone()))
        .await?
        .check()?
        .take(1)?,
    )
  }

  async fn delete(&self, db: &Surreal<Db>) -> Result<(), DbError> {
    db.query("USE NS remex DB remex; DELETE execution WHERE id = $id;")
      .bind(("id", self.id.key.clone()))
      .await?
      .check()?;
    Ok(())
  }
}
