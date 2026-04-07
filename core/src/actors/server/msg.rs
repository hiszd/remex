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
  pub client_id: String,
  pub client_name: String,
  pub addr: actix::Addr<session::RemexSession>,
}
/// Handler for Connect message.
impl Handler<ClientConnect> for super::RemexServer {
  type Result = Result<(), crate::codec::DisconnectReason>;
  fn handle(&mut self, msg: ClientConnect, _: &mut Context<Self>) -> Self::Result {
    if self.sessions.exists(&msg.client_id) {
      tracing::warn!(
        client_id = %msg.client_id,
        client_name = %msg.client_name,
        "DUPLICATE CLIENT CONNECTION DENIED: a client with id '{}' (name: '{}') attempted to \
         connect while another session with the same id is already active. \
         This may indicate a misconfiguration or unauthorized access attempt. \
         Review client credentials and endpoint deployments.",
        &msg.client_id,
        &msg.client_name,
      );
      return Err(crate::codec::DisconnectReason::DuplicateClient);
    }

    // TODO: Implement JWT pull from the database and sending back to the client
    tracing::info!("New client connected: {}", &msg.client_id);
    match self.sessions.insert(msg.client_id, msg.addr.clone()) {
      Ok(_) => Ok(()),
      Err(e) => {
        let err = format!("Session insert error: {}", e);
        tracing::error!("{}", &err);
        Err(crate::codec::DisconnectReason::Unknown(err))
      }
    }
  }
}

// TODO: Implement a JWT refresh message that the client can initiate
// Some ideas for this might include using the existing, still vaild JWT to authenticate with the
// database, then pulling the new JWT and sending that along.
// Or maybe store the JWT for each client in the self.sessions HashMap and compare the JWT with
// that. The benefit of this approach is that you could use the expired JWT from the client to
// request a new one from the server.
//
// Acually scratch that. I think once the client has a valid JWT it can just pull a new one from the
// Database server itself without interacting with the server. The server will just be the method
// for extablishing the initial connection with the Database server and restoring connection should
// the endpoint let it's JWT expire.
