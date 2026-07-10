use remex_core::{
  db::{
    model,
    DbError,
  },
  impl_surreal_db_operator,
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

impl From<(String, JobCacheData)> for JobCache {
  fn from((id, data): (String, JobCacheData)) -> Self {
    JobCache {
      id: surrealdb::types::RecordId::new("job", id.as_str()),
      job_id: data.job_id,
      job_info: data.job_info,
      completed: data.completed,
    }
  }
}

impl JobCache {
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

impl_surreal_db_operator!(pub SurrealJobCacheRepo, JobCache, JobCacheData, "job", "remex", "remex");
