use std::{
  cmp::Ordering,
  collections::BinaryHeap,
  sync::Arc,
  time::Duration,
};

use remex_core::db::model::{
  executions::ExecutionStatus,
  groups::Group,
  jobs::{
    Enabled,
    Job,
    JobType,
  },
};
use surrealdb::{
  engine::local::Db,
  types::{
    Action,
    RecordId,
    ToSql,
  },
  Surreal,
};
use tokio::{
  sync::Mutex,
  time::{
    sleep,
    Instant,
  },
};
use tokio_stream::StreamExt;

use crate::{
  db::{
    get_local_remex,
    get_remote_remex,
  },
  ConnState,
};

#[derive(Debug, Clone)]
pub enum JobQueueMessage {
  Immediate {
    job: Job,
    client_id: String,
  },
  Scheduled {
    job: Job,
    execution_time: Instant,
    client_id: String,
  },
  Remove {
    id: RecordId,
  },
  SyncFromRemote,
}

#[derive(Debug, Clone)]
struct ScheduledJob {
  execution_time: Instant,
  job: Job,
  client_id: String,
}

impl Ord for ScheduledJob {
  fn cmp(&self, other: &Self) -> Ordering { other.execution_time.cmp(&self.execution_time) }
}

impl PartialOrd for ScheduledJob {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}

impl PartialEq for ScheduledJob {
  fn eq(&self, other: &Self) -> bool {
    self.execution_time == other.execution_time && self.job.id == other.job.id
  }
}

impl Eq for ScheduledJob {
}

pub async fn job_scheduler_loop(
  mut rx: tokio::sync::mpsc::Receiver<JobQueueMessage>,
) -> Result<(), crate::Error> {
  let mut heap: BinaryHeap<ScheduledJob> = BinaryHeap::new();

  loop {
    let now = Instant::now();

    let next_fire = heap
      .peek()
      .map(|t| t.execution_time.saturating_duration_since(now));

    let sleep_duration = next_fire.unwrap_or(Duration::from_secs(3600 * 24 * 365));
    tracing::debug!("Queue state: heap_size={}, next_fire={:?}", heap.len(), next_fire);
    tracing::debug!("Sleeping for {:?}", sleep_duration);
    let sleep_fut = sleep(sleep_duration);

    tokio::select! {
      Some(injection) = rx.recv() => {
      match injection {
        JobQueueMessage::Immediate { job, client_id } => {
          tracing::debug!("Immediate job received: {}", job.job_name);
          let job_id = job.id.clone();
          let job_clone = job.clone();
          let client_id_clone = client_id.clone();
          tokio::spawn(async move {
            if should_skip_job(&job_id).await {
              tracing::debug!("Skipping immediate job {} (recent execution exists)", job_clone.job_name);
              return;
            }
            if let Err(e) = execute_job(job_clone, &client_id_clone).await {
              tracing::error!("Job execution failed Immediate: {}", e);
            }
          });
        }
        JobQueueMessage::Scheduled { job, execution_time, client_id } => {
          tracing::debug!("Scheduled job received: {} at {:?}", job.job_name, execution_time);
          if execution_time > now {
            tracing::debug!("Job queued: {} at {:?}", job.job_name, execution_time);
            heap.push(ScheduledJob {
              execution_time,
              job,
              client_id,
            });
          } else {
            let job_id = job.id.clone();
            let job_clone = job.clone();
            let client_id_clone = client_id.clone();
            tokio::spawn(async move {
              if should_skip_job(&job_id).await {
                tracing::debug!("Skipping scheduled job {} (recent execution exists)", job_clone.job_name);
                return;
              }
              if let Err(e) = execute_job(job_clone, &client_id_clone).await {
                tracing::error!("Job execution failed: {}", e);
              }
            });
          }
        }
      JobQueueMessage::Remove { id } => {
        println!("Removing job from queue: {}", id.to_sql());
        heap.retain(|j| j.job.id != id);
      }
        JobQueueMessage::SyncFromRemote => {
          println!("Clearing job queue for sync...");
          tracing::info!("Sync from remote requested, clearing job queue");
          heap.clear();
        }
      }
    }
      _ = sleep_fut, if next_fire.is_some() => {
        if let Some(scheduled) = heap.pop() {
          tracing::debug!("Scheduled job firing: {}", scheduled.job.job_name);
          let job_id = scheduled.job.id.clone();
          let job_clone = scheduled.job.clone();
          let client_id_clone = scheduled.client_id.clone();
          tokio::spawn(async move {
            if should_skip_job(&job_id).await {
              tracing::debug!("Skipping heap-fired job {} (recent execution exists)", job_clone.job_name);
              return;
            }
            if let Err(e) = execute_job(job_clone, &client_id_clone).await {
              tracing::error!("Job execution failed: {}", e);
            }
          });
        }
      }
    }
  }
}

