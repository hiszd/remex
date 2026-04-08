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
pub struct JobData {
  pub job_id: String,
  pub job_info: Job,
}

#[derive(Debug, Serialize, Deserialize, SurrealValue, Clone)]
pub struct Job {
  pub id: surrealdb::types::RecordId,
  pub job_id: String,
  pub job_info: model::jobs::Job,
}

impl Job {
  pub async fn migrate(db: &Surreal<Db>) -> Result<(), DbError> {
    // this will create the table in the database if it does not already exist
    db.query(
      r"
        USE NS remex DB remex;
        DEFINE TABLE IF NOT EXISTS job SCHEMAFULL;
        DEFINE FIELD IF NOT EXISTS job_id ON TABLE job TYPE string;
        DEFINE FIELD IF NOT EXISTS job_info ON TABLE job TYPE object;
      ",
    )
    .await?
    .check()?;
    Ok(())
  }
}

impl remex_core::db::DbOperator<Job, JobData> for Job {
  async fn create(obj: JobData, db: &Surreal<Db>) -> Result<Option<Job>, DbError> {
    let s: Option<Job> = db
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
  async fn read(id: String, db: &Surreal<Db>) -> Result<Option<Job>, DbError> {
    Ok(
      db.query("USE NS remex DB remex; SELECT * FROM job WHERE id = $id;")
        .bind(("id", id))
        .await?
        .check()?
        .take(1)?,
    )
  }
  async fn push(&mut self, db: &Surreal<Db>) -> Result<(), DbError> {
    tracing::info!("Pushing job: {}", serde_json::to_string_pretty(self).unwrap());
    let s: Option<Job> = db
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

  async fn pull(&self, db: &Surreal<Db>) -> Result<Option<Job>, DbError> {
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
