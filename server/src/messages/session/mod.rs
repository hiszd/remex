use actix::prelude::*;
use remex_core::codec::s2c;
use tracing::info;

use crate::session::RemexSession;

pub mod conn;
pub mod exchange;

/// Message from client to log on host
#[derive(Message)]
#[rtype(result = "()")]
pub struct Message {
  pub msg: String,
}
/// Handler for message.
impl Handler<Message> for RemexSession {
  type Result = ();
  fn handle(&mut self, msg: Message, _: &mut Context<Self>) -> Self::Result {
    if !self.authenticated {
      return;
    }
    info!("Client {:?} sent message: {}", &self.identity, &msg.msg);
  }
}

/// Message for chat server communications
///
/// New chat session is created
#[derive(Message)]
#[rtype(result = "()")]
pub struct Command {
  pub cmd: String,
}
/// Handler for Identified message.
impl Handler<Command> for RemexSession {
  type Result = ();
  fn handle(&mut self, cmd: Command, _: &mut Context<Self>) -> Self::Result {
    info!("session command: {}", &cmd.cmd);
    if !self.authenticated {
      return;
    }
    // send message to peer
    self
      .framed
      .write(s2c::S2C::Conn(s2c::Conn::Command(cmd.cmd)));
  }
}
