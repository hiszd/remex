use diesel::{
  QueryDsl,
  RunQueryDsl,
};
use serde::{
  Deserialize,
  Serialize,
};
use utoipa::ToSchema;

use crate::db::{
  model,
  schema,
};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
  Pending,
  Running,
  Completed,
  Failed(String),
  Cancelled,
  TimedOut,
  Disabled,
}

impl From<model::server::jobs::JobSRV> for Job {
  fn from(job: model::server::jobs::JobSRV) -> Self {
    Job {
      id: job.id,
      job_name: job.job_name,
      job_type: job.job_type,
      job_status: job.job_status,
      job_status_message: job.job_status_message,
      job_shell: job.job_shell,
      job_command: job.job_command,
      created_at: job.created_at,
      updated_at: job.updated_at,
    }
  }
}
impl From<model::endpoint::jobs::JobCLT> for Job {
  fn from(job: model::endpoint::jobs::JobCLT) -> Self {
    Job {
      id: job.id,
      job_name: job.job_name,
      job_type: job.job_type,
      job_status: job.job_status,
      job_status_message: job.job_status_message,
      job_shell: job.job_shell,
      job_command: job.job_command,
      created_at: job.created_at,
      updated_at: job.updated_at,
    }
  }
}

// This is the job object that we will use to handle data being sent between the server and
// endpoint. This is not modeled in the database, but it is the model that will be used in the code
// anywhere that the job is used. It will also house functions that will write to the database
// either for the client, or for the server.
#[derive(Debug, Serialize, Deserialize, Clone, ToSchema)]
pub struct Job {
  pub id: String,
  pub job_name: String,
  pub job_type: String,
  pub job_status: String,
  pub job_status_message: Option<String>,
  pub job_shell: String,
  pub job_command: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

impl super::CltDbOperator for Job {
  fn create_clt(&self, conn: &mut diesel::SqliteConnection) -> Result<Self, diesel::result::Error> {
    use model::endpoint::jobs::{
      JobCLT,
      NewJobCLT,
    };
    use schema::endpoint::jobs;
    match diesel::insert_into(jobs::table)
      .values(NewJobCLT::from(self.clone()))
      .get_result::<JobCLT>(conn)
    {
      Ok(job) => Ok(job.into()),
      Err(e) => Err(e),
    }
  }
  fn update_clt(&self, conn: &mut diesel::SqliteConnection) -> Result<Self, diesel::result::Error> {
    use model::endpoint::jobs::{
      JobCLT,
      UpdateJobCLT,
    };
    use schema::endpoint::jobs;
    match diesel::update(jobs::table.find(self.id.clone()))
      .set(UpdateJobCLT::from(self.clone()))
      .get_result::<JobCLT>(conn)
    {
      Ok(job) => Ok(job.into()),
      Err(e) => Err(e),
    }
  }
  fn delete_clt(&self, conn: &mut diesel::SqliteConnection) -> Result<(), diesel::result::Error> {
    use schema::endpoint::jobs;
    match diesel::delete(jobs::table.find(self.id.clone())).execute(conn) {
      Ok(_) => Ok(()),
      Err(e) => Err(e),
    }
  }
  fn read_clt(&self, conn: &mut diesel::SqliteConnection) -> Result<Self, diesel::result::Error> {
    use model::endpoint::jobs::JobCLT;
    use schema::endpoint::jobs;
    match jobs::table.find(self.id.clone()).get_result::<JobCLT>(conn) {
      Ok(job) => Ok(job.into()),
      Err(e) => Err(e),
    }
  }
  fn upsert_clt(&self, conn: &mut diesel::SqliteConnection) -> Result<Self, diesel::result::Error> {
    use model::endpoint::jobs::{
      JobCLT,
      NewJobCLT,
      UpsertJobCLT,
    };
    use schema::endpoint::jobs;
    diesel::insert_into(jobs::table)
      .values(NewJobCLT::from(self.clone()))
      .on_conflict(jobs::id)
      .do_update()
      .set(UpsertJobCLT::from(self.clone()))
      .execute(conn)?;
    jobs::table
      .find(self.id.clone())
      .get_result::<JobCLT>(conn)
      .map(|job| job.into())
  }
}

impl super::SrvDbOperator for Job {
  fn create_srv(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error> {
    use model::server::jobs::{
      JobSRV,
      NewJobSRV,
    };
    use schema::server::jobs;
    match diesel::insert_into(jobs::table)
      .values(NewJobSRV::from(self.clone()))
      .get_result::<JobSRV>(conn)
    {
      Ok(job) => Ok(job.into()),
      Err(e) => Err(e),
    }
  }
  fn update_srv(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error> {
    use model::server::jobs::{
      JobSRV,
      UpdateJobSRV,
    };
    use schema::server::jobs;
    match diesel::update(jobs::table.find(self.id.clone()))
      .set(UpdateJobSRV::from(self.clone()))
      .get_result::<JobSRV>(conn)
    {
      Ok(job) => Ok(job.into()),
      Err(e) => Err(e),
    }
  }
  fn delete_srv(&self, conn: &mut diesel::PgConnection) -> Result<(), diesel::result::Error> {
    use schema::server::jobs;
    match diesel::delete(jobs::table.find(self.id.clone())).execute(conn) {
      Ok(_) => Ok(()),
      Err(e) => Err(e),
    }
  }
  fn read_srv(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error> {
    use model::server::jobs::JobSRV;
    use schema::server::jobs;
    match jobs::table.find(self.id.clone()).get_result::<JobSRV>(conn) {
      Ok(job) => Ok(job.into()),
      Err(e) => Err(e),
    }
  }
  fn upsert_srv(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error> {
    use model::server::jobs::{
      JobSRV,
      NewJobSRV,
      UpsertJobSRV,
    };
    use schema::server::jobs;
    diesel::insert_into(jobs::table)
      .values(NewJobSRV::from(self.clone()))
      .on_conflict(jobs::id)
      .do_update()
      .set(UpsertJobSRV::from(self.clone()))
      .execute(conn)?;
    jobs::table
      .find(self.id.clone())
      .get_result::<JobSRV>(conn)
      .map(|job| job.into())
  }
}
