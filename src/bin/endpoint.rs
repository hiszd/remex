//ENDPOINT
use std::sync::atomic::AtomicU8;
use std::sync::Arc;

use common::endpoint::messagehandler::conn::{process_conn_message, ConnError};
use common::endpoint::messagehandler::exchange::{process_exchange_message, ExchangeError};
use common::endpoint::messagehandler::Error;
use futures_util::StreamExt as _;
use tokio::{net::TcpStream, select};

use crate::common::endpoint::Context;

extern crate common;

use common::core::codec;
use common::core::codec::s2c::S2C;
use common::fs::identity;

const IP: &str = "127.0.0.1";
const PORT: u16 = 4269;

const DB_FN: &str = "context.db";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  tracing_subscriber::fmt::init();

  tracing::info!("Running client");

  let addr = (IP, PORT);
  let attempts = Arc::new(AtomicU8::new(0));
  let attempts_clone = Arc::clone(&attempts);
  tokio::spawn(async move {
    tokio::signal::ctrl_c().await.unwrap();
    let att = attempts_clone.load(std::sync::atomic::Ordering::SeqCst);
    tracing::info!("Ctrl-C received, shutting down");
    tracing::info!("Attempts: {}", &att);
    std::process::exit(0);
  });

  loop {
    // continually try and connect to the server every 5 seconds until we succeed
    // TODO: Maybe handle errors that aren't "Connection Refused" differently in the future
    let st = TcpStream::connect(addr).await;
    if st.is_err() {
      let mut att = attempts.load(std::sync::atomic::Ordering::SeqCst);
      if att == 0 {
        tracing::warn!("Failed to connect to server. Trying again until connected");
      }
      att += 1;
      attempts.store(att, std::sync::atomic::Ordering::SeqCst);
      tracing::info!("Attempts: {}", &att);
      tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    } else {
      let stream = st.unwrap();
      // let mut sigint = tokio::signal::unix::signal(SignalKind::interrupt())?;
      let mut framed = actix_codec::Framed::new(stream, codec::ServerCodec);
      let mut ctx: Context = Context {
        identity: identity::get_identity(),
        connected: false,
        executors: Vec::new(),
      };

      // NOTE: handle server responses
      loop {
        select! {
          Some(msg) = framed.next() => {
            match msg {
              Ok(m) => {
                match m {
                  S2C::Conn(mm) => {
                    match process_conn_message(mm, &mut framed, ctx.clone()).await {
                      Err(e) => {
                        match e {
                          Error::ConnError(ref e) => {
                            match e {
                                ConnError::InvalidClientId => {
                                    tracing::warn!("Invalid client id");
                                    ctx.identity = identity::get_identity();
                                    break;
                                }
                                _ => {}
                            }
                          }
                          _ => {}
                        }
                        tracing::error!("Error processing client message: {}", e);
                        break;
                      }
                      Ok(c) => {
                        attempts.store(0, std::sync::atomic::Ordering::SeqCst); // reset attempts
                        ctx = c;
                      }
                    }
                  }
                  S2C::Exchange(e) => {
                      match process_exchange_message(e, &mut framed, ctx.clone()).await {
                          Err(e) => {
                              match e {
                                  Error::ExchangeError(ref e) => {
                                      match e {
                                          ExchangeError::AuthFailed => {
                                              tracing::error!("Authentication failed. Invalid secret key");
                                              break;
                                          }
                                      }
                                  }
                                  _ => {}
                              }
                              tracing::error!("Error processing exchange message: {}", e);
                              break;
                          }
                          Ok(c) => {
                              attempts.store(0, std::sync::atomic::Ordering::SeqCst); // reset attempts
                              ctx = c;
                          }
                      }
                  }
                }
              }
              _ => {
                tracing::error!("Unknown message from server");
              }
            }
          },
          // Fallback to connecting to the server until the program is terminated, or a
          // connection is made
          else => {
            break;
          }
        }
      }
      let mut att = attempts.load(std::sync::atomic::Ordering::SeqCst);
      if att == 0 {
        tracing::warn!("Failed to connect to server. Trying again until connected");
      }
      att += 1;
      attempts.store(att, std::sync::atomic::Ordering::SeqCst);
      tracing::info!("Attempts: {}", &att);
      tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
  }
}
