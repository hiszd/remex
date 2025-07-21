use actix::prelude::*;
use tracing::info;

use super::server;
use crate::core::codec::{ClientResponse, DisconnectReason};
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
      self.addr.do_send(server::conn::Disconnect {
        identity: self.identity.clone().unwrap(),
      });
    }
    info!("Sending disconnect to peer with reason: {}", &disc.reason);
    self.framed.write(ClientResponse::Disconnect(disc.reason));
  }
}

/// Message for chat server communications
///
/// New chat session is created
#[derive(Message)]
#[rtype(result = "()")]
pub struct Identified {
  pub identity: Endpoint,
  pub secret: String,
  pub temp: bool,
}
/// Handler for Identified message.
impl Handler<Identified> for RemexSession {
  type Result = ();
  fn handle(&mut self, id: Identified, ctx: &mut Context<Self>) -> Self::Result {
    if id.temp {
      self.identified = true;
    } else {
      self.identified = true;
      info!("Sending auth to peer");
      // send message to peer
      self.framed.write(ClientResponse::Authenticated(id.identity.clone(), id.secret.clone()));
    }
    self.addr.do_send(server::conn::IdChange {
      old_identity: self.identity.clone().unwrap(),
      new_identity: id.identity.clone(),
      addr: ctx.address(),
    });
    self.identity = Some(id.identity.clone());
  }
}