pub async fn sync_groups(client_id: &str) -> Result<Vec<Group>, crate::Error> {
  println!("Syncing groups from remote...");
  tracing::info!("Syncing groups from remote database");

  let groups: Vec<Group> = get_remote_remex()
    .await?
    .query(format!("SELECT * FROM group WHERE members CONTAINS {client_id};"))
    .await?
    .check()?
    .take(0)?;

  tracing::debug!("Fetched {} groups from remote", groups.len());

  let id: surrealdb::types::RecordId = surrealdb::types::RecordId::parse_simple(client_id).unwrap();

  let mut grpmems: Vec<Group> = Vec::new();

  for group in groups {
    if !group.members.contains(&id) {
      tracing::debug!("Skipping group (not assigned): {}", group.group_name);
      continue;
    }

    grpmems.push(group);
  }

  Ok(grpmems)
}

pub async fn sync_and_refill_queue(
  job_injection_tx: &tokio::sync::mpsc::Sender<JobQueueMessage>,
  client_id: &str,
  groups: &[surrealdb::types::RecordId],
) -> Result<(), crate::Error> {
  use remex_core::db::DbOperator;

  println!("Syncing jobs from remote...");
  tracing::info!("Syncing jobs from remote database");

  let jobs: Vec<Job> = get_remote_remex()
    .await?
    .query("SELECT * FROM job;")
    .await?
    .check()?
    .take(0)?;

  tracing::debug!("Fetched {} jobs from remote", jobs.len());

  let id: surrealdb::types::RecordId = surrealdb::types::RecordId::parse_simple(client_id).unwrap();

  let mut queued_count = 0;
  for job in jobs {
    if !job.assignments.contains(&id) && !groups.iter().any(|g| job.assignments.contains(g)) {
      tracing::debug!("Skipping job (not assigned): {}", job.job_name);
      continue;
    }

    let job_id_str = job.id.to_sql();
    let local_db = get_local_remex().await?;

    // Check if job already exists in local cache
    let existing: Vec<crate::db::remex::JobCache> = match local_db
      .query(
        "USE NS remex DB remex; SELECT * FROM job WHERE job_id = $job_id LIMIT 1;"
      )
      .bind(("job_id", job_id_str.clone()))
      .await
    {
      Ok(res) => match res.check() {
        Ok(mut r) => r.take(1)?,
        Err(_) => vec![],
      },
      Err(_) => vec![],
    };

    let completed = if let Some(cached) = existing.first() {
      // Job exists locally — compare updated_at to detect offline modifications
      let local_updated = cached.job_info.updated_at;
      let remote_updated = job.updated_at;
      if local_updated != remote_updated {
        tracing::debug!(
          "Job {} was modified while offline, resetting completion status",
          job.job_name
        );
        false // Job changed, needs re-run
      } else {
        cached.completed // Preserve existing completion status
      }
    } else {
      false // New job, needs to run
    };

    // Upsert the job in local cache
    let cache_entry = crate::db::remex::JobCacheData {
      job_id: job_id_str.clone(),
      job_info: job.clone(),
      completed,
    };
    let _ = crate::db::remex::JobCache::create(cache_entry, &local_db).await;

    if let Some(exec_time) = calculate_execution_time(&job.job_type) {
      tracing::debug!("Injecting scheduled job: {} at {:?}", job.job_name, exec_time);
      let _ = job_injection_tx
        .send(JobQueueMessage::Scheduled {
          job,
          execution_time: exec_time,
          client_id: client_id.to_string(),
        })
        .await;
      queued_count += 1;
    } else {
      tracing::debug!("Injecting immediate job: {}", job.job_name);
      let _ = job_injection_tx
        .send(JobQueueMessage::Immediate {
          job,
          client_id: client_id.to_string(),
        })
        .await;
      queued_count += 1;
    }
  }

  println!("Synced {} jobs from remote", queued_count);
  tracing::info!("Queue refilled from remote database: {} jobs", queued_count);
  Ok(())
}

