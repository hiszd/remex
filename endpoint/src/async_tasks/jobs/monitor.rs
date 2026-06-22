use std::time::Duration;

use remex_core::db::{
  model::jobs::{Enabled, Job},
  DbOperator,
};

use crate::db::remex::{JobCacheData, SurrealJobCacheRepo};
use surrealdb::{engine::remote::ws::Client, types::Action};
use surrealdb::types::{RecordId, ToSql};
use surrealdb::Surreal;
use tokio::sync::{mpsc, watch};
use tokio_stream::StreamExt;

#[derive(Debug, Clone)]
pub enum MonitorCommand {
  SetClientId(String),
}

async fn mark_job_incomplete(job_id: &RecordId) -> Result<(), crate::Error> {
  let db = crate::db::get_local_remex().await?;
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

async fn load_jobs_from_local_db(
  job_injection_tx: &mpsc::Sender<super::JobQueueMessage>,
  client_id: &str,
) -> Result<(), crate::Error> {
  println!("Loading cached jobs from local database...");
  tracing::info!("Loading jobs from local database cache");

  let cached_jobs: Vec<crate::db::remex::JobCache> = match crate::db::get_local_remex()
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
  for cached in cached_jobs {
    let job = cached.job_info;
    tracing::debug!("Loading job from cache: {}", job.job_name);
    if let Some(exec_time) = super::calculate_execution_time(&job.job_type) {
      let _ = job_injection_tx
        .send(super::JobQueueMessage::Scheduled {
          job,
          execution_time: exec_time,
          client_id: client_id.to_string(),
        })
        .await;
    } else {
      let _ = job_injection_tx
        .send(super::JobQueueMessage::Immediate {
          job,
          client_id: client_id.to_string(),
        })
        .await;
    }
  }
  Ok(())
}

pub async fn run(
  mut cmd_rx: mpsc::Receiver<MonitorCommand>,
  job_injection_tx: mpsc::Sender<super::JobQueueMessage>,
  mut db_handle_rx: watch::Receiver<Option<Surreal<Client>>>,
) -> Result<(), crate::Error> {
  let mut client_id: Option<String> = None;
  let mut groups: Vec<RecordId> = Vec::new();
  let mut initial_sync_done = false;

  loop {
    let remote_db: Option<Surreal<Client>> = db_handle_rx.borrow_and_update().clone();

    if remote_db.is_none() && client_id.is_none() {
      tokio::time::sleep(Duration::from_secs(1)).await;
      if let Some(cmd) = cmd_rx.try_recv().ok() {
        match cmd {
          MonitorCommand::SetClientId(id) => client_id = Some(id),
        }
      }
      continue;
    }

    if remote_db.is_none() {
      if let Some(ref cid) = client_id {
        tracing::debug!("Remote DB not connected, loading jobs from local cache");
        if let Err(e) = load_jobs_from_local_db(&job_injection_tx, cid).await {
          tracing::warn!("Failed to load from local cache: {}", e);
        }
      }
      tokio::time::sleep(Duration::from_secs(5)).await;
      continue;
    }

    let remote_db = remote_db.unwrap();
    let cid = match client_id.clone() {
      Some(id) => id,
      None => {
        tokio::time::sleep(Duration::from_secs(1)).await;
        continue;
      }
    };

    if !initial_sync_done {
      tracing::info!("First connection to remote, syncing jobs from remote");
      if let Err(e) = super::sync::full_sync(&cid, &job_injection_tx, &remote_db).await {
        tracing::warn!("Failed to sync from remote: {}", e);
      } else {
        initial_sync_done = true;
      }
    }

    let mut stream = match remote_db.select::<Vec<Job>>("job").live().await {
      Ok(s) => s,
      Err(e) => {
        tracing::warn!("Failed to create job live query: {}", e);
        tokio::time::sleep(Duration::from_secs(2)).await;
        continue;
      }
    };

    let mut groupstream = match remote_db.select::<Vec<remex_core::db::model::groups::Group>>("group").live().await {
      Ok(s) => s,
      Err(e) => {
        tracing::warn!("Failed to create group live query: {}", e);
        tokio::time::sleep(Duration::from_secs(2)).await;
        continue;
      }
    };

    tracing::info!("Monitoring jobs loop starting");
    loop {
      let id = match RecordId::parse_simple(&cid) {
        Ok(id) => id,
        Err(_) => break,
      };

      tokio::select! {
        notification = stream.next() => {
          tracing::debug!("Job notification received");
          match notification {
            Some(Ok(notification)) => {
              if !notification.data.assignments.contains(&id)
                && !notification.data.assignments.iter().any(|g| groups.contains(g))
              {
                tracing::debug!("Job {} not assigned to this client, skipping", notification.data.job_name);
                continue;
              }
              match notification.action {
                Action::Create => {
                  tracing::debug!("Job created: {:#?}", notification.data.job_name);
                  let job = notification.data.clone();
                  let job_id = job.id.clone();

                  let local_db = match crate::db::get_local_remex().await {
                    Ok(d) => d,
                    Err(e) => {
                      tracing::warn!("Failed to get local DB for job cache: {}", e);
                      return Ok(());
                    }
                  };
                  let existing: Vec<crate::db::remex::JobCache> = match local_db
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
                    let cache_entry = JobCacheData {
                      job_id: job_id.to_sql(),
                      job_info: job.clone(),
                      completed: false,
                    };
                    let repo = SurrealJobCacheRepo { db: local_db.clone() };
                    let _ = repo.create(cache_entry).await;
                  }

                  let _ = mark_job_incomplete(&job_id).await;

                  if let Some(exec_time) = super::calculate_execution_time(&job.job_type) {
                    let _ = job_injection_tx.send(super::JobQueueMessage::Scheduled {
                      job,
                      execution_time: exec_time,
                      client_id: cid.clone(),
                    }).await;
                  } else {
                    let _ = job_injection_tx.send(super::JobQueueMessage::Immediate {
                      job,
                      client_id: cid.clone(),
                    }).await;
                  }
                }
                Action::Update => {
                  println!("Job updated in remote: {}", notification.data.job_name);
                  tracing::debug!("Job updated: {}", notification.data.job_name);

                  let job_id = notification.data.id.clone();
                  let updated_job = notification.data.clone();

                  let local_db = match crate::db::get_local_remex().await {
                    Ok(d) => d,
                    Err(e) => {
                      tracing::warn!("Failed to get local DB for job cache: {}", e);
                      return Ok(());
                    }
                  };
                  let repo = SurrealJobCacheRepo { db: local_db.clone() };
                  let existing: Vec<crate::db::remex::JobCache> = match local_db
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
                    let data = JobCacheData {
                      job_id: cached.job_id.clone(),
                      job_info: updated_job.clone(),
                      completed: false,
                    };
                    let _ = repo.update(&cached.cache_id(), data).await;
                  } else {
                    let cache_entry = JobCacheData {
                      job_id: job_id.to_sql(),
                      job_info: updated_job.clone(),
                      completed: false,
                    };
                    let _ = repo.create(cache_entry).await;
                  }

                  if notification.data.enabled == Enabled::Enabled {
                    let _ = job_injection_tx
                      .send(super::JobQueueMessage::Remove {
                        id: notification.data.id.clone(),
                      })
                      .await;

                    let job = notification.data.clone();
                    if let Some(exec_time) = super::calculate_execution_time(&job.job_type) {
                      let _ = job_injection_tx.send(super::JobQueueMessage::Scheduled {
                        job,
                        execution_time: exec_time,
                        client_id: cid.clone(),
                      }).await;
                    } else {
                      let _ = job_injection_tx.send(super::JobQueueMessage::Immediate {
                        job,
                        client_id: cid.clone(),
                      }).await;
                    }
                  }
                }
                Action::Delete | Action::Killed => {
                  println!("Job removed from remote: {}", notification.data.job_name);
                  tracing::debug!("Job removed from remote: {}", notification.data.job_name);
                  let _ = job_injection_tx
                    .send(super::JobQueueMessage::Remove {
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
                  if !notification.data.members.contains(&id) {
                    tracing::debug!("Group {} not assigned to this client, skipping", notification.data.group_name);
                    continue;
                  }
                  groups.push(notification.data.id.clone());
                }
                Action::Update => {
                  println!("Group updated in remote: {}", notification.data.group_name);
                  tracing::debug!("Group updated: {}", notification.data.group_name);
                  if !notification.data.members.contains(&id) {
                    tracing::debug!("Group {} not assigned to this client, skipping", notification.data.group_name);
                    groups.retain(|g| g != &notification.data.id);
                    continue;
                  }
                  groups.retain(|g| g != &notification.data.id);
                  groups.push(notification.data.id.clone());
                }
                Action::Delete | Action::Killed => {
                  println!("Group removed from remote: {}", notification.data.group_name);
                  tracing::debug!("Group removed from remote: {}", notification.data.group_name);
                  groups.retain(|g| g != &notification.data.id);
                }
              }

              if let Err(e) = super::sync::sync_and_refill_queue(&job_injection_tx, &cid, &groups, &remote_db).await {
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
