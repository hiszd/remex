use common::fs::identity::StoredEndpoint;
//ENDPOINT
use futures_util::{SinkExt as _, StreamExt as _};
use tokio::{net::TcpStream, select};
use tracing::info;

extern crate common;

use common::core::codec::{self, AuthRequest};
use common::fs::{id, identity};

const IP: &str = "127.0.0.1";
const PORT: u16 = 4269;

const SECRET: &str = "tZs3U%hqY^o$&*y%4HcF8&RyAKevUbZnkTsrjCzPGxfare3Yn9c7shVZETfPDPUc8xR%N38a!TL%2$WbkFhZqmH#jvw&d3^mryPD8Y8TqHoJHwyKSTJeQB7vK7QkW#&B";

struct Context {
  identity: StoredEndpoint,
  authenticated: bool,
}

#[actix_web::main]
async fn main() {
  tracing_subscriber::fmt::init();
  info!("Running client");

  let identity = identity::get_identity();

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
        identity: identity.clone(),
        authenticated: false,
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
                let authreq: AuthRequest =
                if ctx.identity.secret.is_some() {
                  let identity = ctx.identity.clone();
                  info!("Using Id and Secret: {}, {}", ctx.identity.machineid.clone(), ctx.identity.secret.clone().unwrap());
                  AuthRequest::IdSecret(identity.machineid, identity.secret.unwrap())
                } else {
                  info!("Using just Secret");
                  AuthRequest::Secret(SECRET.to_string())
                };
                framed.send(codec::ClientRequest::Identify(ctx.identity.clone().into(), authreq)).await.unwrap()
              }
              Ok(codec::ClientResponse::Authenticated(epnt, secret)) => {
                info!("Session authenticated. Id: {:?}, Name: {}, Secret: {}", &epnt.id, &epnt.name, &secret);
                ctx.identity = StoredEndpoint::from(epnt.clone());
                ctx.identity.secret = Some(secret);
                ctx.authenticated = true;
                identity::save_identity(ctx.identity.clone()).unwrap();
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
