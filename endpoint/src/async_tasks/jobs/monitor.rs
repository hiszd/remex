use actix::prelude::*;
use remex_core::db::{
  model::jobs::{Enabled, Job},
  DbOperator,
};
use surrealdb::{engine::any::Any, types::Action};
use surrealdb::types::{RecordId, ToSql};
use surrealdb::Surreal;
use tokio_stream::StreamExt;

use crate::async_tasks::ConnectionReady;
use crate::db::remex::{JobCacheData, SurrealJobCacheRepo};

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

pub struct MonitorActor {
  remote_db: Option<Surreal<Any>>,
  client_id: Option<String>,
  groups: Vec<RecordId>,
  scheduler_addr: actix::Addr<super::scheduler::SchedulerActor>,
}

#[derive(Message)]
#[rtype(result = "()")]
struct GroupUpdate(Vec<RecordId>);

impl MonitorActor {
  pub fn new(scheduler_addr: actix::Addr<super::scheduler::SchedulerActor>) -> Self {
    MonitorActor {
      remote_db: None,
      client_id: None,
      groups: Vec::new(),
      scheduler_addr,
    }
  }
}

impl Actor for MonitorActor {
  type Context = Context<Self>;
}

impl actix::Supervised for MonitorActor {
    fn restarting(&mut self, _ctx: &mut Context<Self>) {
        tracing::info!("MonitorActor: restarting");
        self.remote_db = None;
        self.client_id = None;
        self.groups.clear();
    }
}

impl Handler<ConnectionReady> for MonitorActor {
  type Result = ();

  fn handle(&mut self, msg: ConnectionReady, ctx: &mut Self::Context) {
    self.remote_db = msg.db.clone();
    if let Some(ref cid) = msg.client_id {
      self.client_id = Some(cid.clone());
    }

    let remote_db = match self.remote_db.clone() {
      Some(db) => db,
      None => return,
    };
    let client_id = match self.client_id.clone() {
      Some(id) => id,
      None => return,
    };
    let groups = self.groups.clone();
    let scheduler_addr = self.scheduler_addr.clone();
    let addr = ctx.address();

    tokio::spawn(async move {
      monitor_task(remote_db, client_id, groups, scheduler_addr, addr).await;
    });
  }
}

impl Handler<GroupUpdate> for MonitorActor {
  type Result = ();

  fn handle(&mut self, msg: GroupUpdate, _ctx: &mut Self::Context) {
    self.groups = msg.0;
    tracing::debug!("MonitorActor: groups updated (total {})", self.groups.len());
  }
}

