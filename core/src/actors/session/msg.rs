use actix::{
  Context,
  Handler,
  Message,
};

use crate::codec::{
  ConnectionResponse,
  ServerResponse,
};

/// Force session close
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
  pub reason: crate::codec::DisconnectReason,
}
/// Handler for Disconnect message.
impl Handler<Disconnect> for super::RemexSession {
  type Result = ();
  fn handle(&mut self, disc: Disconnect, _: &mut Context<Self>) -> Self::Result {
    tracing::info!("Sending disconnect to peer");
    // send message to peer
    self
      .framed
      .write(ServerResponse::ConnectionResponse(ConnectionResponse::Disconnect(disc.reason)));
  }
}
