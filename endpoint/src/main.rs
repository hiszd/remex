//ENDPOINT

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
  auth_type: Option<codec::IdentifyType>,
  authentication_used: Option<codec::IdentifyType>,
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
  tracing_subscriber::fmt::init();
  info!("Running client");
  let mut ctx: Context = Context {
    id: None,
    secret: None,
    name: gethostname().to_string_lossy().to_string(),
    authenticated: false,
    auth_type: None,
    authentication_used: None,
  };

  let args = Args::parse();

  // Validate secret length
  if let Some(sec) = args.secret.clone() {
    if sec.len() < 32 {
      panic!("Secret must be at least 32 characters long");
    }
  }

  // Check if both ID and secret are saved
  let id_result = fs::id::get_id()?;
  let secret_result = fs::secret::get_secret()?;

  ctx.auth_type = match (id_result, secret_result, args.secret.clone()) {
    // if using the server secret for auth, ensure that the ID and secret are removed first
    (_, _, Some(secret)) => {
      fs::id::remove_id().unwrap();
      fs::secret::remove_secret().unwrap();
      Some(codec::IdentifyType::Secret(secret, ctx.name.clone()))
    }
    (Some(id), Some(secret), _) => {
      // Both ID and secret are found, continue normally
      Some(codec::IdentifyType::ClientSecret(secret, ctx.name.clone(), id))
    }
    (_, _, None) => {
      panic!("Neither ID nor secret found. Please provide a secret using the --secret flag");
    }
  };

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

      if !ctx.authenticated {
        info!("Attempting to authenticate");
        let iden = ctx.auth_type.clone().unwrap();
        framed
          .send(codec::ClientRequest::ConnectionRequest(codec::ConnectionRequest::Identify(iden)))
          .await
          .unwrap();
      }

      // NOTE: handle server responses
      loop {
        select! {
          Some(msg) = framed.next() => {
            match msg {
              Ok(m) => {
                match m.clone() {
                  codec::ServerResponse::Ping => {}
                  _ => {
                    info!("Server response: {:#?}", &m);
                  }
                }
                if !ctx.authenticated {
                  match m.clone() {
                    codec::ServerResponse::ConnectionResponse(codec::ConnectionResponse::Authenticated(id, secret)) => {
                      info!("Correct secret, session authenticated. Id: {}, Secret: {}", &id, &secret);
                      ctx.secret = Some( secret.clone() );
                      ctx.id = Some( id.clone() );
                      ctx.authenticated = true;
                      fs::id::save_id(id.to_string()).unwrap();
                      fs::secret::save_secret(secret).unwrap();
                    }
                    codec::ServerResponse::Ping => { framed.send(codec::ClientRequest::Ping).await.unwrap(); },
                    _ => {}
                  }
                } else {
                  match m.clone() {
                    codec::ServerResponse::ConnectionResponse(r) => {
                      match r {
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
                              if let Some(iden) = ctx.authentication_used.clone() {
                                match iden {
                                  codec::IdentifyType::Secret(_, _) => {
                                    fs::id::remove_id().unwrap();
                                    fs::secret::remove_secret().unwrap();
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
                        _ => {}
                      }
                    }
                    codec::ServerResponse::ReceiveJobs(jobs) => {
                      info!("Received {} jobs", jobs.len());
                      for job in jobs {
                        info!("Job: {}", job.job_name);
                      }
                    }
                    // respond to pings with a "pong"
                    codec::ServerResponse::Ping => { framed.send(codec::ClientRequest::Ping).await.unwrap(); },
                  }
                }
              }
              Err(e) => {
                info!("Error: {}", e);
              }
            }
          }
        }
        // if !ctx.authenticated {
        //   info!("Name sent {}, and secret {}", ctx.name.clone(), args.secret.clone().unwrap());
        //   let id = fs::id::get_id();
        //   let secret = fs::secret::get_secret();
        //
        //   let iden = match (id, secret) {
        //     (Ok(Some(id_val)), Ok(Some(_))) => {
        //       // Both ID and secret files found, use ClientSecret
        //       tracing::info!("Using existing id: {} for authentication", &id_val);
        //       codec::IdentifyType::ClientSecret(
        //         args.secret.clone().unwrap(),
        //         ctx.name.clone(),
        //         id_val,
        //       )
        //     }
        //     _ => {
        //       // Either no ID or no secret (or both missing)
        //       // Use the command line secret with Secret identification
        //       tracing::info!("Using command line secret for authentication");
        //       codec::IdentifyType::Secret(args.secret.clone().unwrap(), ctx.name.clone())
        //     }
        //   };
        //
        //   ctx.authentication_used = Some(iden.clone());
        //   framed
        //     .send(codec::ClientRequest::ConnectionRequest(codec::ConnectionRequest::Identify(iden)))
        //     .await
        //     .unwrap();
        // }
        // tracing::warn!("Failed to connect to server. Trying again in 5 seconds");
        // tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
      }
    }
  }
}
