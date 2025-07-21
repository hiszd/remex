use actix::prelude::*;
use tracing::info;

use crate::server::Server;

pub mod conn;

#[derive(Message)]
#[rtype(result = "Vec<String>")]
pub struct GetLogs {}
impl Handler<GetLogs> for Server {
  type Result = Vec<String>;
  fn handle(&mut self, _: GetLogs, _: &mut Context<Self>) -> Self::Result {
    info!("GetLogsServer");
    futures::executor::block_on(async {
      self.db.send(crate::messages::db::GetLogs {}).await.unwrap()
    })
  }
}

/// Send message to specific room
#[derive(Message)]
#[rtype(result = "String")]
pub struct Message {
  /// Id of the client session
  pub id: String,
  /// Peer message
  pub msg: String,
}
/// Handler for Message message.
impl Handler<Message> for Server {
  type Result = String;
  fn handle(&mut self, msg: Message, _: &mut Context<Self>) -> String {
    info!("Message: {}", msg.msg);
    msg.msg
  }
}

/// Send command to specific room
#[derive(Message)]
#[rtype(result = "String")]
pub struct Command {
  /// Id of the client session
  pub id: String,
  /// Peer message
  pub command: String,
}
/// Handler for Command message.
impl Handler<Command> for Server {
  type Result = String;

  fn handle(&mut self, command: Command, _: &mut Context<Self>) -> String {
    info!("Command: {}", command.command);
    command.command
  }
}
