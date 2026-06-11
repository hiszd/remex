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
use surrealdb::types::{
  Action,
  RecordId,
  ToSql,
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
  Immediate { job: Job },
  Scheduled { job: Job, execution_time: Instant },
  Remove { id: RecordId },
  SyncFromRemote,
}

#[derive(Debug, Clone)]
struct ScheduledJob {
  execution_time: Instant,
  job: Job,
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
        JobQueueMessage::Immediate { job } => {
          tracing::debug!("Immediate job received: {}", job.job_name);
          let job_clone = job.clone();
          tokio::spawn(async move {
            if let Err(e) = execute_job(job_clone).await {
              tracing::error!("Job execution failed Immediate: {}", e);
            }
          });
        }
        JobQueueMessage::Scheduled { job, execution_time } => {
          tracing::debug!("Scheduled job received: {} at {:?}", job.job_name, execution_time);
          if execution_time > now {
            tracing::debug!("Job queued: {} at {:?}", job.job_name, execution_time);
            heap.push(ScheduledJob {
              execution_time,
              job,
            });
          } else {
            let job_clone = job.clone();
            tokio::spawn(async move {
              if let Err(e) = execute_job(job_clone).await {
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
          let job_clone = scheduled.job.clone();
          tokio::spawn(async move {
            if let Err(e) = execute_job(job_clone).await {
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
) -> Result<(), crate::Error> {
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
    if !job.assignments.contains(&id) {
      tracing::debug!("Skipping job (not assigned): {}", job.job_name);
      continue;
    }

    if let Some(exec_time) = calculate_execution_time(&job.job_type) {
      tracing::debug!("Injecting scheduled job: {} at {:?}", job.job_name, exec_time);
      let _ = job_injection_tx
        .send(JobQueueMessage::Scheduled {
          job,
          execution_time: exec_time,
        })
        .await;
      queued_count += 1;
    } else {
      tracing::debug!("Injecting immediate job: {}", job.job_name);
      let _ = job_injection_tx
        .send(JobQueueMessage::Immediate { job })
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
        })
        .await;
    } else {
      let _ = job_injection_tx
        .send(JobQueueMessage::Immediate { job })
        .await;
    }
  }

  println!("Loaded {} jobs from local cache", count);
  tracing::info!("Loaded {} jobs from local cache", count);
  Ok(())
}

pub async fn refill_queue_from_remote(
  job_injection_tx: &tokio::sync::mpsc::Sender<JobQueueMessage>,
  client_id: &str,
) -> Result<(), crate::Error> {
  println!("Refilling job queue from remote...");
  tracing::info!("Refilling queue from remote database");

  let _ = job_injection_tx.send(JobQueueMessage::SyncFromRemote).await;

  tokio::time::sleep(Duration::from_millis(50)).await;

  sync_and_refill_queue(job_injection_tx, client_id).await
}

/// Runs a command and returns the output as (output, command, exit_code)
async fn run_command(cmd: &str) -> Result<(String, &str, std::process::ExitStatus), crate::Error> {
  println!("Running command: {}", cmd);
  // tracing::debug!("Running command: {} -c {}", job.job_shell, job.job_command);
  tracing::debug!("Running command: {}", cmd);
  let output = tokio::process::Command::new("echo")
    .arg(&cmd)
    .output()
    .await;

  let out = output?;
  let stdout = String::from_utf8_lossy(out.stdout.as_slice()).to_string();
  let stderr = String::from_utf8_lossy(out.stderr.as_slice()).to_string();
  tracing::debug!(
    "Command exit code: {:?}, stdout: {} , stderr: {} ",
    out.status.code(),
    String::from_utf8_lossy(out.stdout.as_slice()),
    String::from_utf8_lossy(out.stderr.as_slice())
  );
  Ok((format!("out: {}\nerr: {}", stdout, stderr), cmd, out.status))
}

async fn execute_job(job: Job) -> Result<(), crate::Error> {
  use remex_core::db::{
    model::executions::{
      Execution,
      ExecutionData,
      ExecutionStatus,
    },
    DbOperator,
  };

  println!("Executing job: {}", job.job_name);

  let (output_str, command_str, exit_status) = run_command(&job.job_command).await.unwrap();

  let execution_status = if exit_status.success() {
    ExecutionStatus::Completed
  } else {
    ExecutionStatus::Failed
  };

  let time_start = surrealdb::types::Datetime::now();
  let client_id = surrealdb::types::RecordId::parse_simple("client:self").unwrap();
  let execution = ExecutionData {
    job_id: Some(job.id.clone()),
    client_id,
    status: execution_status,
    output: output_str,
    command: command_str.to_string(),
    exit_code: exit_status.code().unwrap_or(0).to_string(),
    execution_start: Some(time_start.clone()),
    execution_end: Some(surrealdb::types::Datetime::now()),
    created_at: Some(surrealdb::types::Datetime::now()),
    updated_at: Some(surrealdb::types::Datetime::now()),
  };

  let db = crate::db::get_local_remex().await?;
  Execution::create(execution, &db).await?;

  Ok(())
}

async fn execute_job_old(job: Job) -> Result<(), crate::Error> {
  println!("Executing job: {}", job.job_name);

  let remote_db = get_remote_remex().await?;
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
      if let Err(e) = load_jobs_from_local_db(&job_injection_tx).await {
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
          if let Err(e) = sync_and_refill_queue(&job_injection_tx, &client_id).await {
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
                  tracing::debug!("Job created: {:#?}", notification.data.job_name);
                  let job = notification.data.clone();

                  if let Some(exec_time) = calculate_execution_time(&job.job_type) {
                    let _ = job_injection_tx.send(JobQueueMessage::Scheduled {
                      job,
                      execution_time: exec_time,
                    }).await;
                  } else {
                    let _ = job_injection_tx.send(JobQueueMessage::Immediate {
                      job,
                    }).await;
                  }
                }
                Action::Update => {
                  println!("Job updated in remote: {}", notification.data.job_name);
                  tracing::debug!("Job updated: {}", notification.data.job_name);

                  if notification.data.enabled == Enabled::Enabled {
                    let _ = job_injection_tx
                      .send(JobQueueMessage::Remove {
                        id: notification.data.id.clone(),
                      })
                      .await;

                    let job = notification.data.clone();
                    if let Some(exec_time) = calculate_execution_time(&job.job_type) {
                      let _ = job_injection_tx.send(JobQueueMessage::Scheduled {
                        job,
                        execution_time: exec_time,
                      }).await;
                    } else {
                      let _ = job_injection_tx.send(JobQueueMessage::Immediate {
                        job,
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
