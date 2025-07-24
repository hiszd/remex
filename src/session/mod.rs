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

use crate::{core::codec::AuthRequest, messages::db, server::Server};
use crate::{
  core::codec::{ClientCodec, ClientRequest, ClientResponse},
  endpoint::Endpoint,
};
use crate::{db::Db, messages::server};

/// `RemexSession` actor is responsible for tcp peer communications.
pub struct RemexSession {
  pub identity: Option<Endpoint>,
  /// is client authenticated
  pub authenticated: bool,
  /// is client identified
  pub identified: bool,
  /// this is address of Remex server
  pub server: Addr<Server>,
  /// this is address of Remex Database
  pub db: Addr<Db>,
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

    self.framed.write(ClientResponse::Identify);
  }

  fn stopping(&mut self, _: &mut Self::Context) -> Running {
    // notify Remex server
    if self.identity.is_some() {
      self.server.do_send(server::conn::Disconnect {
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
      Ok(ClientRequest::Identify(epnt, authreq)) => {
        self.db.do_send(db::conn::ClientAuth {
          identity: epnt.clone(),
          authreq: authreq.clone(),
          session: ctx.address(),
        });
      }
      Ok(ClientRequest::Ping) => self.hb = Instant::now(),
      Ok(ClientRequest::Message(msg)) => {
        let identity = self.identity.clone().unwrap();
        info!("Client {} sent message: {}", &identity.name, &msg);
      }
      _ => ctx.stop(),
    }
  }
}

/// Helper methods
impl RemexSession {
  pub fn new(
    server: Addr<Server>,
    db: Addr<Db>,
    framed: actix::io::FramedWrite<ClientResponse, WriteHalf<TcpStream>, ClientCodec>,
  ) -> RemexSession {
    RemexSession {
      identity: None,
      server,
      db,
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
          act.server.do_send(server::conn::Disconnect {
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
pub async fn tcp_server(s: &str, server: Addr<Server>, db: Addr<Db>) {
  // Create server listener
  let addr = net::SocketAddr::from_str(s).unwrap();

  let listener = TcpListener::bind(&addr).await.unwrap();

  while let Ok((stream, _)) = listener.accept().await {
    let server = server.clone();
    let db = db.clone();
    RemexSession::create(|ctx| {
      let (r, w) = split(stream);
      RemexSession::add_stream(FramedRead::new(r, ClientCodec), ctx);
      // create a 10 digit string of random numbers
      RemexSession::new(server, db, actix::io::FramedWrite::new(w, ClientCodec, ctx))
    });
  }
}
