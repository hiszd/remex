use std::time::Duration;

use remex_core::db::{
  model::executions::{Execution, ExecutionStatus},
  LegacyDbOperator,
};
use surrealdb::{engine::local::Db, types::RecordId, Surreal};
use surrealdb::types::ToSql;

use crate::db::{remex::ExecutionCache, get_local_remex};

fn validate_shell(shell: &str) -> Result<(), crate::Error> {
  use std::path::Path;
  let path = Path::new(shell);
  if !path.exists() {
    return Err(crate::Error::ShellNotFound(shell.to_string()));
  }
  if !path.is_file() {
    return Err(crate::Error::ShellNotFound(shell.to_string()));
  }
  Ok(())
}

async fn run_command(
  shell: &str,
  cmd: &str,
  timeout: Option<Duration>,
) -> Result<(String, std::process::ExitStatus), crate::Error> {
  println!("Running command: {} -c {}", shell, cmd);
  tracing::debug!("Running command: {} -c {}", shell, cmd);

  let output_fut = tokio::process::Command::new(shell).arg("-c").arg(cmd).output();

  let result = if let Some(dur) = timeout {
    match tokio::time::timeout(dur, output_fut).await {
      Ok(Ok(output)) => output,
      Ok(Err(e)) => return Err(crate::Error::from(e)),
      Err(_) => return Err(crate::Error::CommandTimeout),
    }
  } else {
    output_fut.await?
  };

  let stdout = String::from_utf8_lossy(&result.stdout).to_string();
  let stderr = String::from_utf8_lossy(&result.stderr).to_string();
  tracing::debug!(
    "Command exit code: {:?}, stdout: {}, stderr: {}",
    result.status.code(),
    stdout,
    stderr
  );
  Ok((format!("out: {}\nerr: {}", stdout, stderr), result.status))
}

async fn should_skip_job(job_id: &RecordId) -> bool {
  let db = match get_local_remex().await {
    Ok(d) => d,
    Err(_) => return false,
  };

  let cached: Vec<crate::db::remex::JobCache> = match db
    .query(
      "USE NS remex DB remex; SELECT * FROM job WHERE job_id = $job_id LIMIT 1;",
    )
    .bind(("job_id", job_id.to_sql()))
    .await
  {
    Ok(res) => match res.check() {
      Ok(mut r) => match r.take(1) {
        Ok(v) => v,
        Err(_) => return false,
      },
      Err(_) => return false,
    },
    Err(_) => return false,
  };

  cached.first().map(|c| c.completed).unwrap_or(false)
}

async fn mark_job_completed(db: &Surreal<Db>, job_id: &RecordId) -> Result<(), crate::Error> {
  db.query(
    r"
      USE NS remex DB remex;
      LET $cached = (SELECT * FROM job WHERE job_id = $job_id LIMIT 1)[0];
      IF $cached != NONE {
        UPDATE $cached.id SET completed = true;
      };
    ",
  )
  .bind(("job_id", job_id.to_sql()))
  .await?
  .check()?;
  Ok(())
}

