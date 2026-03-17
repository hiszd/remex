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

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobType {}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
  /// The job is created but not yet assigned to any client or is waiting for execution.
  Pending,
  /// The job is currently being executed by a client.
  Running,
  /// The job has finished successfully.
  Completed,
  /// The job has failed during execution, includes an error message.
  Failed(String),
  /// The job was cancelled by a user or system administrator.
  Cancelled,
  /// The job execution exceeded the allocated time limit.
  TimedOut,
  /// The job is disabled and will not be picked up for execution.
  Disabled,
}

impl From<String> for JobStatus {
  fn from(status: String) -> Self {
    match status.as_str() {
      "pending" => JobStatus::Pending,
      "running" => JobStatus::Running,
      "completed" => JobStatus::Completed,
      "failed" => JobStatus::Failed("".to_string()),
      "cancelled" => JobStatus::Cancelled,
      "timed_out" => JobStatus::TimedOut,
      "disabled" => JobStatus::Disabled,
      _ => JobStatus::Pending,
    }
  }
}
impl Into<(String, Option<String>)> for JobStatus {
  fn into(self) -> (String, Option<String>) {
    match self {
      JobStatus::Pending => ("pending".to_string(), None),
      JobStatus::Running => ("running".to_string(), None),
      JobStatus::Completed => ("completed".to_string(), None),
      JobStatus::Failed(f) => ("failed".to_string(), Some(f)),
      JobStatus::Cancelled => ("cancelled".to_string(), None),
      JobStatus::TimedOut => ("timed_out".to_string(), None),
      JobStatus::Disabled => ("disabled".to_string(), None),
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
  pub job_status: JobStatus,
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

impl From<model::server::jobs::JobSRV> for Job {
  fn from(job: model::server::jobs::JobSRV) -> Self {
    Job {
      id: job.id,
      job_name: job.job_name,
      job_type: job.job_type,
      job_status: job.job_status.into(),
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
      job_status: job.job_status.into(),
      job_shell: job.job_shell,
      job_command: job.job_command,
      created_at: job.created_at,
      updated_at: job.updated_at,
    }
  }
}
