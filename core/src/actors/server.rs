//! `RemexServer` is an actor. It maintains list of connected client sessions.

use actix::prelude::*;

use crate::sessionmap::SessionMap;

pub mod msg;

/// `RemexServer` manages connected clients and keeps track of the currently connected ones.
// TODO: store more than just the session ID from each client. Maybe things like:
// time connection started, ip address, etc...
pub struct RemexServer {
  pub sessions: SessionMap<String>,
  pub migrated: bool,
  pub secret: Option<String>,
}

/// Make actor from `RemexServer`
impl Actor for RemexServer {
  /// We are going to use simple Context, we just need ability to communicate
  /// with other actors.
  type Context = Context<Self>;
  fn started(&mut self, _ctx: &mut Context<Self>) {
    self.migrated = false;
    futures::executor::block_on(async {
      let _ = crate::db::get_db();
      self.migrated = true;
      tracing::info!("Database connected");
    });
  }
}
