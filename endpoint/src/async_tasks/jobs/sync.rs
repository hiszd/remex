use std::time::Duration;

use actix::prelude::*;
use remex_core::db::{
  model::{
    groups::Group,
    jobs::Job,
  },
  DbOperator,
};
use surrealdb::{
  engine::any::Any,
  types::{
    RecordId,
    ToSql,
  },
  Surreal,
};
use tokio::time::timeout;

use super::{
  JobQueueMessage,
  JobSender,
};
use crate::db::remex::{
  ExecutionCache,
  ExecutionCacheData,
  SurrealExecutionCacheRepo,
};

pub async fn sync_groups(
  client_id: &str,
  remote_db: &Surreal<Any>,
) -> Result<Vec<Group>, crate::Error> {
  println!("Syncing groups from remote...");
  tracing::info!("Syncing groups from remote database");

  let id =
    RecordId::parse_simple(client_id).map_err(|e| crate::Error::InvalidClientId(e.to_string()))?;

  tracing::info!("sync_groups: querying remote for groups containing {client_id}");
  let groups: Vec<Group> = match timeout(Duration::from_secs(3), async {
    remote_db
      .query("USE NS remex DB remex; SELECT * FROM group WHERE members CONTAINS $client_rid;")
      .bind(("client_rid", id.clone()))
      .await?
      .check()?
      .take(1)
  })
  .await
  {
    Ok(Ok(groups)) => groups,
    Ok(Err(e)) => {
      tracing::warn!("sync_groups: remote query failed: {e}");
      return Ok(Vec::new());
    }
    Err(_) => {
      tracing::warn!("sync_groups: remote query timed out after 3s");
      return Ok(Vec::new());
    }
  };

  tracing::info!("sync_groups: fetched {} groups from remote", groups.len());

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
  sender: &dyn JobSender,
  client_id: &str,
  groups: &[RecordId],
  remote_db: &Surreal<Any>,
) -> Result<(), crate::Error> {
  use crate::db::remex::{
    JobCache,
    SurrealJobCacheRepo,
  };

  tracing::info!("sync_and_refill_queue: querying remote for all jobs");
  let jobs: Vec<Job> = match timeout(Duration::from_secs(3), async {
    remote_db
      .query("USE NS remex DB remex; SELECT * FROM job;")
      .await?
      .check()?
      .take(1)
  })
  .await
  {
    Ok(Ok(jobs)) => jobs,
    Ok(Err(e)) => {
      tracing::warn!("sync_and_refill_queue: remote query failed: {e}");
      return Ok(());
    }
    Err(_) => {
      tracing::warn!("sync_and_refill_queue: remote query timed out after 3s");
      return Ok(());
    }
  };

  tracing::info!("sync_and_refill_queue: fetched {} jobs from remote", jobs.len());

  let id = RecordId::parse_simple(client_id).unwrap();
  let mut queued_count = 0;

  for job in jobs {
    if !job.assignments.contains(&id) && !groups.iter().any(|g| job.assignments.contains(g)) {
      tracing::debug!("Skipping job (not assigned): {}", job.job_name);
      continue;
    } else if job.enabled != remex_core::db::model::jobs::Enabled::Enabled {
      tracing::debug!("Skipping job (disabled): {}", job.job_name);
      continue;
    }

    tracing::info!("sync_and_refill_queue: processing job {}", job.job_name);
    let local_db = crate::db::get_local_remex().await?;

    let existing: Vec<JobCache> = match local_db
      .query("USE NS remex DB remex; SELECT * FROM job WHERE job_id = $job_id LIMIT 1;")
      .bind(("job_id", job.id.to_sql()))
      .await
    {
      Ok(res) => match res.check() {
        Ok(mut r) => r.take(1)?,
        Err(_) => vec![],
      },
      Err(_) => vec![],
    };

    let repo = SurrealJobCacheRepo {
      db: local_db.clone(),
    };
    if let Err(e) = sync_job_to_cache(&job, existing.first(), &repo).await {
      tracing::error!("Failed to sync job {} to local cache: {e}", job.job_name);
    }

    if let Some(exec_time) = super::calculate_execution_time(&job.job_type) {
      tracing::debug!("Injecting scheduled job: {} at {:?}", job.job_name, exec_time);
      if let Err(_) = sender
        .send_job(JobQueueMessage::Scheduled {
          job,
          execution_time: exec_time,
          client_id: client_id.to_string(),
        })
        .await
      {
        tracing::error!("Failed to inject scheduled job into queue");
      }
      queued_count += 1;
    } else {
      tracing::debug!("Injecting immediate job: {}", job.job_name);
      if let Err(_) = sender
        .send_job(JobQueueMessage::Immediate {
          job,
          client_id: client_id.to_string(),
        })
        .await
      {
        tracing::error!("Failed to inject immediate job into queue");
      }
      queued_count += 1;
    }
    tracing::info!("sync_and_refill_queue: finished processing job (queued_count={queued_count})");
  }

  tracing::info!("sync_and_refill_queue: done, queued {queued_count} jobs");
  tracing::info!("Queue refilled from remote database: {} jobs", queued_count);
  Ok(())
}

