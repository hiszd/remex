use diesel::prelude::*;
use remex_core::db::{
  model::server::executions::ExecutionSRV,
  schema::server::groups_clients,
};
use serde::{
  Deserialize,
  Serialize,
};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ClientJobStatusMetadata {
  pub latest_execution_id: Option<String>,
  pub latest_execution_timestamp: Option<chrono::NaiveDateTime>,
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
  conn: &mut PgConnection,
  client_id: &str,
  job_id: &str,
) -> Result<(String, ClientJobStatusMetadata), diesel::result::Error> {
  use remex_core::db::schema::server::executions;

  let latest_execution: Option<ExecutionSRV> = executions::table
    .filter(executions::client_id.eq(client_id))
    .filter(executions::job_id.eq(job_id))
    .order(executions::created_at.desc())
    .first(conn)
    .optional()?;

  let execution_count = executions::table
    .filter(executions::client_id.eq(client_id))
    .filter(executions::job_id.eq(job_id))
    .count()
    .get_result::<i64>(conn)?;

  match latest_execution {
    Some(execution) => {
      let status = if let Some(result) = execution.execution_result {
        if result.is_empty() {
          "completed".to_string()
        } else {
          result
        }
      } else {
        "running".to_string()
      };

      Ok((status, ClientJobStatusMetadata {
        latest_execution_id: Some(execution.id),
        latest_execution_timestamp: Some(execution.created_at),
        execution_count,
      }))
    }
    None => Ok(("pending".to_string(), ClientJobStatusMetadata {
      latest_execution_id: None,
      latest_execution_timestamp: None,
      execution_count: 0,
    })),
  }
}

pub fn get_client_job_status_for_job(
  conn: &mut PgConnection,
  job_id: &str,
) -> Result<Vec<(String, String, ClientJobStatusMetadata)>, diesel::result::Error> {
  use remex_core::db::schema::server::{
    clients,
    jobs_groups,
  };

  let job_clients = clients::table
    .inner_join(groups_clients::table.on(groups_clients::client_id.eq(clients::id.nullable())))
    .inner_join(jobs_groups::table.on(jobs_groups::group_id.eq(groups_clients::group_id)))
    .filter(jobs_groups::job_id.eq(job_id))
    .select((clients::id, clients::client_name))
    .distinct()
    .load::<(String, String)>(conn)?;

  let mut results = Vec::new();
  for (client_id, client_name) in job_clients {
    let (_status, metadata) = get_client_job_status(conn, &client_id, job_id)?;
    results.push((client_id, client_name, metadata));
  }

  Ok(results)
}

pub fn get_group_job_status(
  conn: &mut PgConnection,
  group_id: &str,
  job_id: &str,
) -> Result<(String, GroupJobStatusMetadata), diesel::result::Error> {
  use remex_core::db::schema::server::clients;

  let group_clients: Vec<String> = groups_clients::table
    .filter(groups_clients::group_id.eq(group_id))
    .select(groups_clients::client_id)
    .load::<Option<String>>(conn)?
    .into_iter()
    .flatten()
    .collect();

  let mut client_statuses = Vec::new();
  let mut completed_count: i64 = 0;
  let mut failed_count: i64 = 0;
  let mut running_count: i64 = 0;

  for client_id in group_clients {
    let (status, _metadata) = get_client_job_status(conn, &client_id, job_id)?;

    let client_name = clients::table
      .filter(clients::id.eq(&client_id))
      .select(clients::client_name)
      .first::<String>(conn)
      .unwrap_or_else(|_| "Unknown".to_string());

    client_statuses.push(ClientStatusSummary {
      client_id: client_id.clone(),
      client_name,
      status: status.clone(),
    });

    match status.as_str() {
      "completed" => completed_count += 1,
      "failed" => failed_count += 1,
      "running" => running_count += 1,
      _ => {}
    }
  }

  let total_clients_i64 = client_statuses.len() as i64;
  let overall_status = if failed_count > 0 {
    "failed"
  } else if running_count > 0 {
    "running"
  } else if completed_count == total_clients_i64 && total_clients_i64 > 0 {
    "completed"
  } else {
    "pending"
  }
  .to_string();

  Ok((overall_status, GroupJobStatusMetadata {
    client_statuses,
    total_clients: total_clients_i64,
    completed_clients: completed_count,
    failed_clients: failed_count,
    running_clients: running_count,
  }))
}
