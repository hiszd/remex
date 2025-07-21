use common::endpoint::Endpoint;
//ENDPOINT
use futures_util::{SinkExt as _, StreamExt as _};
use gethostname::gethostname;
use tokio::{net::TcpStream, select};
use tracing::info;

extern crate common;

use common::core::codec;
use common::fs::id;

const IP: &str = "127.0.0.1";
const PORT: u16 = 4269;

const SECRET: &str = "tZs3U%hqY^o$&*y%4HcF8&RyAKevUbZnkTsrjCzPGxfare3Yn9c7shVZETfPDPUc8xR%N38a!TL%2$WbkFhZqmH#jvw&d3^mryPD8Y8TqHoJHwyKSTJeQB7vK7QkW#&B";

struct Context {
  identity: Endpoint,
  used_existing_id: bool,
  authenticated: bool,
}

#[actix_web::main]
async fn main() {
  tracing_subscriber::fmt::init();
  info!("Running client");

  let machineid = common::fs::machineid::get_machineid().unwrap();

  let addr = (IP, PORT);

  loop {
    // continually try and connect to the server every 5 seconds until we succeed
    // TODO: Maybe handle errors that aren't "Connection Refused" differently in the future
    let st = TcpStream::connect(addr).await;
    if st.is_err() {
      tracing::warn!("Failed to connect to server. Trying again in 5 seconds");
      tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    } else {
      let stream = st.unwrap();
      let mut framed = actix_codec::Framed::new(stream, codec::ServerCodec);
      let mut ctx: Context = Context {
        identity: Endpoint {
          id: None,
          name: gethostname().to_string_lossy().to_string(),
          machineid: machineid.clone(),
        },
        authenticated: false,
        used_existing_id: false,
      };

      // NOTE: handle server responses
      loop {
        select! {
          Some(msg) = framed.next() => {
            match msg {
              Ok(codec::ClientResponse::Command(ref cmd)) => {
                info!("command: {cmd}");
              }
              Ok(codec::ClientResponse::Message(ref msg)) => {
                info!("message: {msg}");
              }
              // respond to the server asking us to identify
              Ok(codec::ClientResponse::Identify) => {
                info!("Name sent {}, and secret {}", ctx.identity.name.clone(), SECRET.to_string());
                let id = id::get_id_from_file();
                match id {
                  Ok(s) => {
                    if s.is_some() {
                      let i = s.unwrap();
                      ctx.used_existing_id = true;
                      tracing::info!("Using existing id: {}", &i);
                      tracing::info!("Id {} used for authentication", &i);
                      framed.send(codec::ClientRequest::Identify(Some(i.clone()), None, ctx.identity.clone())).await.unwrap();
                    } else {
                      tracing::info!("Secret {} used for authentication", SECRET.to_string());
                      framed.send(codec::ClientRequest::Identify(None, Some(SECRET.to_string()), ctx.identity.clone())).await.unwrap();
                    }
                  },
                  Err(e) => {
                    tracing::error!("{}",e);
                  }
                }
              }
              Ok(codec::ClientResponse::Authenticated(epnt, secret)) => {
                info!("Correct secret, session authenticated. Id: {:?}, Name: {}, Secret: {}", &epnt.id, &epnt.name, &secret);
                // TODO: maybe add more verification that this is the correct endpoint, or create a
                // merge function for the Endpoint struct that merges what makes sense, but leaves
                // the stuff that shouldn't change.
                ctx.identity.merge(epnt.clone());
                ctx.authenticated = true;
                if !ctx.used_existing_id {
                  id::save_id(epnt.id.clone().unwrap()).unwrap();
                } else {
                  info!("Using existing id");
                }
                framed.send(codec::ClientRequest::Message("Successfully authenticated".to_string())).await.unwrap();
              }
              Ok(codec::ClientResponse::Disconnect(reason)) => {
                info!("Disconnected: {}", reason.to_string());
                match reason {
                  codec::DisconnectReason::InvalidClientId => {
                    // if the client id is invalid, then remove it so you can get a new
                    // one
                    id::remove_id().unwrap();
                    break;
                  }
                  _ => {
                    break;
                  }
                }
              }

              // respond to pings with a "pong"
              Ok(codec::ClientResponse::Ping) => { framed.send(codec::ClientRequest::Ping).await.unwrap(); },

              _ => { eprintln!("{msg:?}"); }
            }
          }
          // Fallback to connecting to the server until the program is terminated, or a
          // connection is made
          else => {
            break;
          }
        }
      }
      tracing::warn!("Failed to connect to server. Trying again in 5 seconds");
      tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
  }
}
