use std::sync::Arc;

use remex_core::{
  codec::{
    ClientRequest,
    ConnectionResponse,
    DisconnectReason,
    ServerResponse,
  },
  db::{
    BearerGrantResponse,
    DbOperator,
  },
};
use surrealdb::types::ToSql;
use tokio::sync::Mutex;

use crate::utils;

pub async fn process_server_msg(
  ctx: Arc<Mutex<crate::db::endpoint::Session>>,
  args_secret: Option<String>,
  client_request_tx: tokio::sync::mpsc::Sender<ClientRequest>,
  mut server_msg_rx: tokio::sync::mpsc::Receiver<ServerResponse>,
) {
  loop {
    tokio::select! {
      msg = server_msg_rx.recv() => {
        let Some(msg) = msg else {
          tracing::info!("Server message channel closed");
          return;
        };

        let mut ctx_lock = ctx.lock().await;
        let authenticated = ctx_lock.tkn.is_some();

        match (msg, authenticated) {
          (ServerResponse::Ping, _) => {
            if let Err(e) = client_request_tx.try_send(ClientRequest::Ping) {
              tracing::error!("Failed to queue Ping reply: {}", e);
            }
            if !authenticated {
              tracing::info!("Attempting to authenticate");
              let iden = match utils::derive_auth(ctx_lock.secret.as_ref(), args_secret.as_ref()) {
                Ok(1) => remex_core::codec::IdentifyType::ClientSecret(
                  ctx_lock.secret.clone().unwrap(),
                  ctx_lock.client_name.clone(),
                  surrealdb::types::RecordId::parse_simple(&ctx_lock.client_id.clone().unwrap()).unwrap(),
                  ctx_lock.hardware_hash.clone(),
                ),
                Ok(2) => remex_core::codec::IdentifyType::Secret(
                  args_secret.clone().unwrap().clone(),
                  ctx_lock.client_name.clone(),
                  ctx_lock.hardware_hash.clone(),
                ),
                Ok(k) => {
                  tracing::error!("Invalid auth derivation: {}", k);
                  std::process::exit(1);
                }
                Err(e) => {
                  tracing::error!("{}", e);
                  std::process::exit(1);
                }
              };
              if let Err(e) = client_request_tx.try_send(
                remex_core::codec::ClientRequest::ConnectionRequest(
                  remex_core::codec::ConnectionRequest::Identify(iden.clone()),
                ),
              ) {
                tracing::error!("Failed to queue Identify request: {}", e);
              }
            }
          }
          (ServerResponse::ConnectionResponse(ConnectionResponse::Disconnect(reason)), _) => {
            match reason {
              DisconnectReason::AuthFailed => {
                tracing::error!("Authentication failed. Removing stored credentials and quitting. Please restart with a valid --secret.");
                let _ = crate::fs::id::remove_id();
                let _ = crate::fs::secret::remove_secret();
                std::process::exit(1);
              }
              DisconnectReason::InvalidClientId => {
                tracing::error!("Invalid client ID. Removing stored credentials and quitting. Please restart with a valid --secret.");
                let _ = crate::fs::id::remove_id();
                let _ = crate::fs::secret::remove_secret();
                std::process::exit(1);
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
          (ServerResponse::JobsResponse(j), true) => {
            tracing::error!("Received jobs response");
          }
          (
            ServerResponse::ConnectionResponse(ConnectionResponse::Authenticated(client_id, token)),
            _,
          ) => {
            tracing::info!("Authenticated and received token: {}", &token.grant.key);
            ctx_lock.client_id = Some(client_id.to_sql());
            ctx_lock.tkn = Some(token.clone());
            ctx_lock.push(&crate::LOCAL_DB).await.unwrap();
          }
          s => {
            tracing::info!("Ignored server response: {:#?}", &s);
          }
        }
      }
    }
  }
}
