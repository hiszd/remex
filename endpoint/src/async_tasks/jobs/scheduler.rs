use std::{
  cmp::Ordering,
  collections::BinaryHeap,
  sync::Arc,
  time::Duration,
};

use actix::prelude::*;
use remex_core::db::model::jobs::Job;
use surrealdb::types::ToSql;
use tokio::time::Instant;

use super::{
  JobExecutor,
  JobQueueMessage,
};

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
  fn eq(&self, other: &Self) -> bool { self.job.id == other.job.id }
}

impl Eq for ScheduledJob {
}

pub struct SchedulerActor {
  heap: BinaryHeap<ScheduledJob>,
  executor: Arc<dyn JobExecutor>,
  execution_recorder: actix::Recipient<crate::async_tasks::RecordExecution>,
}

impl SchedulerActor {
  pub fn new(
    executor: Arc<dyn JobExecutor>,
    execution_recorder: actix::Recipient<crate::async_tasks::RecordExecution>,
  ) -> Self {
    SchedulerActor {
      heap: BinaryHeap::new(),
      executor,
      execution_recorder,
    }
  }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct InjectJob(pub JobQueueMessage);

#[derive(Message)]
#[rtype(result = "()")]
pub(crate) struct SchedulerWakeUp;

impl Actor for SchedulerActor {
  type Context = Context<Self>;
}

impl actix::Supervised for SchedulerActor {
  fn restarting(&mut self, _ctx: &mut Context<Self>) {
    tracing::info!("SchedulerActor: restarting, clearing job queue");
    self.heap.clear();
  }
}

impl Handler<InjectJob> for SchedulerActor {
  type Result = ();

  fn handle(&mut self, msg: InjectJob, ctx: &mut Self::Context) {
    let now = Instant::now();

    match msg.0 {
      JobQueueMessage::Immediate { job, client_id } => {
        tracing::debug!("Immediate job received: {}", job.job_name);
        let executor = Arc::clone(&self.executor);
        let execution_recorder = self.execution_recorder.clone();
        tokio::spawn(async move {
          match executor.execute(job, &client_id).await {
            Ok(Some(result)) => {
              execution_recorder.do_send(crate::async_tasks::RecordExecution { result });
            }
            Ok(None) => {
              tracing::debug!("Job skipped (already completed)");
            }
            Err(e) => {
              tracing::error!("Job execution failed Immediate: {}", e);
            }
          }
        });
      }
      JobQueueMessage::Scheduled {
        job,
        execution_time,
        client_id,
      } => {
        tracing::debug!("Scheduled job received: {} at {:?}", job.job_name, execution_time);
        if execution_time > now {
          tracing::debug!("Job queued: {} at {:?}", job.job_name, execution_time);
          self.heap.push(ScheduledJob {
            execution_time,
            job,
            client_id,
          });
          self.schedule_next(ctx);
        } else {
          let executor = Arc::clone(&self.executor);
          let execution_recorder = self.execution_recorder.clone();
          tokio::spawn(async move {
            match executor.execute(job, &client_id).await {
              Ok(Some(result)) => {
                execution_recorder.do_send(crate::async_tasks::RecordExecution { result });
              }
              Ok(None) => {
                tracing::debug!("Job skipped (already completed)");
              }
              Err(e) => {
                tracing::error!("Job execution failed (scheduled past): {}", e);
              }
            }
          });
        }
      }
      JobQueueMessage::Remove { id } => {
        tracing::info!("Removing job from queue: {}", id.to_sql());
        self.heap.retain(|j| j.job.id != id);
        self.schedule_next(ctx);
      }
    }
  }
}

impl Handler<SchedulerWakeUp> for SchedulerActor {
  type Result = ();

