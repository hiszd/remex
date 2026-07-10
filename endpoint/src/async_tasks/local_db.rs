use actix::prelude::*;
use remex_core::db::DbError;

use crate::{
  async_tasks::{
    GetSession,
    MarkExecutionSynced,
    SaveSession,
  },
  db::endpoint::Session,
};

/// Stub LocalDbActor — placeholder until full implementation in Ticket 6.
/// Exists only to satisfy the type dependency for RemoteDbActor.
pub struct LocalDbActor;

impl LocalDbActor {
  pub fn new() -> Self { LocalDbActor }
}

impl Actor for LocalDbActor {
  type Context = Context<Self>;
}

impl actix::Supervised for LocalDbActor {
  fn restarting(&mut self, _ctx: &mut Context<Self>) {
    tracing::info!("LocalDbActor: restarting (stub — no-op)");
  }
}

impl Handler<GetSession> for LocalDbActor {
  type Result = Result<Session, DbError>;

  fn handle(&mut self, _msg: GetSession, _ctx: &mut Context<Self>) -> Self::Result {
    Err(DbError::OperationFailed("LocalDbActor not yet implemented (stub)".into()))
  }
}

impl Handler<MarkExecutionSynced> for LocalDbActor {
  type Result = ();

  fn handle(&mut self, msg: MarkExecutionSynced, _ctx: &mut Context<Self>) {
    tracing::debug!("LocalDbActor stub: MarkExecutionSynced({}) — no-op", msg.cache_id);
  }
}

impl Handler<SaveSession> for LocalDbActor {
  type Result = ();

  fn handle(&mut self, _msg: SaveSession, _ctx: &mut Context<Self>) {
    tracing::debug!("LocalDbActor stub: SaveSession — no-op");
  }
}
