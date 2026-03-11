//ENDPOINT

use clap::Parser;
use diesel::RunQueryDsl;
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
  db::dal::CltDbOperator,
};
use tokio::{
  net::TcpStream,
  sync::Mutex,
};

mod fs;

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
  id: Option<String>,
  secret: Option<String>,
  name: String,
  authenticated: bool,
  auth_type: Option<codec::IdentifyType>,
  authentication_used: Option<codec::IdentifyType>,
  jobs_last_requested: Option<std::time::Instant>,
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

      // initialize Sqlite Db
      remex_core::db::migrate(remex_core::db::ConnectionType::Sqlite)
        .await
        .unwrap();
      let mut dbconn = remex_core::db::establish_connection_sqlite();

      // spawn a new thread that will monitor the ctx variable and check for new jobs every 5
      // minutes
      let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
      let ctx_clone = ctx.clone();
      let tx_clone = tx.clone();
      tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
        loop {
          interval.tick().await;
          let mut ctx_lock = ctx_clone.lock().await;
          if ctx_lock.authenticated {
            let mut should_request = false;
            if let Some(last_requested) = ctx_lock.jobs_last_requested {
              if last_requested.elapsed().as_secs() >= 30 {
                should_request = true;
              }
            } else {
              should_request = true;
            }
            if should_request {
              ctx_lock.jobs_last_requested = Some(std::time::Instant::now());
              let _ = tx_clone.send(codec::ClientRequest::JobsRequest(codec::JobsRequest::All));
            }
          }
        }
      });

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
                    framed.send(ClientRequest::Ping).await.unwrap();
                    if !ctx_lock.authenticated {
                      tracing::info!("Attempting to authenticate");
                      let iden = ctx_lock.auth_type.clone().unwrap();
                      framed
                        .send(codec::ClientRequest::ConnectionRequest(
                          codec::ConnectionRequest::Identify(iden.clone()),
                        ))
                        .await
                        .unwrap();
                      ctx_lock.authentication_used = Some(iden);
                    }
                  }
                  (ServerResponse::ConnectionResponse(ConnectionResponse::Disconnect(reason)), _) => {
                    match reason {
                      DisconnectReason::AuthFailed => {
                        tracing::error!("Authentication failed\n Removing stored credentials and trying again in 5 seconds");
                        fs::id::remove_id().unwrap();
                        fs::secret::remove_secret().unwrap();
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        break;
                      }
                      DisconnectReason::InvalidClientId => {
                        tracing::error!("Invalid client ID\n Removing stored credentials and trying again in 5 seconds");
                        fs::id::remove_id().unwrap();
                        fs::secret::remove_secret().unwrap();
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                        break;
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
                    use remex_core::db::{schema::endpoint::jobs, model::endpoint::jobs::{JobCLT} };
                    tracing::info!("Received jobs response");
                    match j {
                      JobsResponse::ReceiveJobs(jobs) => {
                        // FIXME: Cache these for the life of the program and update them when a new
                        // one is added.
                        let dbjobs: Vec<JobCLT> = jobs::table.load(&mut dbconn).unwrap();
                        tracing::info!("Received {} jobs", jobs.len());
                        for job in jobs {
                          if !dbjobs.iter().any(|j| j.id == job.id) {
                            job.create_clt(&mut dbconn).unwrap();
                            tracing::info!("Job: {} \n Inserted into database", job.job_name);
                          } else {
                            tracing::info!("Job: {} \n Already exists in database", job.job_name);
                          }
                        }
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
                    framed
                      .send(codec::ClientRequest::JobsRequest(codec::JobsRequest::All))
                      .await
                      .unwrap();
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
          req = rx.recv() => {
            if let Some(req) = req {
              if let Err(e) = framed.send(req).await {
                tracing::error!("Failed to send scheduled request: {}", e);
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