  fn handle(&mut self, _msg: SchedulerWakeUp, ctx: &mut Self::Context) {
    let now = Instant::now();

    while let Some(scheduled) = self.heap.peek() {
      if scheduled.execution_time <= now {
        if let Some(scheduled) = self.heap.pop() {
          tracing::debug!("Scheduled job firing: {}", scheduled.job.job_name);
          let executor = Arc::clone(&self.executor);
          let execution_recorder = self.execution_recorder.clone();
          tokio::spawn(async move {
            match executor.execute(scheduled.job, &scheduled.client_id).await {
              Ok(Some(result)) => {
                execution_recorder.do_send(crate::async_tasks::RecordExecution { result });
              }
              Ok(None) => {
                tracing::debug!("Job skipped (already completed)");
              }
              Err(e) => {
                tracing::error!("Job execution failed (wake-up): {}", e);
              }
            }
          });
        }
      } else {
        break;
      }
    }

    self.schedule_next(ctx);
  }
}

impl SchedulerActor {
  fn schedule_next(&self, ctx: &mut Context<Self>) {
    let now = Instant::now();

    let next_fire = self
      .heap
      .peek()
      .map(|t| t.execution_time.saturating_duration_since(now));

    let sleep_duration = next_fire.unwrap_or(Duration::from_secs(3600 * 24 * 365));
    tracing::debug!("Queue state: heap_size={}, next_fire={:?}", self.heap.len(), next_fire);
    tracing::debug!("Sleeping for {:?}", sleep_duration);

    ctx.notify_later(SchedulerWakeUp, sleep_duration);
  }
}

#[cfg(test)]
mod scheduler_tests {
  use std::{
    sync::{
      Arc,
      Mutex,
    },
    time::Duration,
  };

  use actix::prelude::*;
  use async_trait::async_trait;
  use remex_core::db::model::jobs::{
    Enabled,
    ExecutionStatus,
    Job,
    JobType,
  };
  use tokio::time::Instant;

  use super::{
    super::{
      execution::ExecutionResult,
      JobExecutor,
      JobQueueMessage,
    },
    InjectJob,
    SchedulerActor,
    SchedulerWakeUp,
  };

  struct MockJobExecutor {
    calls: Arc<Mutex<Vec<(Job, String)>>>,
  }

  #[async_trait]
  impl JobExecutor for MockJobExecutor {
    async fn execute(
      &self,
      job: Job,
      client_id: &str,
    ) -> Result<Option<ExecutionResult>, crate::Error> {
      self
        .calls
        .lock()
        .unwrap()
        .push((job.clone(), client_id.to_string()));
      Ok(Some(ExecutionResult {
        output: String::new(),
        exit_code: "0".to_string(),
        execution_start: surrealdb::types::Datetime::default(),
        execution_end: Some(surrealdb::types::Datetime::default()),
        job_id: job.id,
        client_id: surrealdb::types::RecordId::new("client", "mock"),
        status: remex_core::db::model::executions::ExecutionStatus::Completed,
      }))
    }
  }

  #[derive(Default)]
  struct MockLocalDb;

  impl Actor for MockLocalDb {
    type Context = Context<Self>;
  }

  impl Handler<crate::async_tasks::RecordExecution> for MockLocalDb {
    type Result = ();
    fn handle(&mut self, _msg: crate::async_tasks::RecordExecution, _ctx: &mut Self::Context) {
    }
  }

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

  #[actix::test]
  async fn immediate_calls_executor() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let executor = Arc::new(MockJobExecutor {
      calls: calls.clone(),
    });
    let mock_db = MockLocalDb.start();
    let addr = SchedulerActor::new(executor, mock_db.recipient()).start();

    let job = make_test_job("immediate-1", "immediate-test");
    addr
      .send(InjectJob(JobQueueMessage::Immediate {
        job: job.clone(),
        client_id: "client-1".to_string(),
      }))
      .await
      .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    let guard = calls.lock().unwrap();
    assert_eq!(guard.len(), 1, "immediate job should execute immediately");
    assert_eq!(guard[0].0.job_name, "immediate-test");
    assert_eq!(guard[0].1, "client-1");
  }

  #[actix::test]
  async fn scheduled_future_queued() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let executor = Arc::new(MockJobExecutor {
      calls: calls.clone(),
    });
    let mock_db = MockLocalDb.start();
    let addr = SchedulerActor::new(executor, mock_db.recipient()).start();

