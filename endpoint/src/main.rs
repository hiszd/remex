//ENDPOINT

use std::sync::LazyLock;

use clap::Parser;
use futures_util::{
  SinkExt as _,
  StreamExt as _,
};
use gethostname::gethostname;
use remex_core::{
  codec::{
    self,
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
use surrealdb::{
  engine::{
    local::{
      Db,
      SurrealKv,
    },
    remote::ws::Client,
  },
  types::ToSql,
};
use tokio::{
  net::TcpStream,
  sync::Mutex,
};

mod async_tasks;
mod db;
mod fs;
mod utils;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
  /// Secret to use for authentication
  #[clap(long, env = "REMEX_SECRET")]
  secret: Option<String>,
  /// Server IP to connect to
  #[clap(long, env = "REMEX_SERVER")]
  server: String,
  /// Server IP to connect to
  #[clap(long, env = "REMEX_PORT", default_value = "4269")]
  port: String,
}

#[derive(Debug, Clone)]
struct Context {
  id: Option<surrealdb::types::RecordId>,
  name: String,
  hardware_hash: String,
  authenticated: bool,
  token: Option<BearerGrantResponse>,
  secret: Option<String>,
}

// #[derive(Debug, Clone)]
// enum State {
//     Initializing;
// }

static LOCAL_DB: LazyLock<surrealdb::Surreal<Db>> = LazyLock::new(surrealdb::Surreal::init);
static REMOTE_DB: LazyLock<surrealdb::Surreal<Client>> = LazyLock::new(surrealdb::Surreal::init);

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
  tracing_subscriber::fmt::init();
  tracing::info!("Running client");

  // WARN: NEW CODE

  LOCAL_DB.connect::<SurrealKv>("endpoint.db").await?;
  db::endpoint::Session::migrate(&LOCAL_DB).await?;

  tracing::info!("Pulling most recent session");
  let session: Option<db::endpoint::Session> = LOCAL_DB
    .query("USE NS remex DB endpoint; SELECT * FROM session ORDER BY updated_at DESC LIMIT 1;")
    .await?
    .check()?
    .take(1)?;

  /* ********** VARIABLE INITIALIZATION ********** */

  let hardware_hash = machine_uid::get().unwrap();
  let args = Args::parse();
  let ctx_data = match session {
    Some(session) => {
      tracing::info!("Using existing session");
      session
    }
    None => {
      let s = db::endpoint::Session::create(
        db::endpoint::SessionData {
          client_id: None,
          hardware_hash: Some(hardware_hash.clone()),
          client_name: Some(gethostname().to_string_lossy().to_string()),
          db_addr: None,
          tkn: None,
          secret: None,
        },
        &LOCAL_DB,
      )
      .await?
      .unwrap();
      tracing::info!("New session created: {:#?}", &s);
      s
    }
  };
  let ctx = std::sync::Arc::new(Mutex::new(ctx_data));

  /* ********** INPUT VALIDATION ********** */

  // Validate secret length
  if let Some(sec) = args.secret.clone() {
    if sec.len() < 64 {
      panic!("Secret must be at least 64 characters long");
    }
  }

  /* ********** MAIN LOOP ********** */

  // Create bounded channel for outgoing requests with backpressure
  let (client_request_tx, mut client_request_rx) =
    tokio::sync::mpsc::channel::<codec::ClientRequest>(1000);

  // Buffer to hold a message that was popped but failed to send due to disconnect
  let mut pending_request: Option<codec::ClientRequest> = None;

  // spawn threads to request new jobs and execute them outside of the reconnection loop
  // so they keep generating messages even when the connection is down.
  // tokio::spawn(async_tasks::jobs::jobs_check(ctx.clone(), client_request_tx.clone()));
  // tokio::spawn(async_tasks::jobs::jobs_exec(ctx.clone(), client_request_tx.clone()));

  loop {
    // continually try and connect to the server every 5 seconds until we succeed
    // TODO: Maybe handle errors that aren't "Connection Refused" differently in the future
    let st = TcpStream::connect(format!("{}:{}", args.server, args.port)).await;
    if st.is_err() {
      tracing::warn!("Failed to connect to server. Trying again in 5 seconds 1");
      tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    } else {
      let stream = st.unwrap();
      let mut framed = actix_codec::Framed::new(stream, codec::ServerCodec);

      // Flush pending request from a previous failed send before entering the main loop
      if let Some(req) = pending_request.take() {
        if let Err(e) = framed.send(req.clone()).await {
          tracing::error!("Failed to send pending request: {}\n Trying again in 5 seconds", e);
          pending_request = Some(req);
          tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
          continue;
        }
      }

      /* ********** SERVER MESSAGE LOOP ********** */

      loop {
        tokio::select! {
          msg = framed.next() => {
            let Some(msg) = msg else {
              tracing::info!("Server disconnected");
              break;
            };
            match msg {
              Ok(m) => {
                let mut ctx_lock = ctx.lock().await;
                let authenticated = ctx_lock.tkn.is_some();
                match (m, authenticated) {
                  (ServerResponse::Ping, _) => {
                    if let Err(e) = client_request_tx.try_send(ClientRequest::Ping) {
                      tracing::error!("Failed to queue Ping reply: {}", e);
                    }
                    if !authenticated {
                      tracing::info!("Attempting to authenticate");
                      let iden = match utils::derive_auth(ctx_lock.secret.as_ref(), args.secret.as_ref()) {
                        Ok(1) => codec::IdentifyType::ClientSecret(ctx_lock.secret.clone().unwrap(), ctx_lock.client_name.clone(), surrealdb::types::RecordId::parse_simple(&ctx_lock.client_id.clone().unwrap()).unwrap(), ctx_lock.hardware_hash.clone()),
                        Ok(2) => codec::IdentifyType::Secret(args.secret.clone().unwrap().clone(), ctx_lock.client_name.clone(), ctx_lock.hardware_hash.clone()),
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
                        codec::ClientRequest::ConnectionRequest(
                          codec::ConnectionRequest::Identify(iden.clone()),
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
                        let _ = fs::id::remove_id();
                        let _ = fs::secret::remove_secret();
                        std::process::exit(1);
                      }
                      DisconnectReason::InvalidClientId => {
                        tracing::error!("Invalid client ID. Removing stored credentials and quitting. Please restart with a valid --secret.");
                        let _ = fs::id::remove_id();
                        let _ = fs::secret::remove_secret();
                        std::process::exit(1);
                      }
                      DisconnectReason::DuplicateClient => {
                        tracing::error!("Duplicate client ID\n Trying again in 5 seconds");
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        break;
                      }
                      DisconnectReason::HeartbeatFailed => {
                        tracing::error!("Heartbeat failed\n Trying again in 5 seconds");
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        break;
                      }
                      DisconnectReason::Unknown(e) => {
                        tracing::error!("Unknown disconnect reason: {}\n Trying again in 5 seconds", e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        break;
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
                    ctx_lock.push(&LOCAL_DB).await.unwrap();
                  }
                  s => {
                    tracing::info!("Ignored server response: {:#?}", &s);
                  }
                }
              }
              Err(e) => {
                tracing::info!("Client error: {}", e);
                break;
              }
            }
          },
          req = client_request_rx.recv() => {
            if let Some(req) = req {
              if let Err(e) = framed.send(req.clone()).await {
                tracing::error!("Failed to send request: {:?}\n Error: {}", &req, &e);
                pending_request = Some(req);
                break;
              }
            }
          }
        }
      }
      let mut c = ctx.lock().await;
      c.tkn = None;
      c.push(&LOCAL_DB).await?;
    }
  }
}