/// Spawned monitoring task: does full_sync, sets up LIVE SELECT streams,
/// processes job/group notifications, sends InjectJob to SchedulerActor.
async fn monitor_task(
  remote_db: Surreal<Any>,
  client_id: String,
  mut groups: Vec<RecordId>,
  scheduler_addr: actix::Addr<super::scheduler::SchedulerActor>,
  actor_addr: actix::Addr<MonitorActor>,
) {
  // ---- Load cached jobs from local DB ----
  tracing::info!("Monitor: loading cached jobs from local database");
  let cached_jobs: Vec<crate::db::remex::JobCache> = match crate::db::get_local_remex()
    .await
  {
    Ok(db) => match db
      .query("USE NS remex DB remex; SELECT * FROM job;")
      .await
    {
      Ok(res) => match res.check() {
        Ok(mut r) => r.take(1).unwrap_or_default(),
        Err(e) => {
          tracing::warn!("Monitor: failed to check local jobs: {e}");
          vec![]
        }
      },
      Err(e) => {
        tracing::warn!("Monitor: failed to query local jobs: {e}");
        vec![]
      }
    },
    Err(e) => {
      tracing::warn!("Monitor: failed to get local DB for cached jobs: {e}");
      vec![]
    }
  };

  // Forward cached jobs to scheduler
  for cached in cached_jobs {
    let job = cached.job_info;
    if let Some(exec_time) = super::calculate_execution_time(&job.job_type) {
      if let Err(e) = scheduler_addr
        .send(super::scheduler::InjectJob(super::JobQueueMessage::Scheduled {
          job,
          execution_time: exec_time,
          client_id: client_id.clone(),
        }))
        .await
      {
        tracing::warn!("Monitor: failed to send cached scheduled job to scheduler: {e}");
      }
    } else {
      if let Err(e) = scheduler_addr
        .send(super::scheduler::InjectJob(super::JobQueueMessage::Immediate {
          job,
          client_id: client_id.clone(),
        }))
        .await
      {
        tracing::warn!("Monitor: failed to send cached immediate job to scheduler: {e}");
      }
    }
  }

  // ---- Full sync from remote ----
  tracing::info!("Monitor: syncing jobs from remote");
  if let Err(e) = super::sync::full_sync(&client_id, &scheduler_addr, &remote_db).await {
    tracing::warn!("Monitor: full_sync failed: {e}");
  }

  // ---- Set up LIVE SELECT streams ----
  let id = match RecordId::parse_simple(&client_id) {
    Ok(id) => id,
    Err(_) => {
      tracing::error!("Monitor: invalid client_id format: {client_id}");
      return;
    }
  };

  let mut stream = match remote_db.select::<Vec<Job>>("job").live().await {
    Ok(s) => s,
    Err(e) => {
      tracing::warn!("Monitor: failed to create job live query: {e}");
      return;
    }
  };

  let mut groupstream = match remote_db
    .select::<Vec<remex_core::db::model::groups::Group>>("group")
    .live()
    .await
  {
    Ok(s) => s,
    Err(e) => {
      tracing::warn!("Monitor: failed to create group live query: {e}");
      return;
    }
  };

  tracing::info!("Monitor: monitoring jobs loop starting");

  loop {
    tokio::select! {
      notification = stream.next() => {
        match notification {
          Some(Ok(notification)) => {
            // Check assignment
            if !notification.data.assignments.contains(&id)
              && !notification.data.assignments.iter().any(|g| groups.contains(g))
            {
              tracing::debug!("Monitor: job {} not assigned, skipping", notification.data.job_name);
              continue;
            }

            match notification.action {
              Action::Create => {
                tracing::debug!("Monitor: job created: {}", notification.data.job_name);
                let job = notification.data.clone();
                let job_id = job.id.clone();

                // Cache the new job locally
                let local_db = match crate::db::get_local_remex().await {
                  Ok(d) => d,
                  Err(e) => {
                    tracing::warn!("Monitor: failed to get local DB for job cache: {e}");
                    return;
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
                    Ok(mut r) => r.take(1).unwrap_or_default(),
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
                  if let Err(e) = repo.create(cache_entry).await {
                    tracing::error!("Monitor: failed to create cache entry for new job {}: {e}", job.job_name);
                  }
                }

                if let Err(e) = mark_job_incomplete(&job_id).await {
                  tracing::error!("Monitor: failed to mark new job {} as incomplete: {e}", job.job_name);
                }

                // Inject into scheduler
                if let Some(exec_time) = super::calculate_execution_time(&job.job_type) {
                  if let Err(e) = scheduler_addr
                    .send(super::scheduler::InjectJob(super::JobQueueMessage::Scheduled {
                      job,
                      execution_time: exec_time,
                      client_id: client_id.clone(),
                    }))
                    .await
                  {
                    tracing::warn!("Monitor: failed to inject new scheduled job to scheduler: {e}");
                  }
                } else {
                  if let Err(e) = scheduler_addr
                    .send(super::scheduler::InjectJob(super::JobQueueMessage::Immediate {
                      job,
                      client_id: client_id.clone(),
                    }))
                    .await
                  {
                    tracing::warn!("Monitor: failed to inject new immediate job to scheduler: {e}");
                  }
                }
              }
              Action::Update => {
                tracing::debug!("Monitor: job updated: {}", notification.data.job_name);
                let job_id = notification.data.id.clone();
                let updated_job = notification.data.clone();

                // Update local cache
                let local_db = match crate::db::get_local_remex().await {
                  Ok(d) => d,
                  Err(e) => {
                    tracing::warn!("Monitor: failed to get local DB for job cache: {e}");
                    return;
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
                    Ok(mut r) => r.take(1).unwrap_or_default(),
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
                  if let Err(e) = repo.update(&cached.cache_id(), data).await {
                    tracing::error!("Monitor: failed to update cache for job {job_id:?}: {e}");
                  }
                } else {
                  let cache_entry = JobCacheData {
                    job_id: job_id.to_sql(),
                    job_info: updated_job.clone(),
                    completed: false,
                  };
                  if let Err(e) = repo.create(cache_entry).await {
                    tracing::error!("Monitor: failed to create cache entry for updated job {job_id:?}: {e}");
                  }
                }

                if notification.data.enabled == Enabled::Enabled {
                  // Remove old job from scheduler queue
                  if let Err(e) = scheduler_addr
                    .send(super::scheduler::InjectJob(super::JobQueueMessage::Remove {
                      id: notification.data.id.clone(),
                    }))
                    .await
                  {
                    tracing::warn!("Monitor: failed to remove updated job from scheduler: {e}");
                  }

                  // Re-inject updated job
                  let job = notification.data.clone();
                  if let Some(exec_time) = super::calculate_execution_time(&job.job_type) {
                    if let Err(e) = scheduler_addr
                      .send(super::scheduler::InjectJob(super::JobQueueMessage::Scheduled {
                        job,
                        execution_time: exec_time,
                        client_id: client_id.clone(),
                      }))
                      .await
                    {
                      tracing::warn!("Monitor: failed to inject updated scheduled job to scheduler: {e}");
                    }
                  } else {
                    if let Err(e) = scheduler_addr
                      .send(super::scheduler::InjectJob(super::JobQueueMessage::Immediate {
                        job,
                        client_id: client_id.clone(),
                      }))
                      .await
                    {
                      tracing::warn!("Monitor: failed to inject updated immediate job to scheduler: {e}");
                    }
                  }
                }
              }
              Action::Delete | Action::Killed => {
                tracing::debug!("Monitor: job removed from remote: {}", notification.data.job_name);
                if let Err(e) = scheduler_addr
                  .send(super::scheduler::InjectJob(super::JobQueueMessage::Remove {
                    id: notification.data.id.clone(),
                  }))
                  .await
                {
                  tracing::warn!("Monitor: failed to remove deleted job from scheduler: {e}");
                }
              }
            }
          }
          Some(Err(err)) => {
            tracing::error!("Monitor: job stream error: {:#?}", err);
          }
          None => {
            tracing::warn!("Monitor: job notification stream ended");
            break;
          }
        }
      }
      group_notification = groupstream.next() => {
        match group_notification {
          Some(Ok(notification)) => {
            match notification.action {
              Action::Create => {
                tracing::debug!("Monitor: group created: {}", notification.data.group_name);
                if !notification.data.members.contains(&id) {
                  tracing::debug!("Monitor: group {} not assigned, skipping", notification.data.group_name);
                  continue;
                }
                groups.push(notification.data.id.clone());
                if let Err(e) = actor_addr.send(GroupUpdate(groups.clone())).await {
                  tracing::warn!("Monitor: failed to send GroupUpdate after create: {e}");
                }
              }
              Action::Update => {
                tracing::debug!("Monitor: group updated: {}", notification.data.group_name);
                if !notification.data.members.contains(&id) {
                  tracing::debug!("Monitor: group {} not assigned, removing from groups", notification.data.group_name);
                  groups.retain(|g| g != &notification.data.id);
                  if let Err(e) = actor_addr.send(GroupUpdate(groups.clone())).await {
                    tracing::warn!("Monitor: failed to send GroupUpdate after remove: {e}");
                  }
                  continue;
                }
                groups.retain(|g| g != &notification.data.id);
                groups.push(notification.data.id.clone());
                if let Err(e) = actor_addr.send(GroupUpdate(groups.clone())).await {
                  tracing::warn!("Monitor: failed to send GroupUpdate after update: {e}");
                }
              }
              Action::Delete | Action::Killed => {
                tracing::debug!("Monitor: group removed from remote: {}", notification.data.group_name);
                groups.retain(|g| g != &notification.data.id);
                if let Err(e) = actor_addr.send(GroupUpdate(groups.clone())).await {
                  tracing::warn!("Monitor: failed to send GroupUpdate after delete: {e}");
                }
              }
            }

            // Re-sync jobs from remote after group change
            if let Err(e) = super::sync::sync_and_refill_queue(&scheduler_addr, &client_id, &groups, &remote_db).await {
              tracing::warn!("Monitor: sync_and_refill_queue failed after group change: {e}");
            }
          }
          Some(Err(err)) => {
            tracing::error!("Monitor: group stream error: {:#?}", err);
          }
          None => {
            tracing::warn!("Monitor: group notification stream ended");
            break;
          }
        }
      }
    }
  }

  tracing::info!("Monitor: monitoring task ended");
}
