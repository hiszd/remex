pub mod jobs;
pub mod local_db;
pub mod remote_db;

use actix::prelude::*;
use remex_core::db::{
  model::{
    executions::Execution,
    groups::Group,
    jobs::Job,
  },
  DbError,
};
use surrealdb::{
  engine::any::Any,
  types::{
    Action,
    RecordId,
  },
  Surreal,
};

use crate::{
  async_tasks::jobs::execution::ExecutionResult,
  db::endpoint::Session,
};

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct ConnectionReady {
  pub db: Option<Surreal<Any>>,
  pub client_id: Option<String>,
}

/// Broadcast when the remote connection is established.
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct RemoteConnected {
  pub client_id: String,
}

/// Broadcast when the remote connection is lost.
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct RemoteDisconnected;

/// Sent from LocalDbActor to RemoteDbActor: push an execution to the remote DB.
#[derive(Message, Clone)]
#[rtype(result = "Result<(), DbError>")]
pub struct PushExecution {
  pub cache_id: String,
  pub execution: Execution,
}

/// Sent from RemoteDbActor to LocalDbActor: mark a local execution as synced.
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct MarkExecutionSynced {
  pub cache_id: String,
}

/// Sent from RemoteDbActor to LocalDbActor: cache a job locally.
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct CacheJob {
  pub job: Job,
  pub client_id: String,
}

/// Sent from RemoteDbActor to LocalDbActor: remove a job from local cache and scheduler.
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct RemoveJob {
  pub job_id: RecordId,
}

/// Sent from RemoteDbActor to LocalDbActor on connect/reconnect: full job + group snapshot.
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct SyncJobsBatch {
  pub jobs: Vec<Job>,
  pub groups: Vec<RecordId>,
  pub client_id: String,
}

/// Sent from RemoteDbActor to LocalDbActor on group LIVE SELECT events.
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct GroupEvent {
  pub group: Group,
  pub action: Action,
  pub client_id: String,
}

/// Sent from SchedulerActor to LocalDbActor: record the result of an execution.
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct RecordExecution {
  pub result: ExecutionResult,
}

/// Request the current session from LocalDbActor.
#[derive(Message, Clone)]
#[rtype(result = "Result<Session, DbError>")]
pub struct GetSession;

/// Save/update session credentials via LocalDbActor.
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub struct SaveSession {
  pub client_id: String,
  pub secret: String,
}

/// Wire up the RemoteDbActor address to LocalDbActor.
/// Sent once after both actors are created (see main.rs pattern).
#[derive(Message)]
#[rtype(result = "()")]
pub struct SetRemoteDbAddr(pub actix::Addr<crate::async_tasks::remote_db::RemoteDbActor>);

/// Wire up the SchedulerActor address to LocalDbActor.
/// Sent once after SchedulerActor is created (see main.rs pattern).
#[derive(Message)]
#[rtype(result = "()")]
pub struct SetSchedulerAddr(pub actix::Addr<crate::async_tasks::jobs::scheduler::SchedulerActor>);