pub(crate) async fn sync_job_to_cache(
  job: &Job,
  existing_cache: Option<&crate::db::remex::JobCache>,
  cache_repo: &dyn DbOperator<
    Record = crate::db::remex::JobCache,
    Input = crate::db::remex::JobCacheData,
  >,
) -> Result<crate::db::remex::JobCache, remex_core::db::DbError> {
  let completed = match existing_cache {
    Some(cached) if cached.job_info.updated_at == job.updated_at => cached.completed,
    _ => false,
  };

  let cache_entry = crate::db::remex::JobCacheData {
    job_id: job.id.to_sql(),
    job_info: job.clone(),
    completed,
  };
  cache_repo.create(cache_entry).await
}

#[cfg(test)]
mod sync_tests {
  use remex_core::{
    db::{
      model::{
        executions::{
          Execution,
          ExecutionStatus as ExecStatus,
        },
        jobs::{
          Enabled,
          ExecutionStatus,
          Job,
          JobType,
        },
      },
      DbOperator,
    },
    impl_in_memory_db_operator,
  };
  use surrealdb::{
    engine::any::Any,
    types::ToSql,
    Surreal,
  };

  use crate::db::remex::{
    ExecutionCache,
    ExecutionCacheData,
    JobCache,
    JobCacheData,
  };

  impl_in_memory_db_operator!(InMemoryJobCacheRepo, JobCache, JobCacheData, "job");
  impl_in_memory_db_operator!(
    InMemoryExecutionCacheRepo,
    ExecutionCache,
    ExecutionCacheData,
    "execution"
  );

  fn make_test_job(id: &str, name: &str) -> Job {
    Job {
      id: surrealdb::types::RecordId::new("job", id),
      job_name: name.to_string(),
      job_shell: "/bin/sh".to_string(),
      job_command: "echo hello".to_string(),
      job_type: JobType::Instant,
      execution_status: ExecutionStatus::Pending,
      enabled: Enabled::Enabled,
      assignments: vec![],
      timeout: None,
      created_at: surrealdb::types::Datetime::default(),
      updated_at: surrealdb::types::Datetime::default(),
    }
  }

  #[tokio::test]
  async fn sync_new_job_creates_cache_with_completed_false() {
    let repo = InMemoryJobCacheRepo::new();
    let job = make_test_job("job-1", "test-job");

    let created = super::sync_job_to_cache(&job, None, &repo).await.unwrap();

    assert_eq!(created.job_id, job.id.to_sql());
    assert_eq!(created.job_info.job_name, "test-job");
    assert!(!created.completed, "new job should have completed = false");
  }

  #[tokio::test]
  async fn sync_unchanged_job_preserves_completed_status() {
    let repo = InMemoryJobCacheRepo::new();
    let job = make_test_job("job-2", "stable-job");

    let initial = repo
      .create(JobCacheData {
        job_id: job.id.to_sql(),
        job_info: job.clone(),
        completed: true,
      })
      .await
      .unwrap();

    let created = super::sync_job_to_cache(&job, Some(&initial), &repo)
      .await
      .unwrap();

    assert!(created.completed, "unchanged job should preserve completed = true");
  }