pub async fn execute_job(job: remex_core::db::model::jobs::Job, client_id: &str) -> Result<(), crate::Error> {
  if should_skip_job(&job.id).await {
    tracing::debug!("Skipping job {} (recent execution exists)", job.job_name);
    return Ok(());
  }

  println!("Executing job: {}", job.job_name);
  tracing::info!("Executing job: {} on client {}", job.job_name, client_id);

  let time_start = surrealdb::types::Datetime::now();
  let client_id_record = surrealdb::types::RecordId::parse_simple(client_id)
    .map_err(|e| crate::Error::InvalidClientId(e.to_string()))?;

  let timeout = job.timeout.as_ref().map(|d| {
    let secs = d.as_secs().max(1);
    Duration::from_secs(secs as u64)
  });

  let running_execution = Execution {
    id: surrealdb::types::RecordId::parse_simple(
      format!("execution:{}", uuid::Uuid::new_v4()).as_str(),
    )
    .unwrap(),
    job_id: Some(job.id.clone()),
    client_id: client_id_record.clone(),
    status: ExecutionStatus::Running,
    output: String::new(),
    command: job.job_command.clone(),
    exit_code: String::new(),
    execution_start: Some(time_start.clone()),
    execution_end: None,
    created_at: surrealdb::types::Datetime::now(),
    updated_at: surrealdb::types::Datetime::now(),
  };

  let cache_entry = crate::db::remex::ExecutionCacheData {
    execution_id: running_execution.id.to_sql(),
    execution_info: running_execution,
    synced: false,
  };

  let db = get_local_remex().await?;
  let created = ExecutionCache::create(cache_entry, &db)
    .await?
    .ok_or_else(|| {
      crate::Error::DbError(remex_core::db::DbError::OperationFailed(
        "Failed to create execution cache record".to_string(),
      ))
    })?;

  tracing::debug!("Created execution cache: {} with status Running", created.id.to_sql());

  if let Err(e) = validate_shell(&job.job_shell) {
    tracing::error!("Shell validation failed for job {}: {}", job.job_name, e);
    let time_end = surrealdb::types::Datetime::now();
    let mut completed = created;
    completed.execution_info.status = ExecutionStatus::Failed;
    completed.execution_info.output = format!("Shell not found: {}\n{}", job.job_shell, e);
    completed.execution_info.exit_code = "127".to_string();
    completed.execution_info.execution_end = Some(time_end);
    completed.execution_info.updated_at = surrealdb::types::Datetime::now();
    completed.push(&db).await?;
    return Err(e);
  }

  let (output_str, exit_status) = match run_command(&job.job_shell, &job.job_command, timeout).await
  {
    Ok(result) => result,
    Err(crate::Error::CommandTimeout) => {
      tracing::warn!("Job {} timed out", job.job_name);
      let time_end = surrealdb::types::Datetime::now();
      let mut completed = created;
      completed.execution_info.status = ExecutionStatus::TimedOut;
      completed.execution_info.output = format!("Command timed out after {:?}", timeout);
      completed.execution_info.exit_code = "-1".to_string();
      completed.execution_info.execution_end = Some(time_end);
      completed.execution_info.updated_at = surrealdb::types::Datetime::now();
      completed.push(&db).await?;
      return Ok(());
    }
    Err(e) => {
      tracing::error!("Command execution failed for job {}: {}", job.job_name, e);
      let time_end = surrealdb::types::Datetime::now();
      let mut completed = created;
      completed.execution_info.status = ExecutionStatus::Failed;
      completed.execution_info.output = format!("Execution error: {}", e);
      completed.execution_info.exit_code = "1".to_string();
      completed.execution_info.execution_end = Some(time_end);
      completed.execution_info.updated_at = surrealdb::types::Datetime::now();
      completed.push(&db).await?;
      return Err(e);
    }
  };

  let is_completed = exit_status.success();
  let execution_status = if is_completed {
    ExecutionStatus::Completed
  } else {
    ExecutionStatus::Failed
  };

  let time_end = surrealdb::types::Datetime::now();
  let mut completed = created;
  completed.execution_info.status = execution_status;
  completed.execution_info.output = output_str;
  completed.execution_info.exit_code = exit_status.code().unwrap_or(0).to_string();
  completed.execution_info.execution_end = Some(time_end);
  completed.execution_info.updated_at = surrealdb::types::Datetime::now();
  completed.push(&db).await?;

  if is_completed {
    if let Err(e) = mark_job_completed(&db, &job.id).await {
      tracing::warn!("Failed to mark job as completed in cache: {}", e);
    }
  }

  tracing::info!(
    "Job {} completed with status: {:?}",
    job.job_name,
    completed.execution_info.status
  );

  Ok(())
}