#[allow(dead_code)]
pub async fn clear_local_job_cache() {
  if let Err(e) = crate::db::LOCAL_DB
    .query("USE NS remex DB remex; DELETE job;")
    .await
  {
    tracing::error!("Failed to clear local job cache: {}", e);
  } else {
    tracing::info!("Local job cache cleared");
  }
}

pub async fn load_jobs_from_local_db(
  job_injection_tx: &tokio::sync::mpsc::Sender<JobQueueMessage>,
  client_id: &str,
) -> Result<(), crate::Error> {
  println!("Loading cached jobs from local database...");
  tracing::info!("Loading jobs from local database cache");

  let cached_jobs: Vec<crate::db::remex::JobCache> = match get_local_remex()
    .await?
    .query("USE NS remex DB remex; SELECT * FROM job;")
    .await
  {
    Ok(res) => match res.check() {
      Ok(mut r) => r.take(1)?,
      Err(e) => {
        tracing::warn!("Failed to check local jobs: {}", e);
        return Ok(());
      }
    },
    Err(e) => {
      tracing::warn!("Failed to query local jobs: {}", e);
      return Ok(());
    }
  };

  tracing::debug!("Found {} cached jobs", cached_jobs.len());

  let count = cached_jobs.len();
  for cached in cached_jobs {
    let job = cached.job_info;
    tracing::debug!("Loading job from cache: {}", job.job_name);
    if let Some(exec_time) = calculate_execution_time(&job.job_type) {
      let _ = job_injection_tx
        .send(JobQueueMessage::Scheduled {
          job,
          execution_time: exec_time,
          client_id: client_id.to_string(),
        })
        .await;
    } else {
      let _ = job_injection_tx
        .send(JobQueueMessage::Immediate {
          job,
          client_id: client_id.to_string(),
        })
        .await;
    }
  }

  println!("Loaded {} jobs from local cache", count);
  tracing::info!("Loaded {} jobs from local cache", count);
  Ok(())
}

// pub async fn refill_queue_from_remote(
//   job_injection_tx: &tokio::sync::mpsc::Sender<JobQueueMessage>,
//   client_id: &str,
//   groups: Vec<surrealdb::types::RecordId>,
// ) -> Result<(), crate::Error> {
//   println!("Refilling job queue from remote...");
//   tracing::info!("Refilling queue from remote database");
//
//   let _ = job_injection_tx.send(JobQueueMessage::SyncFromRemote).await;
//
//   tokio::time::sleep(Duration::from_millis(50)).await;
//
//   sync_and_refill_queue(job_injection_tx, client_id, groups).await
// }

/// Validates that the shell executable exists on the system.
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

/// Runs a command using the specified shell and returns (stdout+stderr, exit_status).
/// Applies an optional timeout.
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
      Err(_) => {
        return Err(crate::Error::CommandTimeout);
      }
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

