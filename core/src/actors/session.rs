//! `ClientSession` is an actor, it manages peer tcp connection and
//! proxies commands from peer to `RemexServer`.

use std::{
  io, net,
  str::FromStr,
  time::{Duration, Instant},
};

use actix::prelude::*;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper};
use tokio::{
  io::{split, WriteHalf},
  net::{TcpListener, TcpStream},
};
use tokio_util::codec::FramedRead;
use tracing::info;

use crate::codec::{
  ClientCodec, ClientRequest, ConnectionRequest, ConnectionResponse, ServerResponse,
};
use crate::{
  actors::server::{self, RemexServer},
  codec::IdentifyType,
};

pub mod msg;

#[allow(dead_code)]
/// `RemexSession` actor is responsible for tcp peer communications.
pub struct RemexSession {
  /// unique session id
  id: String,
  /// unique client id
  client_id: Option<u64>,
  /// machine name
  name: Option<String>,
  /// server secret
  server_secret: String,
  /// is client authenticated
  authenticated: bool,
  /// is client identified
  identified: bool,
  /// this is address of Remex server
  addr: Addr<RemexServer>,
  /// Client must send ping at least once per 10 seconds, otherwise we drop
  /// connection.
  hb: Instant,
  /// Framed wrapper
  framed: actix::io::FramedWrite<ServerResponse, WriteHalf<TcpStream>, ClientCodec>,
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
      reason: crate::codec::DisconnectReason::Unknown("Session stopping".to_string()),
    });
    Running::Stop
  }
}

impl actix::io::WriteHandler<io::Error> for RemexSession {
}

// NOTE: handle client requests
// To use `Framed` we have to define Io type and Codec
impl StreamHandler<Result<ClientRequest, io::Error>> for RemexSession {
  /// This is main event loop for client requests
  fn handle(&mut self, msg: Result<ClientRequest, io::Error>, ctx: &mut Context<Self>) {
    match msg {
      Ok(m) => {
        match m.clone() {
          ClientRequest::ConnectionRequest(c) => {
            tracing::info!("Client connection request: {:#?}", &c);
            match c {
              ConnectionRequest::Identify(i) => {
                let (name, id, secret): (String, String, String) = match i {
                  // Server secret is used to identify client
                  IdentifyType::Secret(sec, name) => {
                    info!("Client attempting to connect with server secret: {}", &sec);
                    if sec == self.server_secret {
                      info!("Secret match");
                      let secret = crate::utils::generate_secret(false);
                      // create a new client or pull existing
                      let c: crate::db::model::clients::Client =
                        futures::executor::block_on(async {
                          let mut c = crate::db::establish_connection_postgres();
                          use crate::db::model::clients::NewClient;
                          use crate::db::schema::clients;
                          diesel::insert_into(clients::table)
                            .values(&NewClient {
                              id: uuid::Uuid::new_v4().to_string(),
                              client_name: name.clone(),
                              secret,
                            })
                            .on_conflict_do_nothing()
                            .get_result(&mut c)
                            .unwrap()
                        });
                      info!("Client: {}, ID: {}, Secret: {}", &c.client_name, &c.id, &c.secret);
                      (c.client_name, c.id, c.secret)
                    } else {
                      info!("Secret mismatch");
                      // disconnect from server and close the actor
                      self.framed.write(ServerResponse::ConnectionResponse(
                        ConnectionResponse::Disconnect(crate::codec::DisconnectReason::AuthFailed),
                      ));
                      ctx.stop();
                      return;
                    }
                  }
                  // Client secret that was assigned to the client by the server is used to authenticate
                  IdentifyType::ClientSecret(sec, name, id) => {
                    info!(
                      "Client attempting to connect with client secret: {}, name: {}, id: {}",
                      &sec, &name, &id
                    );
                    let c = futures::executor::block_on(async {
                      let mut c = crate::db::establish_connection_postgres();
                      use crate::db::model::clients::Client;
                      use crate::db::schema::clients;
                      clients::table
                        .select(Client::as_select())
                        .filter(clients::client_name.eq(&name))
                        .filter(clients::id.eq(&id))
                        .get_result(&mut c)
                    });
                    if let Ok(clnt) = c {
                      if clnt.secret == sec {
                        (clnt.client_name, clnt.id, clnt.secret)
                      } else {
                        self.framed.write(ServerResponse::ConnectionResponse(
                          ConnectionResponse::Disconnect(
                            crate::codec::DisconnectReason::AuthFailed,
                          ),
                        ));
                        ctx.stop();
                        return;
                      }
                    } else {
                      self.framed.write(ServerResponse::ConnectionResponse(
                        ConnectionResponse::Disconnect(crate::codec::DisconnectReason::AuthFailed),
                      ));
                      ctx.stop();
                      return;
                    }
                  }
                };
                self.name = Some(name.clone());
                self.authenticated = true;
                self.id = id.clone();
                tracing::info!("Sending auth to client: {}", &name);
                self.framed.write(ServerResponse::ConnectionResponse(
                  ConnectionResponse::Authenticated(id, secret.clone()),
                ));
              }
            }
          }
          // Ping from client
          ClientRequest::Ping => self.hb = Instant::now(),
          s => info!("Client request: {:?}", s),
        }
      }
      Err(e) => info!("Client error: {}", e),
    }
  }
}

/// Helper methods
impl RemexSession {
  pub fn new(
    secret: String,
    server: Addr<crate::actors::server::RemexServer>,
    framed: actix::io::FramedWrite<ServerResponse, WriteHalf<TcpStream>, ClientCodec>,
  ) -> RemexSession {
    RemexSession {
      id: uuid::Uuid::new_v4().to_string(),
      client_id: None,
      name: None,
      server_secret: secret,
      addr: server,
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
          reason: crate::codec::DisconnectReason::HeartbeatFailed,
        });

        // stop actor
        ctx.stop();
      }

      // if we can not send message to sink, sink is closed (disconnected)
      act.framed.write(ServerResponse::Ping);
    });
  }
}

/// Define TCP server that will accept incoming TCP connection and create
/// Client actors.
pub async fn tcp_server(s: &str, secret: &str, server: Addr<RemexServer>) {
  // Create server listener
  let addr = net::SocketAddr::from_str(s).unwrap();

  let listener = TcpListener::bind(&addr).await.unwrap();

  while let Ok((stream, _)) = listener.accept().await {
    let server = server.clone();
    RemexSession::create(|ctx| {
      let (r, w) = split(stream);
      RemexSession::add_stream(FramedRead::new(r, ClientCodec), ctx);
      RemexSession::new(secret.into(), server, actix::io::FramedWrite::new(w, ClientCodec, ctx))
    });
  }
}