  #[tokio::test]
  async fn sync_changed_job_resets_completed_to_false() {
    let repo = InMemoryJobCacheRepo::new();
    let mut job = make_test_job("job-3", "changed-job");

    let initial = repo
      .create(JobCacheData {
        job_id: job.id.to_sql(),
        job_info: job.clone(),
        completed: true,
      })
      .await
      .unwrap();

    job.updated_at = surrealdb::types::Datetime::default();
    job.job_name = "updated-name".to_string();

    let created = super::sync_job_to_cache(&job, Some(&initial), &repo)
      .await
      .unwrap();

    assert!(!created.completed, "changed job should reset completed to false");
    assert_eq!(created.job_info.job_name, "updated-name");
  }

  #[tokio::test]
  async fn sync_missing_cache_treated_as_new_job() {
    let repo = InMemoryJobCacheRepo::new();
    let job = make_test_job("job-4", "no-cache-job");

    // Create an unrelated cache entry (different job_id) — should not affect this job
    let _other = repo
      .create(JobCacheData {
        job_id: "job:other".to_string(),
        job_info: make_test_job("other", "other"),
        completed: true,
      })
      .await
      .unwrap();

    let created = super::sync_job_to_cache(&job, None, &repo).await.unwrap();

    assert_eq!(created.job_id, job.id.to_sql());
    assert!(!created.completed, "missing cache should default to completed = false");
  }

  #[tokio::test]
  async fn sync_multiple_jobs_are_independent() {
    let repo = InMemoryJobCacheRepo::new();

    let job1 = make_test_job("batch-1", "alpha");
    let job2 = make_test_job("batch-2", "beta");

    let c1 = super::sync_job_to_cache(&job1, None, &repo).await.unwrap();
    let c2 = super::sync_job_to_cache(&job2, None, &repo).await.unwrap();

    assert_eq!(c1.job_id, job1.id.to_sql());
    assert_eq!(c2.job_id, job2.id.to_sql());
    assert_ne!(c1.cache_id(), c2.cache_id(), "each sync must create a separate cache entry");
  }

  // ── push_unsynced_executions tests ─────────────────────────────────────────

  fn make_test_execution(exec_key: &str, job_key: &str, client_key: &str) -> Execution {
    Execution {
      id: surrealdb::types::RecordId::new("execution", exec_key),
      job_id: Some(surrealdb::types::RecordId::new("job", job_key)),
      client_id: surrealdb::types::RecordId::new("client", client_key),
      status: ExecStatus::Completed,
      output: "test output".to_string(),
      command: "echo hi".to_string(),
      exit_code: "0".to_string(),
      execution_start: surrealdb::types::Datetime::default(),
      execution_end: Some(surrealdb::types::Datetime::default()),
      created_at: surrealdb::types::Datetime::default(),
      updated_at: surrealdb::types::Datetime::default(),
    }
  }

  fn make_execution_cache_entry(
    exec_key: &str,
    job_key: &str,
    client_key: &str,
    synced: bool,
  ) -> ExecutionCacheData {
    let exec = make_test_execution(exec_key, job_key, client_key);
    ExecutionCacheData {
      execution_id: format!("execution:{exec_key}"),
      execution_info: exec,
      synced,
    }
  }

  /// Helper to initialise an in-memory remote DB with the execution table
  async fn setup_memory_remote_db() -> Surreal<Any> {
    let remote: Surreal<Any> = Surreal::init();
    remote.connect("memory").await.unwrap();
    remote.use_ns("remex").use_db("remex").await.unwrap();
    remex_core::db::migrate(&remote).await.unwrap();
    remote
  }

  #[tokio::test]
  async fn push_empty_list_returns_zero() {
    let local_repo = InMemoryExecutionCacheRepo::new();
    let remote_db = setup_memory_remote_db().await;

    let count = super::push_unsynced_executions(vec![], &local_repo, &remote_db).await;
    assert_eq!(count, 0, "empty list should return 0");
  }

