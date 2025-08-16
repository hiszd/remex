use actix::prelude::*;
use tracing::info;

use super::server;
use crate::core::codec::{s2c, DisconnectReason};
use crate::endpoint::executor::Executor;
use crate::endpoint::Endpoint;
use crate::session::RemexSession;

// This message should come from the Db actor.
// It means that the client has been authenticated.
#[derive(Message)]
#[rtype(result = "()")]
pub struct ExecutorList {
  pub identity: Endpoint,
  pub executors: Vec<Executor>,
}
/// Handler for Identified message.
impl Handler<ExecutorList> for RemexSession {
  type Result = ();
  fn handle(&mut self, msg: ExecutorList, _ctx: &mut Context<Self>) -> Self::Result {
    self
      .framed
      .write(s2c::S2C::Exchange(s2c::Exchange::ExecutorList(msg.executors.clone())));
  }
}
