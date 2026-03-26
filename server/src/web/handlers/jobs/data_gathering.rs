use serde::{
  Deserialize,
  Serialize,
};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClientJobStatusMetadata {
  pub latest_execution_id: Option<String>,
  pub latest_execution_timestamp: Option<String>,
  pub execution_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClientStatusSummary {
  pub client_id: String,
  pub client_name: String,
  pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct GroupJobStatusMetadata {
  pub client_statuses: Vec<ClientStatusSummary>,
  pub total_clients: i64,
  pub completed_clients: i64,
  pub failed_clients: i64,
  pub running_clients: i64,
}

pub fn get_client_job_status(
  _client_id: &str,
  _job_id: &str,
) -> Result<(String, ClientJobStatusMetadata), String> {
  // TODO: Implement client job status retrieval
  Ok(("pending".to_string(), ClientJobStatusMetadata {
    latest_execution_id: None,
    latest_execution_timestamp: None,
    execution_count: 0,
  }))
}

pub fn get_client_job_status_for_job(
  _job_id: &str,
) -> Result<Vec<(String, String, ClientJobStatusMetadata)>, String> {
  // TODO: Implement client job status for job
  Ok(vec![])
}

pub fn get_group_job_status(
  _group_id: &str,
  _job_id: &str,
) -> Result<(String, GroupJobStatusMetadata), String> {
  // TODO: Implement group job status
  Ok(("pending".to_string(), GroupJobStatusMetadata {
    client_statuses: vec![],
    total_clients: 0,
    completed_clients: 0,
    failed_clients: 0,
    running_clients: 0,
  }))
}
