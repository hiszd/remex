use std::sync::Arc;

use futures::{
  SinkExt,
  StreamExt,
};
use remex_core::{
  codec::{
    self,
    ClientRequest,
    DisconnectReason,
    ServerResponse,
  },
  db::{
    DbError,
    DbOperator,
  },
};
use surrealdb::types::ToSql;
use tokio::{
  net::TcpStream,
  sync::Mutex,
};

use crate::{
  utils,
  ConnState,
};

pub async fn server_msg_loop(
  ctx: Arc<Mutex<crate::Context>>,
  args_secret: Option<String>,
  args_server: String,
  args_port: String,
  mut client_request_rx: tokio::sync::mpsc::Receiver<ClientRequest>,
) -> Result<(), DbError> {
  // Buffer to hold a message that was popped but failed to send due to disconnect
  let mut pending_request: Option<codec::ClientRequest> = None;

  loop {
    println!("Connecting to server");
    let st = TcpStream::connect(format!("{}:{}", args_server, args_port)).await;
    match st {
      Err(e) => {
        tracing::warn!("Failed to connect to server {}.\nTrying again in 5 seconds", e);
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
      }
      Ok(stream) => {
        tracing::info!("Connected to server. Setting up codec");
        let mut framed = actix_codec::Framed::new(stream, codec::ServerCodec);
        let local_endpoint = crate::db::get_local_endpoint().await?;

        // Flush pending request from a previous failed send before entering the main loop
        if let Some(req) = pending_request.take() {
          if let Err(e) = framed.send(req.clone()).await {
            tracing::error!("Failed to send pending request: {}\n Trying again in 5 seconds", e);
            pending_request = Some(req);
            continue;
          }
        }

        loop {
          tokio::select! {
          msg = framed.next() => {

            if let Some(msg) = msg {
              let mut ctx_lock = ctx.lock().await;
              let authenticated = ctx_lock.authenticated;

              match msg {
                Ok(msg) => {
                  match (msg, authenticated) {
                    (ServerResponse::Ping, _) => {
                      if let Err(e) = framed.send(ClientRequest::Ping).await {
                        tracing::error!("Failed to queue Ping reply: {}", e);
                      }
                      if !authenticated {
                        tracing::debug!("Attempting to authenticate");
                        match utils::derive_auth(ctx_lock.session.secret.as_ref(), args_secret.as_ref()) {
                          Ok(1) => {
                            if let Err(e) = framed.send(
                              remex_core::codec::ClientRequest::SigninClient(
                                ctx_lock.session.secret.clone().unwrap(),
                                ctx_lock.session.client_name.clone(),
                                surrealdb::types::RecordId::parse_simple(&ctx_lock.session.client_id.clone().unwrap()).unwrap(),
                                ctx_lock.session.hardware_hash.clone(),
                              )
                            ).await {
                              tracing::error!("Failed to queue Identify request: {}", e);
                            } else {
                              ctx_lock.state.server_connected = ConnState::Connecting;
                            }
                          }
                          Ok(2) => {
                            if let Err(e) = framed.send(
                              remex_core::codec::ClientRequest::SignupClient(
                                args_secret.clone().unwrap().clone(),
                                ctx_lock.session.client_name.clone(),
                                ctx_lock.session.hardware_hash.clone(),
                              )
                            ).await {
                              tracing::error!("Failed to queue Identify request: {}", e);
                            } else {
                              ctx_lock.state.server_connected = ConnState::Connecting;
                            }
                          }
                          Ok(k) => {
                            tracing::error!("Invalid auth derivation: {}", k);
                            std::process::exit(1);
                          }
                          Err(e) => {
                            tracing::error!("{}", e);
                            std::process::exit(1);
                          }
                        }
                      }
                    }
                    (ServerResponse::Disconnect(reason), _) => {
                      match reason {
                        DisconnectReason::AuthFailed => {
                          tracing::error!("Authentication failed. Removing stored credentials and quitting. Please restart with a valid --secret.");
                          local_endpoint.query("DELETE session;").await.unwrap();
                          ctx_lock.authenticated = false;
                          if ctx_lock.server_secret.is_none() {
                            std::process::exit(1);
                          }
                        }
                        DisconnectReason::InvalidClientId => {
                          tracing::error!("Invalid client ID. Removing stored credentials and quitting. Please restart with a valid --secret.");
                          local_endpoint.query("USE NS remex DB endpoint; DELETE session;").await.unwrap();
                          ctx_lock.authenticated = false;
                          if ctx_lock.server_secret.is_none() {
                            std::process::exit(1);
                          }
                        }
                        DisconnectReason::DuplicateClient => {
                          tracing::error!("Duplicate client ID");
                        }
                        DisconnectReason::HeartbeatFailed => {
                          tracing::error!("Heartbeat failed");
                        }
                        DisconnectReason::Unknown(e) => {
                          tracing::error!("Unknown disconnect reason: {}", e);
                        }
                      }
                    }
                    (
                      ServerResponse::SignedIn(token, secret, server_url),
                      _,
                    ) => {
                      println!("Signed in and received token: {}", &token.grant.key);
                      if let Some(s) = secret {
                        ctx_lock.session.secret = Some(s);
                      }
                      ctx_lock.session.tkn = Some(token.clone());
                      ctx_lock.state.server_connected = ConnState::Connected;
                      ctx_lock.session.db_addr = Some(server_url);
                      ctx_lock.authenticated = true;
                      ctx_lock.session.push(&local_endpoint).await.unwrap();
                    }
                    (
                      ServerResponse::SignedUp(client_id, token, secret, server_url),
                      _,
                    ) => {
                      println!("Signed up and received token: {}", &token.grant.key);
                      ctx_lock.session.secret = Some(secret);
                      ctx_lock.session.tkn = Some(token.clone());
                      ctx_lock.session.client_id = Some(client_id.to_sql());
                      ctx_lock.state.server_connected = ConnState::Connected;
                      ctx_lock.session.db_addr = Some(server_url);
                      ctx_lock.authenticated = true;
                      ctx_lock.session.push(&local_endpoint).await.unwrap();
                    }
                    #[allow(unreachable_patterns)]
                    s => {
                      tracing::debug!("Ignored server response: {:#?}", &s);
                    }
                  }

                }
                Err(e) => {
                  tracing::error!("Failed to receive server message: {}", e);
                }
              }
            } else {
              println!("Server disconnected");
              let mut c = ctx.lock().await;
              c.state.server_connected = ConnState::Reconnecting;
              c.authenticated = false;
              tracing::info!("Changed connection state: {:?}", c.state.server_connected);
              tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
              break;
            }
          }
            msg = client_request_rx.recv() => {
              if let Some(m) = msg {
                if let Err(e) = framed.send(m.clone()).await {
                  tracing::error!("Failed to send server message: {}", e);
                  pending_request = Some(m);
                  break;
                }
              } else {
                tracing::debug!("Server message channel closed");
                break;
              };
            }
          }
        }
      }
    }
  }
}

