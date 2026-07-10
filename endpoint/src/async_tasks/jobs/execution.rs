use std::time::Duration;

use remex_core::db::{
  model::executions::ExecutionStatus,
  DbOperator,
};

#[derive(Debug, Clone)]
pub struct ExecutionResult {
  pub output: String,
  pub exit_code: String,
  pub execution_start: surrealdb::types::Datetime,
  pub execution_end: Option<surrealdb::types::Datetime>,
  pub job_id: surrealdb::types::RecordId,
  pub client_id: surrealdb::types::RecordId,
  pub status: ExecutionStatus,
}

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

  let output_fut = tokio::process::Command::new(shell)
    .arg("-c")
    .arg(cmd)
    .output();

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

async fn should_skip_job(
  job_id: &str,
  cache_repo: &dyn DbOperator<
    Record = crate::db::remex::JobCache,
    Input = crate::db::remex::JobCacheData,
  >,
) -> bool {
  let caches = match cache_repo.list().await {
    Ok(c) => c,
    Err(_) => return false,
  };
  caches
    .iter()
    .find(|c| c.job_id == job_id)
    .map(|c| c.completed)
    .unwrap_or(false)
}

pub(crate) async fn mark_job_completed(
  job_id: &str,
  cache_repo: &dyn DbOperator<
    Record = crate::db::remex::JobCache,
    Input = crate::db::remex::JobCacheData,
  >,
) -> Result<(), crate::Error> {
  let caches = cache_repo.list().await?;
  if let Some(cache) = caches.iter().find(|c| c.job_id == job_id) {
    let data = crate::db::remex::JobCacheData {
      job_id: cache.job_id.clone(),
      job_info: cache.job_info.clone(),
      completed: true,
    };
    cache_repo.update(&cache.cache_id(), data).await?;
  }
  Ok(())
}

pub async fn execute_job(
  job: remex_core::db::model::jobs::Job,
  client_id: &str,
) -> Result<Option<ExecutionResult>, crate::Error> {
  println!("Executing job: {}", job.job_name);
  tracing::info!("Executing job: {} on client {}", job.job_name, client_id);

  let time_start = surrealdb::types::Datetime::now();
  let client_id_record = surrealdb::types::RecordId::parse_simple(client_id)
    .map_err(|e| crate::Error::InvalidClientId(e.to_string()))?;

  let timeout = job.timeout.as_ref().map(|d| {
    let secs = d.as_secs().max(1);
    Duration::from_secs(secs)
  });

  if let Err(e) = validate_shell(&job.job_shell) {
    tracing::error!("Shell validation failed for job {}: {}", job.job_name, e);
    let time_end = surrealdb::types::Datetime::now();
    return Ok(Some(ExecutionResult {
      output: format!("Shell not found: {}\n{}", job.job_shell, e),
      exit_code: "127".to_string(),
      execution_start: time_start,
      execution_end: Some(time_end),
      job_id: job.id,
      client_id: client_id_record,
      status: ExecutionStatus::Failed,
    }));
  }

  let (output_str, exit_status) = match run_command(&job.job_shell, &job.job_command, timeout).await
  {
    Ok(result) => result,
    Err(crate::Error::CommandTimeout) => {
      tracing::warn!("Job {} timed out", job.job_name);
      let time_end = surrealdb::types::Datetime::now();
      return Ok(Some(ExecutionResult {
        output: format!("Command timed out after {:?}", timeout),
        exit_code: "-1".to_string(),
        execution_start: time_start,
        execution_end: Some(time_end),
        job_id: job.id,
        client_id: client_id_record,
        status: ExecutionStatus::TimedOut,
      }));
    }
    Err(e) => {
      tracing::error!("Command execution failed for job {}: {}", job.job_name, e);
      let time_end = surrealdb::types::Datetime::now();
      return Ok(Some(ExecutionResult {
        output: format!("Execution error: {}", e),
        exit_code: "1".to_string(),
        execution_start: time_start,
        execution_end: Some(time_end),
        job_id: job.id,
        client_id: client_id_record,
        status: ExecutionStatus::Failed,
      }));
    }
  };

  let is_completed = exit_status.success();
  let execution_status = if is_completed {
    ExecutionStatus::Completed
  } else {
    ExecutionStatus::Failed
  };

  let time_end = surrealdb::types::Datetime::now();

  tracing::info!("Job {} completed with status: {:?}", job.job_name, execution_status);

  Ok(Some(ExecutionResult {
    output: output_str,
    exit_code: exit_status.code().unwrap_or(0).to_string(),
    execution_start: time_start,
    execution_end: Some(time_end),
    job_id: job.id,
    client_id: client_id_record,
    status: execution_status,
  }))
}

