//ENDPOINT

use clap::Parser;
use futures_util::{SinkExt as _, StreamExt as _};
use gethostname::gethostname;
use remex_core::codec::{
  self, ClientRequest, ConnectionResponse, DisconnectReason, ServerResponse,
};
use tokio::{net::TcpStream, select, time::Instant};

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

struct Context {
  id: Option<String>,
  secret: Option<String>,
  name: String,
  authenticated: bool,
  auth_type: Option<codec::IdentifyType>,
  authentication_used: Option<codec::IdentifyType>,
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
  let mut ctx: Context = Context {
    id: None,
    secret: None,
    name: gethostname().to_string_lossy().to_string(),
    authenticated: false,
    auth_type: None,
    authentication_used: None,
  };
  ctx.auth_type = match (id_result, secret_result, args.secret.clone()) {
    // if using the server secret for auth, ensure that the ID and secret are removed first
    (_, _, Some(secret)) => {
      fs::id::remove_id().unwrap();
      fs::secret::remove_secret().unwrap();
      Some(codec::IdentifyType::Secret(secret, ctx.name.clone(), hw_hash))
    }
    (Some(id), Some(secret), _) => {
      // Both ID and secret are found, continue normally
      Some(codec::IdentifyType::ClientSecret(secret, ctx.name.clone(), id, hw_hash))
    }
    (_, _, None) => {
      panic!("Neither ID nor secret found. Please provide a secret using the --secret flag");
    }
  };

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
      remex_core::db::migrate(remex_core::db::ConnectionType::Sqlite).await.unwrap();
      let mut dbconn = remex_core::db::establish_connection_sqlite();

      /* ********** SERVER MESSAGE LOOP ********** */

      while let Some(msg) = framed.next().await {
        match msg {
          Ok(m) => {
            match (m, ctx.authenticated) {
              (ServerResponse::Ping, _) => {
                framed.send(ClientRequest::Ping).await.unwrap();
                if !ctx.authenticated {
                  tracing::info!("Attempting to authenticate");
                  let iden = ctx.auth_type.clone().unwrap();
                  framed
                    .send(codec::ClientRequest::ConnectionRequest(
                      codec::ConnectionRequest::Identify(iden),
                    ))
                    .await
                    .unwrap();
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
              (ServerResponse::ReceiveJobs(jobs), true) => {
                tracing::info!("Received {} jobs", jobs.len());
                for job in jobs {
                  tracing::info!("Job: {}", job.job_name);
                }
              }
              (
                ServerResponse::ConnectionResponse(ConnectionResponse::Authenticated(id, secret)),
                _,
              ) => {
                tracing::info!("Authenticated with id: {}, secret: {}", &id, &secret);
                ctx.id = Some(id.clone());
                ctx.secret = Some(secret.clone());
                ctx.authenticated = true;
                fs::id::save_id(id).unwrap();
                fs::secret::save_secret(secret).unwrap();
              }
              (ServerResponse::ReceiveJobs(jobs), true) => {
                tracing::info!("Received {} jobs", jobs.len());
                for job in jobs {
                  tracing::info!("Job: {}", job.job_name);
                }
              }
              s => {
                tracing::info!("Ignored server response: {:#?}", &s);
              }
            }
          }
          Err(e) => {
            tracing::info!("Client error: {}", e);
          }
        }
      }
      ctx.authenticated = false;
    }
  }
}
