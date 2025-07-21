//! `ClientSession` is an actor, it manages peer tcp connection and
//! proxies commands from peer to `RemexServer`.

use std::{
  io, net,
  str::FromStr,
  time::{Duration, Instant},
};

use actix::prelude::*;
use tokio::{
  io::{split, WriteHalf},
  net::{TcpListener, TcpStream},
};
use tokio_util::codec::FramedRead;
use tracing::{error, info};

use crate::messages::server;
use crate::server::Server;
use crate::{
  core::codec::{ClientCodec, ClientRequest, ClientResponse},
  endpoint::Endpoint,
};

/// `RemexSession` actor is responsible for tcp peer communications.
pub struct RemexSession {
  pub identity: Option<Endpoint>,
  /// is client authenticated
  pub authenticated: bool,
  /// is client identified
  pub identified: bool,
  /// this is address of Remex server
  pub addr: Addr<Server>,
  /// Client must send ping at least once per 10 seconds, otherwise we drop
  /// connection.
  pub hb: Instant,
  /// Framed wrapper
  pub framed: actix::io::FramedWrite<ClientResponse, WriteHalf<TcpStream>, ClientCodec>,
}

impl Actor for RemexSession {
  /// For tcp communication we are going to use `FramedContext`.
  /// It is convenient wrapper around `Framed` object from `tokio_io`
  type Context = Context<Self>;

  fn started(&mut self, ctx: &mut Self::Context) {
    // we'll start heartbeat process on session start.
    self.hb(ctx);

    info!("Session started");
    self.framed.write(ClientResponse::Identify);
  }

  fn stopping(&mut self, _: &mut Self::Context) -> Running {
    // notify Remex server
    if self.identity.is_some() {
      self.addr.do_send(server::conn::Disconnect {
        identity: self.identity.clone().unwrap(),
      });
    }
    Running::Stop
  }
}

impl actix::io::WriteHandler<io::Error> for RemexSession {
}

// NOTE: handle client requests
/// To use `Framed` we have to define Io type and Codec
impl StreamHandler<Result<ClientRequest, io::Error>> for RemexSession {
  /// This is main event loop for client requests
  fn handle(&mut self, msg: Result<ClientRequest, io::Error>, ctx: &mut Context<Self>) {
    match msg {
      Ok(ClientRequest::Message(msg)) => {
        if (!self.authenticated) || (!self.identified) {
          return;
        }
        info!("Client {:?} sent message: {}", &self.identity, &msg);
      }
      // Ok(ClientRequest::Command(cmd)) => {
      //   if (!self.authenticated) || (self.identity.is_none()) {
      //     return;
      //   }
      //   // Send Command message to Remex server and wait for response
      //   info!("Receive message");
      //   self
      //     .addr
      //     .send(super::server::messages::Command {
      //       identity: self.identity.clone().unwrap(),
      //       command: cmd,
      //     })
      //     .into_actor(self)
      //     .then(|res, act, _| {
      //       match res {
      //         Ok(res) => {
      //           act.framed.write(ClientResponse::Message(res));
      //         }
      //         _ => info!("Something is wrong"),
      //       }
      //       actix::fut::ready(())
      //     })
      //     .wait(ctx)
      //   // .wait(ctx) pauses all events in context,
      //   // so actor wont receive any new messages until it get list of rooms back
      // }
      Ok(ClientRequest::Identify(id, secret, epnt)) => {
        match (id.is_some(), secret.is_some()) {
          // Secret sent
          (false, true) => {
            // TODO: implement way to synchronize secret with a client once their secret is expired
            // by using a username and password, or something like that.
            if secret.unwrap() == crate::SECRET {
              info!("Correct secret. Session authenticated for {:?}, {:?}", &self.identity, &epnt);
              self.authenticated = true;
              self.identified = false;
              self.identity = Some(epnt.clone());

              // register self in Remex server. `AsyncContext::wait` register
              // future within context, but context waits until this future resolves
              // before processing any other events.
              let addr = ctx.address();
              self
                .addr
                .send(server::conn::Connect {
                  identity: epnt.clone(),
                  addr: addr.clone(),
                })
                .into_actor(self)
                .then(|res, _, ctx| {
                  match res {
                    Ok(_) => {}
                    // something is wrong with chat server
                    _ => ctx.stop(),
                  }
                  actix::fut::ready(())
                })
                .wait(ctx);
            } else {
              info!("Invalid secret. Stopping session");
              ctx.stop();
            }
          }
          // Id sent
          (true, false) => {
            // TODO: implement way to synchronize secret with a client once their secret is expired
            // by using a username and password, or something like that.
            self.identity = Some(epnt.clone());
            self.identified = true;
            self.authenticated = false;

            // register self in Remex server. `AsyncContext::wait` register
            // future within context, but context waits until this future resolves
            // before processing any other events.
            let addr = ctx.address();
            self
              .addr
              .send(server::conn::Connect {
                identity: epnt.clone(),
                addr: addr.clone(),
              })
              .into_actor(self)
              .then(|res, _, ctx| {
                match res {
                  Ok(_) => {}
                  // something is wrong with chat server
                  _ => ctx.stop(),
                }
                actix::fut::ready(())
              })
              .wait(ctx);
          }
          (true, true) => error!("Id and secret sent"),
          (false, false) => error!("No id or secret sent"),
        }
      }
      // we update heartbeat time on ping from peer
      Ok(ClientRequest::Ping) => self.hb = Instant::now(),
      _ => ctx.stop(),
    }
  }
}

/// Helper methods
impl RemexSession {
  pub fn new(
    addr: Addr<Server>,
    framed: actix::io::FramedWrite<ClientResponse, WriteHalf<TcpStream>, ClientCodec>,
  ) -> RemexSession {
    RemexSession {
      identity: None,
      addr,
      hb: Instant::now(),
      authenticated: false,
      identified: false,
      framed,
    }
  }

  /// helper method that sends ping to client every second.
  ///
  /// also this method check heartbeats from client
  fn hb(&self, ctx: &mut Context<Self>) {
    ctx.run_interval(Duration::new(1, 0), |act, ctx| {
      // check client heartbeats
      if Instant::now().duration_since(act.hb) > Duration::new(10, 0) {
        // heartbeat timed out
        info!("Client heartbeat failed, disconnecting!");

        // notify Remex server
        if act.identity.is_some() {
          act.addr.do_send(server::conn::Disconnect {
            identity: act.identity.clone().unwrap(),
          });
        }

        // stop actor
        ctx.stop();
      }

      act.framed.write(ClientResponse::Ping);
      // if we can not send message to sink, sink is closed (disconnected)
    });
  }
}

/// Define TCP server that will accept incoming TCP connection and create
/// Client actors.
pub async fn tcp_server(s: &str, server: Addr<Server>) {
  // Create server listener
  let addr = net::SocketAddr::from_str(s).unwrap();

  let listener = TcpListener::bind(&addr).await.unwrap();

  while let Ok((stream, _)) = listener.accept().await {
    let server = server.clone();
    RemexSession::create(|ctx| {
      let (r, w) = split(stream);
      RemexSession::add_stream(FramedRead::new(r, ClientCodec), ctx);
      // create a 10 digit string of random numbers
      RemexSession::new(server, actix::io::FramedWrite::new(w, ClientCodec, ctx))
    });
  }
}
