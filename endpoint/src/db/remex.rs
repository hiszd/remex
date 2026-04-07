use remex_core::db::model::jobs::Job;
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
pub struct JobData {
  pub id: surrealdb::types::RecordId,
  pub job_id: String,
  pub job_info: Job,
}

impl JobData {
  pub async fn save(&self, db: &Surreal<Db>) -> Result<(), surrealdb::Error> {
    let _: Option<JobData> = db.upsert(("job", "data")).content(self.clone()).await?;
    Ok(())
  }

  pub async fn load(db: &Surreal<Db>) -> Result<Option<JobData>, surrealdb::Error> {
    let result: Option<JobData> = db.select(("job", "data")).await?;
    Ok(result)
  }

  pub fn init(db: &Surreal<Db>) -> Result<(), surrealdb::Error> {
    // this will create the table in the database if it does not already exist
    db.query(
      r"
        DEFINE TABLE IF NOT EXISTS job SCHEMAFULL;
        DEFINE FIELD job_id ON job TYPE string;
        DEFINE FIELD job_info ON job TYPE object;

        DEFINE INDEX idx_job_job_id ON TABLE job FIELDS job_id UNIQUE;
      ",
    );
    Ok(())
  }
}
