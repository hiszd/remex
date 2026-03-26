//ENDPOINT

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
};
use tokio::{
  net::TcpStream,
  sync::Mutex,
};

mod async_tasks;
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
struct CacheJob {
  locked: bool,
  job_name: String,
}

#[derive(Debug, Clone)]
struct Cache {
  jobs: Vec<CacheJob>,
}

#[derive(Debug, Clone)]
struct Context {
  id: Option<String>,
  secret: Option<String>,
  name: String,
  authenticated: bool,
  auth_type: Option<codec::IdentifyType>,
  authentication_used: Option<codec::IdentifyType>,
  jobs_last_requested: Option<std::time::Instant>,
  cache: Cache,
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
  tracing_subscriber::fmt::init();
  tracing::info!("Running client");

  /* ********** VARIABLE INITIALIZATION ********** */

  // Check if both ID and secret are saved
  let id_result = fs::id::get_id()?;
  let secret_result = fs::secret::get_secret()?;
  // Get the machine's hardware hash
  let hw_hash = machine_uid::get().unwrap();
  let args = Args::parse();
  let mut ctx_data = Context {
    id: None,
    secret: None,
    name: gethostname().to_string_lossy().to_string(),
    authenticated: false,
    auth_type: None,
    authentication_used: None,
    jobs_last_requested: None,
    cache: Cache { jobs: vec![] },
  };
  ctx_data.auth_type = match (id_result, secret_result, args.secret.clone()) {
    // if using the server secret for auth, ensure that the ID and secret are removed first
    (_, _, Some(secret)) => {
      fs::id::remove_id().unwrap();
      fs::secret::remove_secret().unwrap();
      Some(codec::IdentifyType::Secret(secret, ctx_data.name.clone(), hw_hash))
    }
    (Some(id), Some(secret), _) => {
      // Both ID and secret are found, continue normally
      Some(codec::IdentifyType::ClientSecret(secret, ctx_data.name.clone(), id, hw_hash))
    }
    (_, _, None) => {
      panic!("Neither ID nor secret found. Please provide a secret using the --secret flag");
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
  tokio::spawn(async_tasks::jobs::jobs_check(ctx.clone(), client_request_tx.clone()));
  tokio::spawn(async_tasks::jobs::jobs_exec(ctx.clone(), client_request_tx.clone()));

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
                let authenticated = ctx_lock.authenticated;
                match (m, authenticated) {
                  (ServerResponse::Ping, _) => {
                    if let Err(e) = client_request_tx.try_send(ClientRequest::Ping) {
                      tracing::error!("Failed to queue Ping reply: {}", e);
                    }
                    if !ctx_lock.authenticated {
                      tracing::info!("Attempting to authenticate");
                      let iden = ctx_lock.auth_type.clone().unwrap();
                      if let Err(e) = client_request_tx.try_send(
                        codec::ClientRequest::ConnectionRequest(
                          codec::ConnectionRequest::Identify(iden.clone()),
                        ),
                      ) {
                        tracing::error!("Failed to queue Identify request: {}", e);
                      }
                      ctx_lock.authentication_used = Some(iden);
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
                    use codec::JobsResponse;
                    tracing::info!("Received jobs response");
                    match j {
                      JobsResponse::ReceiveJobs(_jobs) => {
                        // TODO: Cache these for the life of the program and update them when a new
                        // one is added.
                        tracing::info!("Received jobs (placeholder)");
                        ctx_lock.cache.jobs = vec![];
                      }
                      j => {
                        tracing::info!("Ignored jobs response: {:#?}", &j);
                      }
                    }
                  }
                  (
                    ServerResponse::ConnectionResponse(ConnectionResponse::Authenticated(id, secret)),
                    _,
                  ) => {
                    tracing::info!("Authenticated with id: {}, secret: {}", &id, &secret);
                    ctx_lock.id = Some(id.clone());
                    ctx_lock.secret = Some(secret.clone());
                    ctx_lock.authenticated = true;
                    fs::id::save_id(id).unwrap();
                    fs::secret::save_secret(secret).unwrap();
                    if let Err(e) = client_request_tx.try_send(
                      codec::ClientRequest::JobsRequest(codec::JobsRequest::All),
                    ) {
                      tracing::error!("Failed to queue JobsRequest: {}", e);
                    }
                    ctx_lock.jobs_last_requested = Some(std::time::Instant::now());
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
      ctx.lock().await.authenticated = false;
    }
  }
}