  #[tokio::test]
  async fn push_already_synced_is_skipped() {
    let local_repo = InMemoryExecutionCacheRepo::new();
    let remote_db = setup_memory_remote_db().await;

    let entry = local_repo
      .create(make_execution_cache_entry("skip-1", "job-1", "client-1", true))
      .await
      .unwrap();

    let count = super::push_unsynced_executions(vec![entry], &local_repo, &remote_db).await;
    assert_eq!(count, 0, "already-synced entry should be skipped");

    // Local state should still be synced=true
    let all = local_repo.list().await.unwrap();
    assert_eq!(all.len(), 1);
    assert!(all[0].synced, "already synced entry should remain synced");
  }

  #[tokio::test]
  async fn push_unsynced_to_remote_succeeds() {
    let local_repo = InMemoryExecutionCacheRepo::new();
    let remote_db = setup_memory_remote_db().await;

    // Create an unsynced execution in local cache
    let entry = local_repo
      .create(make_execution_cache_entry("push-ok", "job-1", "client-1", false))
      .await
      .unwrap();

    let count = super::push_unsynced_executions(vec![entry], &local_repo, &remote_db).await;
    assert_eq!(count, 1, "should have pushed 1 execution");

    // Local entry should now be synced
    let all = local_repo.list().await.unwrap();
    assert_eq!(all.len(), 1);
    assert!(all[0].synced, "entry should be marked synced after successful push");

    // Remote should have the execution record
    let remote_execs: Vec<serde_json::Value> = remote_db
      .query("SELECT * FROM execution;")
      .await
      .unwrap()
      .check()
      .unwrap()
      .take(0)
      .unwrap();
    assert_eq!(remote_execs.len(), 1, "remote should have 1 execution record");
  }

  #[tokio::test]
  async fn push_unsynced_mixed_entries() {
    let local_repo = InMemoryExecutionCacheRepo::new();
    let remote_db = setup_memory_remote_db().await;

    // Already synced entry
    let synced_entry = local_repo
      .create(make_execution_cache_entry("already", "job-1", "client-1", true))
      .await
      .unwrap();

    // Unsynced entry
    let unsynced_entry = local_repo
      .create(make_execution_cache_entry("fresh", "job-2", "client-2", false))
      .await
      .unwrap();

    let count =
      super::push_unsynced_executions(vec![synced_entry, unsynced_entry], &local_repo, &remote_db)
        .await;
    assert_eq!(count, 1, "should have pushed only 1 (the unsynced one)");

    // Both local entries should show synced=true (the already-synced was already, the fresh was pushed)
    let all = local_repo.list().await.unwrap();
    assert_eq!(all.len(), 2);
    for entry in &all {
      assert!(entry.synced, "all entries should be synced after push");
    }

    // Remote should have exactly 1 execution
    let remote_execs: Vec<serde_json::Value> = remote_db
      .query("SELECT * FROM execution;")
      .await
      .unwrap()
      .check()
      .unwrap()
      .take(0)
      .unwrap();
    assert_eq!(remote_execs.len(), 1, "remote should have 1 execution record");
  }

  #[tokio::test]
  async fn push_syncs_remote_and_marks_entry_synced() {
    let local_repo = InMemoryExecutionCacheRepo::new();
    let remote_db = setup_memory_remote_db().await;

    // Create an unsynced execution
    let entry = local_repo
      .create(make_execution_cache_entry("verify-sync", "job-1", "client-1", false))
      .await
      .unwrap();
    let count = super::push_unsynced_executions(vec![entry], &local_repo, &remote_db).await;
    assert_eq!(count, 1, "should have pushed 1 execution");

    // Local entry should be marked synced
    let all = local_repo.list().await.unwrap();
    assert_eq!(all.len(), 1);
    assert!(all[0].synced, "entry should be marked synced after successful push");

    // Remote should contain the execution with correct fields
    let remote_execs: Vec<serde_json::Value> = remote_db
      .query("SELECT * FROM execution;")
      .await
      .unwrap()
      .check()
      .unwrap()
      .take(0)
      .unwrap();
    assert_eq!(remote_execs.len(), 1, "remote should have the execution");

    // Remote execution should reference the client record
    let client_id = remote_execs[0]["client_id"].as_str().unwrap().to_string();
    assert!(
      client_id.contains("client-1"),
      "remote execution should reference correct client_id ({client_id})"
    );
  }
}