async fn execute_job(job: Job, client_id: &str) -> Result<(), crate::Error> {
  use remex_core::db::{
    model::executions::{
      Execution,
      ExecutionStatus,
    },
    DbOperator,
  };

  use crate::db::remex::ExecutionCache;

  println!("Executing job: {}", job.job_name);
  tracing::info!("Executing job: {} on client {}", job.job_name, client_id);

  let time_start = surrealdb::types::Datetime::now();
  let client_id_record = surrealdb::types::RecordId::parse_simple(client_id)
    .map_err(|e| crate::Error::InvalidClientId(e.to_string()))?;

  // Convert timeout from SurrealDB Duration to std::time::Duration
  let timeout = job.timeout.as_ref().map(|d| {
    let secs = d.as_secs().max(1);
    Duration::from_secs(secs as u64)
  });

  // Step 1: Create execution record with Running status in local cache
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

  let mut cache_entry = crate::db::remex::ExecutionCacheData {
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

  // Step 2: Validate shell exists
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

  // Step 3: Execute command
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

  // Step 4: Update execution with final status
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

  // Step 5: If successful, mark the job as completed in JobCache
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

/// Marks a job as completed in the local JobCache.
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

/// Marks a job as not completed in the local JobCache (e.g., when the job is updated).
pub async fn mark_job_incomplete(job_id: &RecordId) -> Result<(), crate::Error> {
  let db = get_local_remex().await?;
  db.query(
    r"
      USE NS remex DB remex;
      LET $cached = (SELECT * FROM job WHERE job_id = $job_id LIMIT 1)[0];
      IF $cached != NONE {
        UPDATE $cached.id SET completed = false;
      };
    ",
  )
  .bind(("job_id", job_id.to_sql()))
  .await?
  .check()?;
  Ok(())
}

/// Background loop that syncs unsynced local executions to the remote database.
/// Runs every 30 seconds. Only pushes when remote DB is connected.
/// Also handles periodic cleanup of old synced executions (every 6 hours, tracked via last_action).
pub async fn execution_sync_loop(ctx: Arc<Mutex<crate::Context>>) -> Result<(), crate::Error> {
  use remex_core::db::DbOperator;

  const CLEANUP_INTERVAL_SECS: u64 = 6 * 3600; // 6 hours

  loop {
    tokio::time::sleep(Duration::from_secs(30)).await;

    let is_connected = {
      let ctx_lock = ctx.lock().await;
      ctx_lock.state.remote_db_connected == ConnState::Connected
    };

    if !is_connected {
      tracing::debug!("Remote DB not connected, skipping execution sync");
      continue;
    }

    let db = match get_local_remex().await {
      Ok(d) => d,
      Err(e) => {
        tracing::warn!("Failed to get local DB for execution sync: {}", e);
        continue;
      }
    };

    // Check if cleanup should run (every 6 hours, tracked via last_action table)
    match crate::db::last_action::LastAction::should_skip(
      &db,
      "cleanup_executions",
      CLEANUP_INTERVAL_SECS,
    )
    .await
    {
      Ok(false) => {
        // Cleanup hasn't run in 6+ hours, run it
        if let Err(e) = cleanup_old_executions(&db).await {
          tracing::warn!("Execution cleanup failed: {}", e);
        } else {
          if let Err(e) =
            crate::db::last_action::LastAction::record(&db, "cleanup_executions").await
          {
            tracing::warn!("Failed to record cleanup timestamp: {}", e);
          }
          // Also purge old last_action records
          if let Err(e) = crate::db::last_action::LastAction::cleanup_old(&db).await {
            tracing::warn!("Failed to purge old last_action records: {}", e);
          }
        }
      }
      Ok(true) => {
        // tracing::debug!("Execution cleanup already ran recently, skipping");
      }
      Err(e) => {
        tracing::warn!("Failed to check last_action for cleanup: {}", e);
      }
    }

    // Query unsynced executions
    let unsynced: Vec<crate::db::remex::ExecutionCache> = match db
      .query("USE NS remex DB remex; SELECT * FROM execution WHERE synced = false;")
      .await
    {
      Ok(res) => match res.check() {
        Ok(mut r) => r.take(1)?,
        Err(e) => {
          tracing::warn!("Failed to query unsynced executions: {}", e);
          continue;
        }
      },
      Err(e) => {
        tracing::warn!("Failed to query unsynced executions: {}", e);
        continue;
      }
    };

    if unsynced.is_empty() {
      continue;
    }

    tracing::info!("Syncing {} unsynced executions to remote", unsynced.len());

    let remote_db = match get_remote_remex().await {
      Ok(d) => d,
      Err(e) => {
        tracing::warn!("Failed to get remote DB for execution sync: {}", e);
        continue;
      }
    };

    for entry in unsynced {
      let exec = entry.execution_info.clone();
      let exec_id = entry.execution_id.clone();

      // Mark as synced BEFORE pushing to remote to prevent duplicate syncs
      let mut synced_entry = entry;
      synced_entry.synced = true;
      if let Err(e) = synced_entry.push(&db).await {
        tracing::warn!("Failed to mark execution as synced before push: {}", e);
        continue;
      }

      // Push execution to remote DB
      match remote_db
        .query("CREATE execution CONTENT $data")
        .bind(("data", exec))
        .await
      {
        Ok(result) => match result.check() {
          Ok(_) => {
            tracing::debug!("Synced execution: {}", exec_id);
          }
          Err(e) => {
            tracing::warn!("Failed to push execution to remote, reverting synced flag: {}", e);
            // Revert synced flag so it will be retried next cycle
            let mut reverted = synced_entry;
            reverted.synced = false;
            let _ = reverted.push(&db).await;
          }
        },
        Err(e) => {
          tracing::warn!("Failed to push execution to remote, reverting synced flag: {}", e);
          // Revert synced flag so it will be retried next cycle
          let mut reverted = synced_entry;
          reverted.synced = false;
          let _ = reverted.push(&db).await;
        }
      }
    }
  }
}

/// Deletes synced executions older than 7 days to prevent unbounded storage growth.
async fn cleanup_old_executions(db: &Surreal<Db>) -> Result<(), crate::Error> {
  let result = db
    .query(
      "USE NS remex DB remex; DELETE execution WHERE synced = true AND created_at < time::now() - 7d;"
    )
    .await?
    .check()?;

  tracing::info!("Execution cleanup completed: {:?}", result);
  Ok(())
}

/// Checks if a job has been completed in the local JobCache to prevent duplicate runs.
/// Returns true if the job should be skipped.
pub async fn should_skip_job(job_id: &RecordId) -> bool {
  let db = match get_local_remex().await {
    Ok(d) => d,
    Err(_) => return false,
  };

  // Check JobCache for completed status
  let cached: Vec<crate::db::remex::JobCache> = match db
    .query("USE NS remex DB remex; SELECT * FROM job WHERE job_id = $job_id LIMIT 1;")
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

  // Skip only if the job is marked as completed
  cached.first().map(|c| c.completed).unwrap_or(false)
}

async fn execute_job_old(job: Job) -> Result<(), crate::Error> {
  println!("Executing job: {}", job.job_name);

  let _remote_db = get_remote_remex().await?;
  tracing::debug!("Running command: {} -c {}", job.job_shell, job.job_command);
  let output = tokio::process::Command::new("echo")
    .arg(&job.job_command)
    .output()
    .await;

  match output {
    Ok(out) => {
      tracing::debug!(
        "Command exit code: {:?}, stdout: {} , stderr: {} ",
        out.status.code(),
        String::from_utf8_lossy(out.stdout.as_slice()),
        String::from_utf8_lossy(out.stderr.as_slice())
      );
      if out.status.success() {
        println!("Job completed: {}", job.job_name);
        tracing::info!("Job completed successfully: {}", job.job_name);
      } else {
        let error_msg = String::from_utf8_lossy(&out.stderr).to_string();
        println!("Job FAILED: {} - {}", job.job_name, error_msg);
        tracing::error!("Job failed: {}", error_msg);
      }
    }
    Err(e) => {
      tracing::debug!("Job execution error: {}", e);
    }
  }

  Ok(())
}

fn calculate_execution_time(job_type: &JobType) -> Option<Instant> {
  match job_type {
    JobType::Instant => None,
    JobType::Scheduled(dt) => {
      let datetime: chrono::DateTime<chrono::Utc> = (*dt).into();
      let duration = datetime.signed_duration_since(chrono::Utc::now());
      let millis = duration.num_milliseconds();
      if millis > 0 {
        Some(Instant::now() + Duration::from_millis(millis as u64))
      } else {
        Some(Instant::now())
      }
    }
    JobType::Recurring(dt, _interval) => {
      let datetime: chrono::DateTime<chrono::Utc> = (*dt).into();
      let duration = datetime.signed_duration_since(chrono::Utc::now());
      let millis = duration.num_milliseconds();
      if millis > 0 {
        Some(Instant::now() + Duration::from_millis(millis as u64))
      } else {
        Some(Instant::now())
      }
    }
  }
}

pub async fn monitor_jobs(
  ctx: Arc<Mutex<crate::Context>>,
  job_injection_tx: tokio::sync::mpsc::Sender<JobQueueMessage>,
) -> Result<(), crate::Error> {
  println!("Starting job monitor...");

  let mut initial_sync_done = false;

  loop {
    let is_connected = {
      let ctx_lock = ctx.lock().await;
      ctx_lock.state.remote_db_connected == ConnState::Connected
    };

    if !is_connected {
      tracing::debug!("Remote DB not connected, loading jobs from local cache");
      let client_id = {
        let ctx_lock = ctx.lock().await;
        ctx_lock.session.client_id.clone().unwrap_or_default()
      };
      if let Err(e) = load_jobs_from_local_db(&job_injection_tx, &client_id).await {
        tracing::warn!("Failed to load from local cache: {}", e);
      }
      tokio::time::sleep(Duration::from_secs(5)).await;
      continue;
    }

    let remote_db = crate::db::get_remote_remex().await?;
    println!("Setting up live query for job changes...");
    tracing::debug!("Creating live query stream for jobs");

    let client_id = {
      let ctx_lock = ctx.lock().await;
      ctx_lock.session.client_id.clone().unwrap()
    };

    let mut groups = {
      let ctx_lock = ctx.lock().await;
      ctx_lock.session.groups.clone()
    };

    if !initial_sync_done {
      tracing::info!("First connection to remote, syncing jobs from remote");
      match sync_groups(&client_id).await {
        Ok(g) => {
          tracing::debug!("Synced {} groups from remote", g.len());
          {
            let mut ctx_lock = ctx.lock().await;
            g.iter()
              .for_each(|g| ctx_lock.session.groups.push(g.id.clone()));
            groups = g.iter().map(|g| g.id.clone()).collect();
          };
          if let Err(e) = sync_and_refill_queue(&job_injection_tx, &client_id, &groups).await {
            tracing::warn!("Failed to sync from remote: {}", e);
          } else {
            initial_sync_done = true;
          }
        }
        Err(e) => {
          tracing::warn!("Failed to sync groups from remote: {}", e);
        }
      }
    }

    let mut stream = remote_db.select::<Vec<Job>>("job").live().await?;
    let mut groupstream = remote_db.select::<Vec<Group>>("group").live().await?;

    tracing::info!("Monitoring jobs loop starting");
    loop {
      tokio::select! {
        notification = stream.next() => {
          tracing::debug!("Job notification received");
          match notification {
            Some(Ok(notification)) => {
              let id: surrealdb::types::RecordId =
              surrealdb::types::RecordId::parse_simple(&client_id).unwrap();

              if !notification.data.assignments.contains(&id) &&  !notification.data.assignments.iter().any(|g| groups.contains(g)) {
                tracing::debug!("Job {} not assigned to this client, skipping", notification.data.job_name);
                continue;
              }
              match notification.action {
                Action::Create => {
                  use remex_core::db::DbOperator;
                  tracing::debug!("Job created: {:#?}", notification.data.job_name);
                  let job = notification.data.clone();
                  let cid = client_id.clone();
                  let job_id = job.id.clone();

                  // Check if job already exists in local cache (e.g., from re-sync)
                  let db = match get_local_remex().await {
                    Ok(d) => d,
                    Err(e) => {
                      tracing::warn!("Failed to get local DB for job cache: {}", e);
                      return Ok(());
                    }
                  };
                  let existing: Vec<crate::db::remex::JobCache> = match db
                    .query(
                      "USE NS remex DB remex; SELECT * FROM job WHERE job_id = $job_id LIMIT 1;"
                    )
                    .bind(("job_id", job_id.to_sql()))
                    .await
                  {
                    Ok(res) => match res.check() {
                      Ok(mut r) => r.take(1)?,
                      Err(_) => vec![],
                    },
                    Err(_) => vec![],
                  };

                  if existing.is_empty() {
                    // New job, cache locally with completed = false
                    let job_id_str = job_id.to_sql();
                    let cache_entry = crate::db::remex::JobCacheData {
                      job_id: job_id_str.clone(),
                      job_info: job.clone(),
                      completed: false,
                    };
                    let _ = crate::db::remex::JobCache::create(cache_entry, &db).await;
                  }

                  // Mark as incomplete in local cache (new job needs to run)
                  let _ = mark_job_incomplete(&job_id).await;

                  if let Some(exec_time) = calculate_execution_time(&job.job_type) {
                    let _ = job_injection_tx.send(JobQueueMessage::Scheduled {
                      job,
                      execution_time: exec_time,
                      client_id: cid,
                    }).await;
                  } else {
                    let _ = job_injection_tx.send(JobQueueMessage::Immediate {
                      job,
                      client_id: cid,
                    }).await;
                  }
                }
                Action::Update => {
                  use remex_core::db::DbOperator;
                  println!("Job updated in remote: {}", notification.data.job_name);
                  tracing::debug!("Job updated: {}", notification.data.job_name);

                  let job_id = notification.data.id.clone();
                  let updated_job = notification.data.clone();

                  // Update the cached job info and mark as incomplete
                  let db = match get_local_remex().await {
                    Ok(d) => d,
                    Err(e) => {
                      tracing::warn!("Failed to get local DB for job cache: {}", e);
                      return Ok(());
                    }
                  };
                  let existing: Vec<crate::db::remex::JobCache> = match db
                    .query(
                      "USE NS remex DB remex; SELECT * FROM job WHERE job_id = $job_id LIMIT 1;"
                    )
                    .bind(("job_id", job_id.to_sql()))
                    .await
                  {
                    Ok(res) => match res.check() {
                      Ok(mut r) => r.take(1)?,
                      Err(_) => vec![],
                    },
                    Err(_) => vec![],
                  };

                  if let Some(cached) = existing.first() {
                    // Update the cached job info with new data, reset completed
                    let mut updated = cached.clone();
                    updated.job_info = updated_job.clone();
                    updated.completed = false;
                    let _ = updated.push(&db).await;
                  } else {
                    // Job wasn't cached yet, create it
                    let cache_entry = crate::db::remex::JobCacheData {
                      job_id: job_id.to_sql(),
                      job_info: updated_job.clone(),
                      completed: false,
                    };
                    let _ = crate::db::remex::JobCache::create(cache_entry, &db).await;
                  }

                  if notification.data.enabled == Enabled::Enabled {
                    let _ = job_injection_tx
                      .send(JobQueueMessage::Remove {
                        id: notification.data.id.clone(),
                      })
                      .await;

                    let job = notification.data.clone();
                    let cid = client_id.clone();
                    if let Some(exec_time) = calculate_execution_time(&job.job_type) {
                      let _ = job_injection_tx.send(JobQueueMessage::Scheduled {
                        job,
                        execution_time: exec_time,
                        client_id: cid,
                      }).await;
                    } else {
                      let _ = job_injection_tx.send(JobQueueMessage::Immediate {
                        job,
                        client_id: cid,
                      }).await;
                    }

                  }
                }
                Action::Delete | Action::Killed => {
                  println!("Job removed from remote: {}", notification.data.job_name);
                  tracing::debug!("Job removed from remote: {}", notification.data.job_name);
                  let _ = job_injection_tx
                    .send(JobQueueMessage::Remove {
                      id: notification.data.id.clone(),
                    })
                    .await;
                }
              }
            }
            Some(Err(err)) => {
              tracing::error!("Error: {:#?}", err);
            }
            None => {
              tracing::warn!("Job notification stream ended, recreating");
              break;
            }
          }
        }
        group_notification = groupstream.next() => {
          tracing::debug!("Group notification received");
          match group_notification {
            Some(Ok(notification)) => {
              match notification.action {
                Action::Create => {
                  println!("Group created in remote: {}", notification.data.group_name);
                  tracing::debug!("Group created: {}", notification.data.group_name);
                  if !notification.data.members.contains(&surrealdb::types::RecordId::parse_simple(&client_id).unwrap()) {
                    tracing::debug!("Group {} not assigned to this client, skipping", notification.data.group_name);
                    continue;
                  }
                  {
                    let mut ctx_lock = ctx.lock().await;
                    ctx_lock.session.groups.push(notification.data.id.clone());
                  }
                  groups.push(notification.data.id.clone());
                }
                Action::Update => {
                  println!("Group updated in remote: {}", notification.data.group_name);
                  tracing::debug!("Group updated: {}", notification.data.group_name);
                  if !notification.data.members.contains(&surrealdb::types::RecordId::parse_simple(&client_id).unwrap()) {
                    tracing::debug!("Group {} not assigned to this client, skipping", notification.data.group_name);
                    {
                      let mut ctx_lock = ctx.lock().await;
                      ctx_lock.session.groups.retain(|g| g != &notification.data.id);
                    }
                    groups.retain(|g| g != &notification.data.id);
                    continue;
                  }
                  {
                    let mut ctx_lock = ctx.lock().await;
                    ctx_lock.session.groups.retain(|g| g != &notification.data.id);
                    ctx_lock.session.groups.push(notification.data.id.clone());
                  }
                  groups.retain(|g| g != &notification.data.id);
                  groups.push(notification.data.id.clone());
                }
                Action::Delete => {
                  println!("Group removed from remote: {}", notification.data.group_name);
                  tracing::debug!("Group removed from remote: {}", notification.data.group_name);
                  {
                    let mut ctx_lock = ctx.lock().await;
                    ctx_lock.session.groups.retain(|g| g != &notification.data.id);
                  }
                  groups.retain(|g| g != &notification.data.id);
                }
                Action::Killed => {
                  println!("Group removed from remote: {}", notification.data.group_name);
                  tracing::debug!("Group removed from remote: {}", notification.data.group_name);
                  {
                    let mut ctx_lock = ctx.lock().await;
                    ctx_lock.session.groups.retain(|g| g != &notification.data.id);
                  }
                  groups.retain(|g| g != &notification.data.id);
                }
              }
              println!("Group updated in remote: {}", notification.data.group_name);
              tracing::debug!("Group updated: {}", notification.data.group_name);

              if let Err(e) = sync_and_refill_queue(&job_injection_tx, &client_id, &groups).await {
                tracing::warn!("Failed to sync from remote: {}", e);
              }
            }
            Some(Err(err)) => {
              tracing::error!("Error: {:#?}", err);
            }
            None => {
              tracing::warn!("Group notification stream ended, recreating");
              break;
            }
          }
        }
      }
    }
    tokio::time::sleep(Duration::from_secs(2)).await;
  }
}
