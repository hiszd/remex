//ENDPOINT
use std::io::{self, Write};

use clap::Parser;
use futures_util::{SinkExt as _, StreamExt as _};
use gethostname::gethostname;
use remex_core::codec;
use tokio::{net::TcpStream, select};
use tracing::info;

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
  authentication_used: Option<codec::IdentifyType>,
}

#[actix_web::main]
async fn main() {
  tracing_subscriber::fmt::init();
  info!("Running client");

  let args = Args::parse();

  // Validate secret length
  if args.secret.clone().unwrap_or("".to_string()).len() < 32 {
    panic!("Secret must be at least 32 characters long");
  }

  // Check if both ID and secret are saved
  let id_result = fs::id::get_id();
  let secret_result = fs::secret::get_secret();

  match (id_result, secret_result) {
    (Ok(Some(_)), Ok(Some(_))) => {
      // Both ID and secret are found, continue normally
    }
    _ => {
      // Either ID or secret (or both) are missing
      if args.secret.is_none() {
        panic!("Neither ID nor secret found. Please provide a secret using the --secret flag");
      }
    }
  }

  loop {
    // continually try and connect to the server every 5 seconds until we succeed
    // TODO: Maybe handle errors that aren't "Connection Refused" differently in the future
    let st = TcpStream::connect(format!("{}:{}", args.server, args.port)).await;
    if st.is_err() {
      tracing::warn!("Failed to connect to server. Trying again in 5 seconds");
      tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    } else {
      let stream = st.unwrap();
      let mut framed = actix_codec::Framed::new(stream, codec::ServerCodec);
      let mut ctx: Context = Context {
        id: None,
        secret: None,
        name: gethostname().to_string_lossy().to_string(),
        authenticated: false,
        authentication_used: None,
      };

      // NOTE: handle server responses
      loop {
        select! {
              Some(msg) = framed.next() => {
                match msg {
                  Ok(codec::ServerResponse::ConnectionResponse(r)) => {
                    match r {
                      codec::ConnectionResponse::Authenticated(id, secret) => {
                        info!("Correct secret, session authenticated. Id: {}, Secret: {}", &id, &secret);
                        ctx.secret = Some( secret.clone() );
                        ctx.id = Some( id.clone() );
                        ctx.authenticated = true;
                        fs::id::save_id(id.to_string()).unwrap();
                      }
                      codec::ConnectionResponse::Disconnect(reason) => {
                        info!("Disconnected: {}", reason.to_string());
                        match reason {
                          codec::DisconnectReason::InvalidClientId => {
                            // if the client id is invalid, then remove it so you can get a new
                            // one
                            fs::id::remove_id().unwrap();
                            break;
                          }
                          codec::DisconnectReason::AuthFailed => {
                          if let Some(iden) = ctx.authentication_used {
                            match iden {
                              codec::IdentifyType::Secret(_, _) => {
                                // if the secret is invalid, then remove it so you can get a new
                                // one
                                fs::id::remove_id().unwrap();
                              ctx.authenticated = false;
                                break;
                              }
                              codec::IdentifyType::ClientSecret(_, _, _) => {
                                // if the client id is invalid, then remove it so you can get a new
                                // one
                                fs::id::remove_id().unwrap();
                              ctx.authenticated = false;
                                break;
                              }
                            }
                          }
                        }
                        _ => {
                          break;
                        }
                      }
                    }
                      // respond to pings with a "pong"
                      codec::ConnectionResponse::Ping => { framed.send(codec::ClientRequest::ConnectionRequest(codec::ConnectionRequest::Ping)).await.unwrap(); },
                      _ => {}
                    }
                  }
                  Ok(codec::ServerResponse::ReceiveJobs(jobs)) => {
                    info!("Received {} jobs", jobs.len());
                    for job in jobs {
                      info!("Job: {}", job.job_name);
                    }
                  }
                  Err(e) => {
                    info!("Error: {}", e);
                }
              }
          }
        }
        if !ctx.authenticated {
          info!("Name sent {}, and secret {}", ctx.name.clone(), args.secret.clone().unwrap());
          let id = fs::id::get_id();
          let secret = fs::secret::get_secret();

          let iden = match (id, secret) {
            (Ok(Some(id_val)), Ok(Some(_))) => {
              // Both ID and secret files found, use ClientSecret
              tracing::info!("Using existing id: {} for authentication", &id_val);
              codec::IdentifyType::ClientSecret(
                args.secret.clone().unwrap(),
                ctx.name.clone(),
                id_val,
              )
            }
            _ => {
              // Either no ID or no secret (or both missing)
              // Use the command line secret with Secret identification
              tracing::info!("Using command line secret for authentication");
              codec::IdentifyType::Secret(args.secret.clone().unwrap(), ctx.name.clone())
            }
          };

          ctx.authentication_used = Some(iden.clone());
          framed
            .send(codec::ClientRequest::ConnectionRequest(codec::ConnectionRequest::Identify(iden)))
            .await
            .unwrap();
        }
        tracing::warn!("Failed to connect to server. Trying again in 5 seconds");
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
      }
    }
  }
}