#[cfg(test)]
mod execution_tests {
  use remex_core::{
    db::DbOperator,
    impl_in_memory_db_operator,
  };

  use crate::db::remex::{
    JobCache,
    JobCacheData,
  };

  impl_in_memory_db_operator!(InMemoryJobCacheRepo, JobCache, JobCacheData, "job");

  /// The string format used by `RecordId::to_sql()` for a `RecordId::new("job", <key>)`.
  fn job_sql_id(key: &str) -> String { format!("job:{key}") }

  fn make_cache(job_key: &str, completed: bool) -> JobCacheData {
    use remex_core::db::model::jobs::{
      Enabled,
      ExecutionStatus,
      Job,
      JobType,
    };

    let job = Job {
      id: surrealdb::types::RecordId::new("job", job_key),
      job_name: format!("test-{job_key}"),
      job_shell: "/bin/sh".to_string(),
      job_command: "echo hi".to_string(),
      job_type: JobType::Instant,
      execution_status: ExecutionStatus::Pending,
      enabled: Enabled::Enabled,
      assignments: vec![],
      timeout: None,
      created_at: surrealdb::types::Datetime::default(),
      updated_at: surrealdb::types::Datetime::default(),
    };
    let job_id = format!("job:{job_key}");
    JobCacheData {
      job_id,
      job_info: job,
      completed,
    }
  }

  // ---- should_skip_job ----

  #[tokio::test]
  async fn skip_no_cache_returns_false() {
    let repo = InMemoryJobCacheRepo::new();
    let job_id = job_sql_id("no-cache");
    assert!(!super::should_skip_job(&job_id, &repo).await);
  }

  #[tokio::test]
  async fn skip_cache_completed_false_returns_false() {
    let repo = InMemoryJobCacheRepo::new();
    let job_id = job_sql_id("not-done");
    let _ = repo.create(make_cache("not-done", false)).await.unwrap();
    assert!(!super::should_skip_job(&job_id, &repo).await);
  }

  #[tokio::test]
  async fn skip_cache_completed_true_returns_true() {
    let repo = InMemoryJobCacheRepo::new();
    let job_id = job_sql_id("done");
    let _ = repo.create(make_cache("done", true)).await.unwrap();
    assert!(super::should_skip_job(&job_id, &repo).await);
  }

  #[tokio::test]
  async fn skip_other_cache_returns_false() {
    let repo = InMemoryJobCacheRepo::new();
    let _ = repo.create(make_cache("other", true)).await.unwrap();
    let job_id = job_sql_id("mine");
    assert!(!super::should_skip_job(&job_id, &repo).await);
  }

  // ---- mark_job_completed ----

  #[tokio::test]
  async fn mark_noop_when_cache_does_not_exist() {
    let repo = InMemoryJobCacheRepo::new();
    let job_id = job_sql_id("ghost");
    super::mark_job_completed(&job_id, &repo).await.unwrap();

    let all = repo.list().await.unwrap();
    assert!(all.is_empty(), "no cache should be created");
  }

  #[tokio::test]
  async fn mark_updates_existing_cache() {
    let repo = InMemoryJobCacheRepo::new();
    let job_id = job_sql_id("existing");
    let _ = repo.create(make_cache("existing", false)).await.unwrap();
    super::mark_job_completed(&job_id, &repo).await.unwrap();

    let all = repo.list().await.unwrap();
    assert_eq!(all.len(), 1);
    assert!(all[0].completed);
  }

  #[tokio::test]
  async fn mark_does_not_affect_other_caches() {
    let repo = InMemoryJobCacheRepo::new();
    let _ = repo.create(make_cache("other", true)).await.unwrap();
    let job_id = job_sql_id("target");
    let _ = repo.create(make_cache("target", false)).await.unwrap();
    super::mark_job_completed(&job_id, &repo).await.unwrap();

    let all = repo.list().await.unwrap();
    assert_eq!(all.len(), 2);
    for cache in &all {
      if cache.job_id == job_id {
        assert!(cache.completed);
      }
    }
  }

  #[tokio::test]
  async fn mark_idempotent_on_already_completed() {
    let repo = InMemoryJobCacheRepo::new();
    let job_id = job_sql_id("already-done");
    let _ = repo.create(make_cache("already-done", true)).await.unwrap();
    super::mark_job_completed(&job_id, &repo).await.unwrap();

    let all = repo.list().await.unwrap();
    assert_eq!(all.len(), 1);
    assert!(all[0].completed);
  }
}
