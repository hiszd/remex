use serde::{
  Deserialize,
  Serialize,
};
use surrealdb_types::SurrealValue;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, SurrealValue, ToSchema)]
pub enum JobType {
  Instant,
  Scheduled(String),
  Recurring(String, String),
}

impl JobType {
  pub fn options() -> String {
    let options = [
      serde_json::to_string(&JobType::Instant).unwrap(),
      serde_json::to_string(&JobType::Scheduled(chrono::Utc::now().to_rfc3339())).unwrap(),
      serde_json::to_string(&JobType::Recurring(
        chrono::Utc::now().to_rfc3339(),
        "60s".to_string(),
      ))
      .unwrap(),
    ];
    options.join(", ")
  }
}

impl From<String> for JobType {
  fn from(s: String) -> Self {
    match serde_json::from_str(&s) {
      Ok(v) => v,
      Err(e) => {
        tracing::info!("Failed to parse job type: {}", s);
        tracing::info!("Options: {}", JobType::options());
        panic!("{}", e);
      }
    }
  }
}

impl From<JobType> for String {
  fn from(jt: JobType) -> Self { serde_json::to_string(&jt).unwrap() }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, SurrealValue, ToSchema)]
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

impl From<String> for JobStatus {
  fn from(status: String) -> Self {
    match serde_json::from_str(&status) {
      Ok(v) => v,
      Err(e) => {
        tracing::info!("Failed to parse job status: {}", status);
        panic!("{}", e);
      }
    }
  }
}

impl From<JobStatus> for String {
  fn from(val: JobStatus) -> Self { serde_json::to_string(&val).unwrap() }
}

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue, ToSchema)]
pub struct Job {
  pub id: Option<String>,
  pub job_name: String,
  pub job_type: JobType,
  pub job_status: JobStatus,
  pub job_shell: String,
  pub job_command: String,
  pub created_at: Option<String>,
  pub updated_at: Option<String>,
}

impl Job {
  pub fn new(
    job_name: String,
    job_type: JobType,
    job_status: JobStatus,
    job_shell: String,
    job_command: String,
  ) -> Self {
    let now = chrono::Utc::now().to_rfc3339();
    Job {
      id: None,
      job_name,
      job_type,
      job_status,
      job_shell,
      job_command,
      created_at: Some(now.clone()),
      updated_at: Some(now),
    }
  }
}