// pub async fn process_server_msg(
//   ctx: Arc<Mutex<crate::Context>>,
//   args_secret: Option<String>,
//   mut client_request_rx: tokio::sync::mpsc::Receiver<ClientRequest>,
//   mut client_request_tx: tokio::sync::mpsc::Sender<ClientRequest>,
// ) {
//   loop {
//     tokio::select! {
//       msg = client_request_rx.recv() => {
//         let Some(msg) = msg else {
//           tracing::info!("Server message channel closed");
//           return;
//         };
//
//         let mut ctx_lock = ctx.lock().await;
//         let authenticated = ctx_lock.session.tkn.is_some();
//
//         match (msg, authenticated) {
//           (ServerResponse::Ping, _) => {
//             tracing::info!("Received Ping");
//             if let Err(e) = client_request_tx.try_send(ClientRequest::Ping) {
//               tracing::error!("Failed to queue Ping reply: {}", e);
//             }
//             if !authenticated {
//               tracing::info!("Attempting to authenticate");
//               match utils::derive_auth(ctx_lock.session.secret.as_ref(), args_secret.as_ref()) {
//                             Ok(1) => {
//               if let Err(e) = client_request_tx.try_send(
//                 remex_core::codec::ClientRequest::SigninClient(
//                   ctx_lock.session.secret.clone().unwrap(),
//                   ctx_lock.session.client_name.clone(),
//                   surrealdb::types::RecordId::parse_simple(&ctx_lock.session.client_id.clone().unwrap()).unwrap(),
//                   ctx_lock.session.hardware_hash.clone(),
//                     )
//               ) {
//                 tracing::error!("Failed to queue Identify request: {}", e);
//               } else {
//                 ctx_lock.state = crate::State::Authenticating;
//               }
//                             }
//                 Ok(2) => {
//                   if let Err(e) = client_request_tx.try_send(
//                 remex_core::codec::ClientRequest::SignupClient(
//                   args_secret.clone().unwrap().clone(),
//                   ctx_lock.session.client_name.clone(),
//                   ctx_lock.session.hardware_hash.clone(),
//                     )
//               ) {
//                 tracing::error!("Failed to queue Identify request: {}", e);
//               } else {
//                 ctx_lock.state = crate::State::Authenticating;
//               }
//                 }
//                 Ok(k) => {
//                   tracing::error!("Invalid auth derivation: {}", k);
//                   std::process::exit(1);
//                 }
//                 Err(e) => {
//                   tracing::error!("{}", e);
//                   std::process::exit(1);
//                 }
//               }
//             }
//           }
//           (ServerResponse::Disconnect(reason), _) => {
//             match reason {
//               DisconnectReason::AuthFailed => {
//                 tracing::error!("Authentication failed. Removing stored credentials and quitting. Please restart with a valid --secret.");
//                 let _ = crate::fs::id::remove_id();
//                 let _ = crate::fs::secret::remove_secret();
//                 std::process::exit(1);
//               }
//               DisconnectReason::InvalidClientId => {
//                 tracing::error!("Invalid client ID. Removing stored credentials and quitting. Please restart with a valid --secret.");
//                 let _ = crate::fs::id::remove_id();
//                 let _ = crate::fs::secret::remove_secret();
//                 std::process::exit(1);
//               }
//               DisconnectReason::DuplicateClient => {
//                 tracing::error!("Duplicate client ID");
//               }
//               DisconnectReason::HeartbeatFailed => {
//                 tracing::error!("Heartbeat failed");
//               }
//               DisconnectReason::Unknown(e) => {
//                 tracing::error!("Unknown disconnect reason: {}", e);
//               }
//             }
//           }
//           (
//             ServerResponse::SignedIn(token, secret),
//             _,
//           ) => {
//             tracing::info!("Authenticated and received token: {}", &token.grant.key);
//             if let Some(s) = secret {
//               ctx_lock.session.secret = Some(s);
//             }
//             ctx_lock.session.tkn = Some(token.clone());
//             ctx_lock.state = crate::State::Connected;
//             ctx_lock.session.push(&crate::LOCAL_DB).await.unwrap();
//           }
//           (
//             ServerResponse::SignedUp(client_id, token, secret),
//             _,
//           ) => {
//             tracing::info!("Authenticated and received token: {}", &token.grant.key);
//             ctx_lock.session.secret = Some(secret);
//             ctx_lock.session.tkn = Some(token.clone());
//             ctx_lock.session.client_id = Some(client_id.to_sql());
//             ctx_lock.state = crate::State::Connected;
//             ctx_lock.session.push(&crate::LOCAL_DB).await.unwrap();
//           }
//           #[allow(unreachable_patterns)]
//           s => {
//             tracing::info!("Ignored server response: {:#?}", &s);
//           }
//         }
//       }
//     }
//   }
// }