pub async fn full_sync(
  client_id: &str,
  sender: &dyn JobSender,
  remote_db: &Surreal<Any>,
) -> Result<(), crate::Error> {
  tracing::info!("full_sync: starting sync_groups for client {client_id}");
  let groups = sync_groups(client_id, remote_db).await?;
  tracing::info!("full_sync: sync_groups returned {} groups", groups.len());
  let group_ids: Vec<RecordId> = groups.iter().map(|g| g.id.clone()).collect();
  tracing::info!("full_sync: calling sync_and_refill_queue with {} groups", group_ids.len());
  let result = sync_and_refill_queue(sender, client_id, &group_ids, remote_db).await;
  tracing::info!("full_sync: sync_and_refill_queue completed");
  result
}

/// Push all unsynced execution records to the remote database.
///
/// For each entry in `entries`:
/// 1. Skips if already synced
/// 2. Sets `synced = true` on local cache (optimistic)
/// 3. Pushes `execution_info` to remote via `CREATE execution CONTENT $data`
/// 4. If remote push fails, reverts `synced = false` on local cache
///
/// Returns the number of executions successfully pushed to the remote.
/// Individual failures are logged but do not stop processing of other entries.
pub(crate) async fn push_unsynced_executions(
  entries: Vec<ExecutionCache>,
  local_repo: &dyn DbOperator<Record = ExecutionCache, Input = ExecutionCacheData>,
  remote_db: &Surreal<Any>,
) -> usize {
  if entries.is_empty() {
    return 0;
  }

  tracing::info!("Pushing {} unsynced executions to remote", entries.len());
  let mut pushed_count: usize = 0;

  for entry in entries {
    if entry.synced {
      continue;
    }

    let exec = entry.execution_info.clone();
    let exec_id = entry.execution_id.clone();
    let cache_id = entry.cache_id();

    // Optimistic: mark as synced before push
    let data = ExecutionCacheData {
      execution_id: exec_id.clone(),
      execution_info: exec.clone(),
      synced: true,
    };
    if let Err(e) = local_repo.update(&cache_id, data).await {
      tracing::warn!("Failed to mark execution {exec_id} as synced before push: {e}");
      continue;
    }

    // Push to remote
    let push_result = remote_db
      .query("CREATE execution CONTENT $data")
      .bind(("data", exec))
      .await;

    match push_result {
      Ok(result) => match result.check() {
        Ok(_) => {
          tracing::debug!("Synced execution: {exec_id}");
          pushed_count += 1;
        }
        Err(e) => {
          tracing::warn!(
            "Failed to push execution {exec_id} to remote, reverting synced flag: {e}"
          );
          let revert_data = ExecutionCacheData {
            execution_id: exec_id.clone(),
            execution_info: entry.execution_info.clone(),
            synced: false,
          };
          if let Err(revert_err) = local_repo.update(&cache_id, revert_data).await {
            tracing::error!("Failed to revert synced flag for execution {exec_id}: {revert_err}");
          }
        }
      },
      Err(e) => {
        tracing::warn!(
          "Failed to push execution {exec_id} to remote (transport), reverting synced flag: {e}"
        );
        let revert_data = ExecutionCacheData {
          execution_id: exec_id.clone(),
          execution_info: entry.execution_info.clone(),
          synced: false,
        };
        if let Err(revert_err) = local_repo.update(&cache_id, revert_data).await {
          tracing::error!("Failed to revert synced flag for execution {exec_id}: {revert_err}");
        }
      }
    }
  }

  pushed_count
}

