use std::time::Duration;

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
use tokio::sync::watch;

use super::JobQueueMessage;
use crate::db::remex::{
  ExecutionCacheData,
  SurrealExecutionCacheRepo,
};

pub async fn sync_groups(
  client_id: &str,
  remote_db: &Surreal<Any>,
) -> Result<Vec<Group>, crate::Error> {
  println!("Syncing groups from remote...");
  tracing::info!("Syncing groups from remote database");

  let groups: Vec<Group> = remote_db
    .query(format!("SELECT * FROM group WHERE members CONTAINS {client_id};"))
    .await?
    .check()?
    .take(0)?;

  tracing::debug!("Fetched {} groups from remote", groups.len());

  let id = RecordId::parse_simple(client_id).unwrap();
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
  groups: &[RecordId],
  remote_db: &Surreal<Any>,
) -> Result<(), crate::Error> {
  use crate::db::remex::{
    JobCache,
    SurrealJobCacheRepo,
  };

  println!("Syncing jobs from remote...");
  tracing::info!("Syncing jobs from remote database");

  let jobs: Vec<Job> = remote_db
    .query("SELECT * FROM job;")
    .await?
    .check()?
    .take(0)?;

  tracing::debug!("Fetched {} jobs from remote", jobs.len());

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
    let _ = sync_job_to_cache(&job, existing.first(), &repo).await;

    if let Some(exec_time) = super::calculate_execution_time(&job.job_type) {
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
      model::jobs::{
        Enabled,
        ExecutionStatus,
        Job,
        JobType,
      },
      DbOperator,
    },
    impl_in_memory_db_operator,
  };
  use surrealdb::types::ToSql;

  use crate::db::remex::{
    JobCache,
    JobCacheData,
  };

  impl_in_memory_db_operator!(InMemoryJobCacheRepo, JobCache, JobCacheData, "job");

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
}

pub async fn full_sync(
  client_id: &str,
  job_injection_tx: &tokio::sync::mpsc::Sender<JobQueueMessage>,
  remote_db: &Surreal<Any>,
) -> Result<(), crate::Error> {
  let groups = sync_groups(client_id, remote_db).await?;
  let group_ids: Vec<RecordId> = groups.iter().map(|g| g.id.clone()).collect();
  sync_and_refill_queue(job_injection_tx, client_id, &group_ids, remote_db).await
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

pub async fn execution_sync_loop(
  mut db_handle_rx: watch::Receiver<Option<Surreal<Any>>>,
) -> Result<(), crate::Error> {
  use crate::db::remex::ExecutionCache;

  const CLEANUP_INTERVAL_SECS: u64 = 6 * 3600;

  loop {
    tokio::time::sleep(Duration::from_secs(30)).await;

    let remote_db = match db_handle_rx.borrow_and_update().clone() {
      Some(db) => db,
      None => {
        tracing::debug!("Remote DB not connected, skipping execution sync");
        continue;
      }
    };

    let db = match crate::db::get_local_remex().await {
      Ok(d) => d,
      Err(e) => {
        tracing::warn!("Failed to get local DB for execution sync: {}", e);
        continue;
      }
    };

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

    let unsynced: Vec<ExecutionCache> = match db
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

    let repo = SurrealExecutionCacheRepo { db: db.clone() };

    for entry in unsynced {
      let exec = entry.execution_info.clone();
      let exec_id = entry.execution_id.clone();
      let cache_id = entry.cache_id();

      let data = ExecutionCacheData {
        execution_id: exec_id.clone(),
        execution_info: exec.clone(),
        synced: true,
      };
      if let Err(e) = repo.update(&cache_id, data).await {
        tracing::warn!("Failed to mark execution as synced before push: {}", e);
        continue;
      }

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
            let revert_data = ExecutionCacheData {
              execution_id: exec_id,
              execution_info: entry.execution_info.clone(),
              synced: false,
            };
            let _ = repo.update(&cache_id, revert_data).await;
          }
        },
        Err(e) => {
          tracing::warn!("Failed to push execution to remote, reverting synced flag: {}", e);
          let revert_data = ExecutionCacheData {
            execution_id: exec_id,
            execution_info: entry.execution_info.clone(),
            synced: false,
          };
          let _ = repo.update(&cache_id, revert_data).await;
        }
      }
    }
  }
}
