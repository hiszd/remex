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
  /// SurrealDB URL
  #[clap(long, env = "SURREALDB_URL", default_value = "ws://192.168.10.87:8090")]
  surrealdb_url: String,
  /// SurrealDB Namespace
  #[clap(long, env = "SURREALDB_NAMESPACE", default_value = "remex")]
  surrealdb_namespace: String,
  /// SurrealDB Database
  #[clap(long, env = "SURREALDB_DATABASE", default_value = "remex")]
  surrealdb_database: String,
}

#[derive(Debug, Clone)]
struct CacheJob {
  locked: bool,
  job: remex_core::db::surreal::models::Job,
}

#[derive(Debug, Clone)]
struct Cache {
  jobs: Vec<CacheJob>,
}

#[derive(Debug, Clone)]
struct Context {
  id: Option<String>,
  secret: Option<String>,
  jwt_token: Option<String>,
  db: Option<db::Db>,
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

  let id_result = fs::id::get_id()?;
  let secret_result = fs::secret::get_secret()?;
  let hw_hash = machine_uid::get().unwrap();
  let args = Args::parse();
  let mut ctx_data = Context {
    id: None,
    secret: None,
    jwt_token: None,
    db: None,
    name: gethostname().to_string_lossy().to_string(),
    authenticated: false,
    auth_type: None,
    authentication_used: None,
    jobs_last_requested: None,
    cache: Cache { jobs: Vec::new() },
  };
  ctx_data.auth_type = match (id_result, secret_result, args.secret.clone()) {
    (_, _, Some(secret)) => {
      fs::id::remove_id().unwrap();
      fs::secret::remove_secret().unwrap();
      Some(codec::IdentifyType::Secret(secret, ctx_data.name.clone(), hw_hash))
    }
    (Some(id), Some(secret), _) => {
      Some(codec::IdentifyType::ClientSecret(secret, ctx_data.name.clone(), id, hw_hash))
    }
    (_, _, None) => {
      panic!("Neither ID nor secret found. Please provide a secret using the --secret flag");
    }
  };
  let ctx = std::sync::Arc::new(Mutex::new(ctx_data));

  /* ********** INPUT VALIDATION ********** */

  if let Some(sec) = args.secret.clone() {
    if sec.len() < 64 {
      panic!("Secret must be at least 64 characters long");
    }
  }

  /* ********** MAIN LOOP ********** */

  let (client_request_tx, mut client_request_rx) =
    tokio::sync::mpsc::channel::<codec::ClientRequest>(1000);

  let mut pending_request: Option<codec::ClientRequest> = None;

  tokio::spawn(async_tasks::jobs::jobs_check(ctx.clone(), client_request_tx.clone()));
  tokio::spawn(async_tasks::jobs::jobs_exec(ctx.clone(), client_request_tx.clone()));
  tokio::spawn(async_tasks::server_monitor::server_monitor(ctx.clone()));

  let (jwt_response_tx, jwt_response_rx) = tokio::sync::mpsc::channel::<codec::ServerResponse>(10);
  let jwt_response_tx_for_loop = jwt_response_tx.clone();
  tokio::spawn(async_tasks::jwt::jwt_refresh(ctx.clone(), client_request_tx.clone(), jwt_response_rx));

  loop {
    let st = TcpStream::connect(format!("{}:{}", args.server, args.port)).await;
    if st.is_err() {
      tracing::warn!("Failed to connect to server. Trying again in 5 seconds 1");
      tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    } else {
      let stream = st.unwrap();
      let mut framed = actix_codec::Framed::new(stream, codec::ServerCodec);

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
                  (ServerResponse::JwtRefreshed(token), _) => {
                    if let Err(e) = jwt_response_tx_for_loop.send(ServerResponse::JwtRefreshed(token.clone())).await {
                      tracing::error!("Failed to send JWT refresh to handler: {}", e);
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
                      JobsResponse::ReceiveJobs(jobs) => {
                        tracing::info!("Received {} jobs", jobs.len());
                        ctx_lock.cache.jobs = jobs
                          .iter()
                          .map(|j| {
                            tracing::info!("Job: {}, status: {:?}", j.job_name, j.job_status);
                            CacheJob {
                              locked: false,
                              job: j.clone(),
                            }
                          })
                          .collect();
                      }
                      j => {
                        tracing::info!("Ignored jobs response: {:#?}", &j);
                      }
                    }
                  }
                  (
                    ServerResponse::ConnectionResponse(ConnectionResponse::Authenticated(id, secret, jwt_token)),
                    _,
                  ) => {
                    tracing::info!("Authenticated with id: {}, secret: {}, jwt_token: {}", &id, &secret, &jwt_token);
                    ctx_lock.id = Some(id.clone());
                    ctx_lock.secret = Some(secret.clone());
                    ctx_lock.jwt_token = Some(jwt_token.clone());
                    ctx_lock.authenticated = true;
                    fs::id::save_id(id).unwrap();
                    fs::secret::save_secret(secret).unwrap();
                    
                    let db_url = args.surrealdb_url.clone();
                    let db_namespace = args.surrealdb_namespace.clone();
                    let db_database = args.surrealdb_database.clone();
                    let jwt = jwt_token.clone();
                    let db_clone = ctx.clone();
                    
                    tokio::spawn(async move {
                      match db::connect(&db_url, &db_namespace, &db_database, &jwt).await {
                        Ok(database) => {
                          tracing::info!("Connected to SurrealDB");
                          let mut ctx_write = db_clone.lock().await;
                          ctx_write.db = Some(database);
                        }
                        Err(e) => {
                          tracing::error!("Failed to connect to SurrealDB: {}", e);
                        }
                      }
                    });
                    
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
