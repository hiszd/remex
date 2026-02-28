//! `RemexServer` is an actor. It maintains list of connection client session.
//! And manages available rooms. Peers send messages to other peers in same
//! room through `RemexServer`.

use actix::prelude::*;

use crate::actors::session;

// NOTE: I need to handle messages here that are sent to the Server.
// Since the client is connecting with the DB directly, we won't need to include that communication
// here.
// We can handle messages here that are sent to the Server from the Session actor and that's it.
// Let's talk about what those might be.

/// Force session close
#[derive(Message)]
#[rtype(result = "()")]
pub struct ClientDisconnect {
  pub id: String,
  pub reason: crate::codec::DisconnectReason,
}
/// Handler for Disconnect message.
impl Handler<ClientDisconnect> for super::RemexServer {
  type Result = ();
  fn handle(&mut self, msg: ClientDisconnect, _: &mut Context<Self>) -> Self::Result {
    self.sessions.remove(&msg.id);
    tracing::info!("Client disconnected: {} with reason: {:?}", &msg.id, &msg.reason);
  }
}

/// Connect a new client
#[derive(Message)]
#[rtype(result = "Result<(), crate::codec::DisconnectReason>")]
pub struct ClientConnect {
  pub client: crate::db::model::server::clients::Client,
  pub addr: actix::Addr<session::RemexSession>,
}
/// Handler for Connect message.
impl Handler<ClientConnect> for super::RemexServer {
  type Result = Result<(), crate::codec::DisconnectReason>;
  fn handle(&mut self, msg: ClientConnect, _: &mut Context<Self>) -> Self::Result {
    if self.sessions.exists(&msg.client.id) {
      tracing::warn!(
        client_id = %msg.client.id,
        client_name = %msg.client.client_name,
        "DUPLICATE CLIENT CONNECTION DENIED: a client with id '{}' (name: '{}') attempted to \
         connect while another session with the same id is already active. \
         This may indicate a misconfiguration or unauthorized access attempt. \
         Review client credentials and endpoint deployments.",
        &msg.client.id,
        &msg.client.client_name,
      );
      return Err(crate::codec::DisconnectReason::DuplicateClient);
    }

    tracing::info!("New client connected: {}", &msg.client.id);
    match self.sessions.insert(msg.client.id, msg.addr.clone()) {
      Ok(_) => Ok(()),
      Err(e) => {
        let err = format!("Session insert error: {}", e);
        tracing::error!("{}", &err);
        Err(crate::codec::DisconnectReason::Unknown(err))
      }
    }
  }
}

/// Connect a new client
#[derive(Message)]
#[rtype(result = "anyhow::Result<String>")]
pub struct GetSecret {}
/// Handler for Connect message.
impl Handler<GetSecret> for super::RemexServer {
  type Result = anyhow::Result<String>;
  fn handle(&mut self, _msg: GetSecret, _: &mut Context<Self>) -> Self::Result {
    // FIXME: handle empyy value
    return Ok(self.secret.clone().unwrap());
  }
}