async fn cleanup_old_executions() -> Result<(), crate::Error> {
  let db = crate::db::get_local_remex().await?;
  let result = db
    .query(
      "USE NS remex DB remex; DELETE execution WHERE synced = true AND created_at < time::now() - 7d;",
    )
    .await?
    .check()?;

  tracing::info!("Execution cleanup completed: {:?}", result);
  Ok(())
}

pub struct SyncActor {
  remote_db: Option<Surreal<Any>>,
}

impl SyncActor {
  pub fn new() -> Self { SyncActor { remote_db: None } }
}

#[derive(Message)]
#[rtype(result = "()")]
struct SyncTick;

impl Actor for SyncActor {
  type Context = Context<Self>;

  fn started(&mut self, ctx: &mut Self::Context) {
    ctx.notify_later(SyncTick, Duration::from_secs(30));
  }
}

impl actix::Supervised for SyncActor {
  fn restarting(&mut self, ctx: &mut Context<Self>) {
    tracing::info!("SyncActor: restarting");
    self.remote_db = None;
    // Re-schedule the sync tick (started() is not called on restart)
    ctx.notify_later(SyncTick, Duration::from_secs(30));
  }
}

impl Handler<crate::async_tasks::ConnectionReady> for SyncActor {
  type Result = ();

  fn handle(&mut self, msg: crate::async_tasks::ConnectionReady, _ctx: &mut Self::Context) {
    self.remote_db = msg.db;
    tracing::info!("Sync actor received connection (db={})", self.remote_db.is_some());
  }
}

impl Handler<SyncTick> for SyncActor {
  type Result = ();

  fn handle(&mut self, _msg: SyncTick, ctx: &mut Self::Context) {
    let remote_db = self.remote_db.clone();
    tokio::spawn(async move {
      const CLEANUP_INTERVAL_SECS: u64 = 6 * 3600;

      let remote_db = match remote_db {
        Some(db) => db,
        None => {
          tracing::debug!("Remote DB not connected, skipping execution sync");
          return;
        }
      };

      let db = match crate::db::get_local_remex().await {
        Ok(d) => d,
        Err(e) => {
          tracing::warn!("Failed to get local DB for execution sync: {}", e);
          return;
        }
      };

      // ---- Cleanup (same logic as before, no changes) ----
      match crate::db::last_action::LastAction::should_skip(
        &db,
        "cleanup_executions",
        CLEANUP_INTERVAL_SECS,
      )
      .await
      {
        Ok(false) => {
          if let Err(e) = cleanup_old_executions().await {
            tracing::warn!("Execution cleanup failed: {}", e);
          } else {
            if let Err(e) =
              crate::db::last_action::LastAction::record(&db, "cleanup_executions").await
            {
              tracing::warn!("Failed to record cleanup timestamp: {}", e);
            }
            if let Err(e) = crate::db::last_action::LastAction::cleanup_old(&db).await {
              tracing::warn!("Failed to purge old last_action records: {}", e);
            }
          }
        }
        Ok(true) => {}
        Err(e) => {
          tracing::warn!("Failed to check last_action for cleanup: {}", e);
        }
      }

      // ---- Query unsynced executions (same as before) ----
      let unsynced: Vec<crate::db::remex::ExecutionCache> = match db
        .query("USE NS remex DB remex; SELECT * FROM execution WHERE synced = false;")
        .await
      {
        Ok(res) => match res.check() {
          Ok(mut r) => match r.take(1) {
            Ok(v) => v,
            Err(e) => {
              tracing::warn!("Failed to take unsynced executions: {}", e);
              return;
            }
          },
          Err(e) => {
            tracing::warn!("Failed to check unsynced executions: {}", e);
            return;
          }
        },
        Err(e) => {
          tracing::warn!("Failed to query unsynced executions: {}", e);
          return;
        }
      };

      if unsynced.is_empty() {
        return;
      }

      let repo = SurrealExecutionCacheRepo { db: db.clone() };
      let count = push_unsynced_executions(unsynced, &repo, &remote_db).await;
      tracing::info!("Pushed {count} unsynced executions to remote");
    });

    ctx.notify_later(SyncTick, Duration::from_secs(30));
  }
}
