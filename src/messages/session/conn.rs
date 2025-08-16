use actix::prelude::*;
use tracing::info;

use super::server;
use crate::core::codec::{s2c, DisconnectReason};
use crate::endpoint::Endpoint;
use crate::session::RemexSession;

/// Force session close
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
  pub reason: DisconnectReason,
}
/// Handler for Disconnect message.
impl Handler<Disconnect> for RemexSession {
  type Result = ();
  fn handle(&mut self, disc: Disconnect, _: &mut Context<Self>) -> Self::Result {
    if self.identity.is_some() {
      info!("Sending disconnect to server with reason: {}", &disc.reason);
      self.server.do_send(server::conn::Disconnect {
        identity: self.identity.clone().unwrap(),
      });
    }
    info!("Sending disconnect to peer with reason: {}", &disc.reason);
    self
      .framed
      .write(s2c::S2C::Conn(s2c::Conn::Disconnect(disc.reason)));
  }
}

// This message should come from the Db actor.
// It means that the client has been authenticated.
#[derive(Message)]
#[rtype(result = "()")]
pub struct Authenticated {
  pub identity: Endpoint,
  pub secret: String,
}
/// Handler for Identified message.
impl Handler<Authenticated> for RemexSession {
  type Result = ();
  fn handle(&mut self, id: Authenticated, ctx: &mut Context<Self>) -> Self::Result {
    self.authenticated = true;
    // send message to peer
    self
      .framed
      .write(s2c::S2C::Conn(s2c::Conn::Authenticated(id.identity.clone(), id.secret.clone())));
    // send message to peer
    // register self in Remex server. `AsyncContext::wait` register
    // future within context, but context waits until this future resolves
    // before processing any other events.
    let addr = ctx.address();
    self
      .server
      .send(server::conn::Connect {
        identity: id.identity.clone(),
        addr: addr.clone(),
      })
      .into_actor(self)
      .then(|res, _, ctx| {
        match res {
          Ok(_) => {}
          // something is wrong with chat server
          _ => ctx.stop(),
        }
        actix::fut::ready(())
      })
      .wait(ctx);
    self.identity = Some(id.identity.clone());
  }
}
