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
pub struct JobCacheData {
  pub job_id: String,
  pub job_info: model::jobs::Job,
  pub completed: bool,
}

#[derive(Debug, Serialize, Deserialize, SurrealValue, Clone)]
pub struct JobCache {
  pub id: surrealdb::types::RecordId,
  pub job_id: String,
  pub job_info: model::jobs::Job,
  pub completed: bool,
}

impl JobCache {
  pub async fn migrate(db: &Surreal<Db>) -> Result<(), DbError> {
    db.query(
      r"
        USE NS remex DB remex;
        DEFINE TABLE IF NOT EXISTS job SCHEMAFULL;
        DEFINE FIELD IF NOT EXISTS job_id ON TABLE job TYPE string;
        DEFINE FIELD IF NOT EXISTS job_info ON TABLE job TYPE object FLEXIBLE;
        DEFINE FIELD IF NOT EXISTS completed ON TABLE job TYPE bool DEFAULT false;
      ",
    )
    .await?
    .check()?;
    Ok(())
  }
}

impl remex_core::db::DbOperator<JobCache, JobCacheData> for JobCache {
  async fn create(obj: JobCacheData, db: &Surreal<Db>) -> Result<Option<JobCache>, DbError> {
    let s: Option<JobCache> = db
      .query(
        r"
        USE NS remex DB remex;
        CREATE job CONTENT $data;
      ",
      )
      .bind(("data", obj))
      .await?
      .check()?
      .take(1)?;
    if let Some(job) = s {
      Ok(Some(job.clone()))
    } else {
      Err(DbError::OperationFailed("Failed to create job".to_string()))
    }
  }
  async fn read(id: String, db: &Surreal<Db>) -> Result<Option<JobCache>, DbError> {
    Ok(
      db.query("USE NS remex DB remex; SELECT * FROM job WHERE id = $id;")
        .bind(("id", id))
        .await?
        .check()?
        .take(1)?,
    )
  }
  async fn push(&mut self, db: &Surreal<Db>) -> Result<(), DbError> {
    tracing::debug!("Pushing job: {}", serde_json::to_string_pretty(self).unwrap());
    let s: Option<JobCache> = db
      .query(format!("USE NS remex DB remex; UPSERT job:{} CONTENT $data;", self.id.key.to_sql()))
      .bind(("data", self.clone()))
      .await?
      .check()?
      .take(1)?;
    if let Some(job) = s {
      *self = job.clone();
      Ok(())
    } else {
      Err(DbError::OperationFailed("Failed to upsert job".to_string()))
    }
  }

  async fn pull(&self, db: &Surreal<Db>) -> Result<Option<JobCache>, DbError> {
    Ok(
      db.query("USE NS remex DB remex; SELECT * FROM job WHERE id = $id;")
        .bind(("id", self.id.key.clone()))
        .await?
        .check()?
        .take(1)?,
    )
  }

  async fn delete(&self, db: &Surreal<Db>) -> Result<(), DbError> {
    db.query("USE NS remex DB remex; DELETE job WHERE id = $id;")
      .bind(("id", self.id.key.clone()))
      .await?
      .check()?;
    Ok(())
  }
}
