//! `ClientSession` is an actor, it manages peer tcp connection and
//! proxies commands from peer to `RemexServer`.

use std::{
  io,
  time::{
    Duration,
    Instant,
  },
};

use actix::prelude::*;
use surrealdb::{
  engine::any::Any,
  Surreal,
};
use tokio::{
  io::{
    split,
    WriteHalf,
  },
  net::{
    TcpListener,
    TcpStream,
  },
};
use tokio_util::codec::FramedRead;
use tracing::info;

use crate::{
  actors::{
    self,
    server::{
      self,
      RemexServer,
    },
  },
  codec::{
    self,
    ClientRequest,
  },
};

pub mod msg;

#[allow(dead_code)]
/// `RemexSession` actor is responsible for tcp peer communications.
pub struct RemexSession {
  /// unique session id
  id: String,
  /// unique client id
  client_id: Option<surrealdb::types::RecordId>,
  /// machine name
  name: Option<String>,
  /// server secret
  server_secret: String,
  /// credential for SurrealDB auth (stored for JWT refresh)
  credential: Option<String>,
  /// current JWT ID (for refresh tracking)
  current_jwt_id: Option<String>,
  /// is client authenticated
  authenticated: bool,
  /// is client identified
  identified: bool,
  /// this is address of Remex server
  addr: Addr<RemexServer>,
  /// SurrealDB connection for auth
  db: Option<Surreal<Any>>,
  /// Client must send ping at least once per 10 seconds, otherwise we drop
  /// connection.
  hb: Instant,
  /// Framed wrapper
  framed: actix::io::FramedWrite<codec::ServerResponse, WriteHalf<TcpStream>, codec::ClientCodec>,
}

impl Actor for RemexSession {
  /// For tcp communication we are going to use `FramedContext`.
  /// It is convenient wrapper around `Framed` object from `tokio_io`
  type Context = Context<Self>;

  fn started(&mut self, ctx: &mut Self::Context) {
    // we'll start heartbeat process on session start.
    self.hb(ctx);
  }

  fn stopping(&mut self, _: &mut Self::Context) -> Running {
    // notify Remex server
    self.addr.do_send(server::msg::ClientDisconnect {
      id: self.id.clone(),
      reason: codec::DisconnectReason::Unknown("Session stopping".to_string()),
    });
    Running::Stop
  }
}

impl actix::io::WriteHandler<io::Error> for RemexSession {
}

impl StreamHandler<Result<ClientRequest, io::Error>> for RemexSession {
  fn handle(&mut self, msg: Result<ClientRequest, io::Error>, ctx: &mut Context<Self>) {
    match msg {
      Ok(ClientRequest::SigninClient(client_secret, client_name, client_id, hardware_hash)) => {
        ctx.notify(msg::SigninClient {
          iden: (client_secret, client_name, client_id, hardware_hash),
          db: self.db.clone(),
        });
      }
      Ok(ClientRequest::SignupClient(client_secret, client_name, hardware_hash)) => {
        ctx.notify(msg::SignupClient {
          iden: (client_secret, client_name, hardware_hash),
          db: self.db.clone(),
          server_secret: self.server_secret.clone(),
        });
      }
      Ok(ClientRequest::Ping) => {
        self.hb = Instant::now();
      }
      #[allow(unreachable_patterns)]
      Ok(s) => {
        tracing::info!("Ignored Client request: {:#?}", &s);
      }
      Err(e) => info!("Client error: {}", e),
    }
  }
}

impl RemexSession {
  pub fn new(
    secret: String,
    server: Addr<actors::server::RemexServer>,
    framed: actix::io::FramedWrite<codec::ServerResponse, WriteHalf<TcpStream>, codec::ClientCodec>,
    db: Option<Surreal<Any>>,
  ) -> RemexSession {
    RemexSession {
      id: uuid::Uuid::new_v4().to_string(),
      client_id: None,
      name: None,
      server_secret: secret,
      credential: None,
      current_jwt_id: None,
      addr: server,
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
      if Instant::now().duration_since(act.hb) > Duration::new(15, 0) {
        // heartbeat timed out
        info!("Client heartbeat failed, disconnecting!");

        // notify Remex server
        act.addr.do_send(server::msg::ClientDisconnect {
          id: act.id.clone(),
          reason: codec::DisconnectReason::HeartbeatFailed,
        });

        // stop actor
        ctx.stop();
      }

      // if we can not send message to sink, sink is closed (disconnected)
      act.framed.write(codec::ServerResponse::Ping);
    });
  }
}

/// Define TCP server that will accept incoming TCP connection and create
/// Client actors.
pub async fn tcp_server(
  s: &str,
  secret: &str,
  server: Addr<RemexServer>,
  db: Option<Surreal<Any>>,
) {
  let addr = s.to_string();
  let listener = TcpListener::bind(&addr).await.unwrap();

  while let Ok((stream, _)) = listener.accept().await {
    let server = server.clone();
    let db_clone = db.clone();
    RemexSession::create(|ctx| {
      let (r, w) = split(stream);
      RemexSession::add_stream(FramedRead::new(r, codec::ClientCodec), ctx);
      RemexSession::new(
        secret.into(),
        server,
        actix::io::FramedWrite::new(w, codec::ClientCodec, ctx),
        db_clone.clone(),
      )
    });
  }
}
