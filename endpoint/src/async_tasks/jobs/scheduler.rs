use std::{cmp::Ordering, collections::BinaryHeap, time::Duration};

use actix::prelude::*;
use remex_core::db::model::jobs::Job;
use surrealdb::types::ToSql;
use tokio::time::Instant;

use super::JobQueueMessage;

#[derive(Debug, Clone)]
struct ScheduledJob {
  execution_time: Instant,
  job: Job,
  client_id: String,
}

impl Ord for ScheduledJob {
  fn cmp(&self, other: &Self) -> Ordering {
    other.execution_time.cmp(&self.execution_time)
  }
}

impl PartialOrd for ScheduledJob {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl PartialEq for ScheduledJob {
  fn eq(&self, other: &Self) -> bool {
    self.execution_time == other.execution_time && self.job.id == other.job.id
  }
}

impl Eq for ScheduledJob {}

pub struct SchedulerActor {
  heap: BinaryHeap<ScheduledJob>,
}

impl SchedulerActor {
  pub fn new() -> Self {
    SchedulerActor {
      heap: BinaryHeap::new(),
    }
  }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct InjectJob(pub JobQueueMessage);

#[derive(Message)]
#[rtype(result = "()")]
struct SchedulerWakeUp;

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
        tokio::spawn(async move {
          if let Err(e) = super::execution::execute_job(job, &client_id).await {
            tracing::error!("Job execution failed Immediate: {}", e);
          }
        });
      }
      JobQueueMessage::Scheduled { job, execution_time, client_id } => {
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
          tokio::spawn(async move {
            if let Err(e) = super::execution::execute_job(job, &client_id).await {
              tracing::error!("Job execution failed: {}", e);
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
          tokio::spawn(async move {
            if let Err(e) = super::execution::execute_job(scheduled.job, &scheduled.client_id).await {
              tracing::error!("Job execution failed: {}", e);
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

    let next_fire = self.heap
      .peek()
      .map(|t| t.execution_time.saturating_duration_since(now));

    let sleep_duration = next_fire.unwrap_or(Duration::from_secs(3600 * 24 * 365));
    tracing::debug!("Queue state: heap_size={}, next_fire={:?}", self.heap.len(), next_fire);
    tracing::debug!("Sleeping for {:?}", sleep_duration);

    ctx.notify_later(SchedulerWakeUp, sleep_duration);
  }
}
