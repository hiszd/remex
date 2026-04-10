//! `RemexServer` is an actor. It maintains list of connected client sessions.

use std::{
  collections::HashMap,
  sync::Arc,
};

use actix::prelude::*;
use surrealdb::{
  engine::any::Any,
  Surreal,
};
use tokio::sync::Mutex;

use crate::sessionmap::SessionMap;

pub mod msg;

pub struct ClientSessionInfo {
  pub credential: String,
  pub client_name: String,
}

// pub struct JwtRefreshState {
//   pub client_id: String,
//   pub credential: String,
//   pub session_addr: Addr<crate::actors::session::RemexSession>,
//   pub pending_jwt_id: Option<String>,
// }

// pub async fn start_jwt_refresh_pusher(
//   db: Surreal<Client>,
//   client_sessions: Arc<Mutex<HashMap<String, ClientSessionInfo>>>,
// ) {
//   loop {
//     tokio::time::sleep(Duration::from_secs(3600)).await;
//
//     let sessions = client_sessions.lock().await;
//     tracing::info!("JWT refresh: pushing new JWTs to {} connected endpoints", sessions.len());
//
//     for (client_id, info) in sessions.iter() {
//       let credential = info.credential.clone();
//       let client_name = info.client_name.clone();
//       let session_addr = info.session_addr.clone();
//
//       let token_result = db
//         .signin(Record {
//           namespace: "remex",
//           database: "remex",
//           access: "endpoint",
//           params: EndpointCreds {
//             credential: credential.clone(),
//             client_name: client_name.clone(),
//           },
//         })
//         .await;
//
//       match token_result {
//         Ok(token) => {
//           let new_jwt = token.access.as_insecure_token().to_string();
//           let new_jwt_id = token.id.to_string();
//
//           session_addr.do_send(ServerResponse::ConnectionResponse(
//             ConnectionResponse::RefreshJwt {
//               new_jwt,
//               jwt_id: new_jwt_id,
//             },
//           ));
//
//           tracing::info!("JWT refresh: pushed new JWT to endpoint {}", client_id);
//         }
//         Err(e) => {
//           tracing::error!("JWT refresh: failed to generate JWT for {}: {}", client_id, e);
//         }
//       }
//     }
//   }
// }

/// `RemexServer` manages connected clients and keeps track of the currently connected ones.
pub struct RemexServer {
  pub sessions: SessionMap<String>,
  pub migrated: bool,
  pub secret: Option<String>,
  pub db: Option<Surreal<Any>>,
  pub client_sessions: Arc<Mutex<HashMap<String, ClientSessionInfo>>>,
}

/// Make actor from `RemexServer`
impl Actor for RemexServer {
  /// We are going to use simple Context, we just need ability to communicate
  /// with other actors.
  type Context = Context<Self>;
  fn started(&mut self, _ctx: &mut Self::Context) {
    self.migrated = true;
    tracing::info!("Server started with database connection");
  }
}
