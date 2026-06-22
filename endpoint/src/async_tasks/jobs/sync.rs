use std::time::Duration;

use remex_core::db::{
  model::groups::Group,
  model::jobs::Job,
  LegacyDbOperator,
};
use surrealdb::{engine::remote::ws::Client, types::RecordId, Surreal};
use surrealdb::types::ToSql;
use tokio::sync::watch;

use super::JobQueueMessage;

pub async fn sync_groups(
  client_id: &str,
  remote_db: &Surreal<Client>,
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
  remote_db: &Surreal<Client>,
) -> Result<(), crate::Error> {
  use crate::db::remex::JobCache;

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
    }

    let job_id_str = job.id.to_sql();
    let local_db = crate::db::get_local_remex().await?;

    let existing: Vec<JobCache> = match local_db
      .query(
        "USE NS remex DB remex; SELECT * FROM job WHERE job_id = $job_id LIMIT 1;",
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
      let local_updated = cached.job_info.updated_at;
      let remote_updated = job.updated_at;
      if local_updated != remote_updated {
        tracing::debug!(
          "Job {} was modified while offline, resetting completion status",
          job.job_name
        );
        false
      } else {
        cached.completed
      }
    } else {
      false
    };

    let cache_entry = crate::db::remex::JobCacheData {
      job_id: job_id_str.clone(),
      job_info: job.clone(),
      completed,
    };
    let _ = JobCache::create(cache_entry, &local_db).await;

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

pub async fn full_sync(
  client_id: &str,
  job_injection_tx: &tokio::sync::mpsc::Sender<JobQueueMessage>,
  remote_db: &Surreal<Client>,
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
  mut db_handle_rx: watch::Receiver<Option<Surreal<Client>>>,
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

    match crate::db::last_action::LastAction::should_skip(&db, "cleanup_executions", CLEANUP_INTERVAL_SECS).await {
      Ok(false) => {
        if let Err(e) = cleanup_old_executions().await {
          tracing::warn!("Execution cleanup failed: {}", e);
        } else {
          if let Err(e) = crate::db::last_action::LastAction::record(&db, "cleanup_executions").await {
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

    for entry in unsynced {
      let exec = entry.execution_info.clone();
      let exec_id = entry.execution_id.clone();

      let mut synced_entry = entry;
      synced_entry.synced = true;
      if let Err(e) = synced_entry.push(&db).await {
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
            let mut reverted = synced_entry;
            reverted.synced = false;
            let _ = reverted.push(&db).await;
          }
        },
        Err(e) => {
          tracing::warn!("Failed to push execution to remote, reverting synced flag: {}", e);
          let mut reverted = synced_entry;
          reverted.synced = false;
          let _ = reverted.push(&db).await;
        }
      }
    }
  }
}
