//! `RemexServer` is an actor. It maintains list of connection client session.
//! And manages available rooms. Peers send messages to other peers in same
//! room through `RemexServer`.

use actix::prelude::*;
use tracing::{error, info};

use crate::actors::server::Server;
use crate::actors::session;

/// Message for chat server communications
///
/// New chat session is created
#[derive(Message)]
#[rtype(result = "()")]
pub struct Connect {
  pub id: Option<String>,
  pub client_name: String,
  pub addr: Addr<session::RemexSession>,
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

/// DB Client identified
///
/// New chat session is created
#[derive(Message)]
#[rtype(result = "()")]
pub struct DbClientIdentified {
  pub id: uuid::Uuid,
  pub client_name: String,
  pub secret: String,
  pub addr: Addr<session::RemexSession>,
}
/// Handler for DbClientIDentified message.
///
/// Change ID for session
impl Handler<DbClientIdentified> for Server {
  type Result = ();
  fn handle(&mut self, db: DbClientIdentified, _: &mut Context<Self>) -> Self::Result {
    info!("Database client id being created with id {}", &db.id);
    match self.sessions.insert(db.id, db.addr.clone()) {
      Err(e) => error!("Could not create session with id: {}", e),
      _ => {
        db.addr.do_send(crate::actors::session::Identified {
          id: db.id,
          name: db.client_name,
        });
      }
    }
  }
}

/// Handler for Connect message.
///
/// Register new session and assign unique id to this session
impl Handler<Connect> for Server {
  type Result = ();

  fn handle(&mut self, msg: Connect, ctx: &mut Context<Self>) -> Self::Result {
    // TODO: make this try and enroll based on a previous id
    let db = self.db.clone();
    info!("Session connection made with client_name {}", &msg.client_name);

    let futr = async move {
      db.send(crate::db::NewClient {
        id: msg.id,
        client_name: msg.client_name,
        addr: msg.addr.clone(),
      })
      .await
      .unwrap()
      .unwrap();
    };
    let fut = actix::fut::wrap_future::<_, Self>(futr);
    ctx.spawn(fut);
  }
}

/// Session is disconnected
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
  pub id: String,
}
/// Handler for Disconnect message.
impl Handler<Disconnect> for Server {
  type Result = ();
  fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
    info!("Session {} disconnected", &msg.id);
    // remove address
    self.sessions.remove(&uuid::Uuid::parse_str(&msg.id.clone()).unwrap());
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

/// Handler for Message message.
impl Handler<Message> for Server {
  type Result = String;

  fn handle(&mut self, msg: Message, _: &mut Context<Self>) -> String {
    info!("Message: {}", msg.msg);
    msg.msg
  }
}