    let job = make_test_job("sched-future-1", "future-job");
    let future_time = Instant::now() + Duration::from_secs(3600);

    addr
      .send(InjectJob(JobQueueMessage::Scheduled {
        job,
        execution_time: future_time,
        client_id: "client-1".to_string(),
      }))
      .await
      .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    let guard = calls.lock().unwrap();
    assert_eq!(guard.len(), 0, "future job should NOT execute yet");
  }

  #[actix::test]
  async fn scheduled_past_executes_immediately() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let executor = Arc::new(MockJobExecutor {
      calls: calls.clone(),
    });
    let mock_db = MockLocalDb.start();
    let addr = SchedulerActor::new(executor, mock_db.recipient()).start();

    let job = make_test_job("sched-past-1", "past-job");
    let past_time = Instant::now() - Duration::from_secs(10);

    addr
      .send(InjectJob(JobQueueMessage::Scheduled {
        job: job.clone(),
        execution_time: past_time,
        client_id: "client-2".to_string(),
      }))
      .await
      .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    let guard = calls.lock().unwrap();
    assert_eq!(guard.len(), 1, "past scheduled job should execute immediately");
    assert_eq!(guard[0].0.job_name, "past-job");
    assert_eq!(guard[0].1, "client-2");
  }

  #[actix::test]
  async fn remove_removes_from_queue() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let executor = Arc::new(MockJobExecutor {
      calls: calls.clone(),
    });
    let mock_db = MockLocalDb.start();
    let addr = SchedulerActor::new(executor, mock_db.recipient()).start();

    let job = make_test_job("remove-1", "remove-job");
    let future_time = Instant::now() + Duration::from_millis(50);

    // Queue a future job
    addr
      .send(InjectJob(JobQueueMessage::Scheduled {
        job,
        execution_time: future_time,
        client_id: "client-1".to_string(),
      }))
      .await
      .unwrap();

    // Remove it
    let rid = surrealdb::types::RecordId::new("job", "remove-1");
    addr
      .send(InjectJob(JobQueueMessage::Remove { id: rid }))
      .await
      .unwrap();

    // Manually trigger wake-up to flush any due jobs
    addr.send(SchedulerWakeUp).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let guard = calls.lock().unwrap();
    assert_eq!(guard.len(), 0, "removed job should not execute");
  }

  #[actix::test]
  async fn wakeup_fires_due_jobs() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let executor = Arc::new(MockJobExecutor {
      calls: calls.clone(),
    });
    let mock_db = MockLocalDb.start();
    let addr = SchedulerActor::new(executor, mock_db.recipient()).start();

    let job = make_test_job("wakeup-1", "wakeup-job");
    let due_time = Instant::now() + Duration::from_millis(50);

    // Queue a job due in 50ms
    addr
      .send(InjectJob(JobQueueMessage::Scheduled {
        job,
        execution_time: due_time,
        client_id: "client-1".to_string(),
      }))
      .await
      .unwrap();

    // Wait for the scheduled wake-up to fire (it's scheduled after 50ms)
    tokio::time::sleep(Duration::from_millis(200)).await;

    let guard = calls.lock().unwrap();
    assert_eq!(guard.len(), 1, "due job should fire via wake-up");
    assert_eq!(guard[0].0.job_name, "wakeup-job");
  }

  #[actix::test]
  async fn wakeup_skips_not_due_jobs() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let executor = Arc::new(MockJobExecutor {
      calls: calls.clone(),
    });
    let mock_db = MockLocalDb.start();
    let addr = SchedulerActor::new(executor, mock_db.recipient()).start();

    let job = make_test_job("not-due-1", "not-due-job");
    let far_future = Instant::now() + Duration::from_secs(3600);

    addr
      .send(InjectJob(JobQueueMessage::Scheduled {
        job,
        execution_time: far_future,
        client_id: "client-1".to_string(),
      }))
      .await
      .unwrap();

    // Manually send wake-up — job is not due yet
    addr.send(SchedulerWakeUp).await.unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    let guard = calls.lock().unwrap();
    assert_eq!(guard.len(), 0, "not-due job should NOT fire via wake-up");
  }
}
