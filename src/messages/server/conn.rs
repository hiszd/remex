use actix::prelude::*;
use tracing::info;

use crate::endpoint::Endpoint;
use crate::server::Server;
use crate::session::RemexSession;

#[derive(Message)]
#[rtype(result = "()")]
pub struct Connect {
  pub identity: Endpoint,
  pub addr: Addr<RemexSession>,
}
/// Handler for Connect message.
///
/// Register new session with the server. The ID is already assigned
impl Handler<Connect> for Server {
  type Result = ();
  fn handle(&mut self, msg: Connect, _ctx: &mut Context<Self>) -> Self::Result {
    self.sessions.insert(msg.identity.clone(), msg.addr.clone()).unwrap();
  }
}

/// Session is disconnected
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
  pub identity: Endpoint,
}
/// Handler for Disconnect message.
impl Handler<Disconnect> for Server {
  type Result = ();
  fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
    info!("Session {:?} disconnected", &msg.identity);
    // remove address
    self.sessions.remove(msg.identity.id.clone().unwrap());
  }
}
