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
use tracing::info;

use crate::actors::server::{self, Server};
use crate::codec::{ClientCodec, ClientRequest, ClientResponse};

/// Force session close
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
  pub reason: crate::codec::DisconnectReason,
}
/// Handler for Disconnect message.
impl Handler<Disconnect> for RemexSession {
  type Result = ();
  fn handle(&mut self, disc: Disconnect, _: &mut Context<Self>) -> Self::Result {
    info!("Sending disconnect to peer");
    // send message to peer
    self.framed.write(ClientResponse::Disconnect(disc.reason));
  }
}

/// Message for chat server communications
///
/// New chat session is created
#[derive(Message)]
#[rtype(result = "()")]
pub struct Identified {
  pub id: uuid::Uuid,
  pub name: String,
}
/// Handler for Identified message.
impl Handler<Identified> for RemexSession {
  type Result = ();
  fn handle(&mut self, id: Identified, _: &mut Context<Self>) -> Self::Result {
    if !self.authenticated {
      self.authenticated = true;
      self.identified = true;
    }
    info!("Sending auth to peer");
    // send message to peer
    self.framed.write(ClientResponse::Authenticated(id.id, id.name));
  }
}

/// Message for chat server communications
///
/// New chat session is created
#[derive(Message)]
#[rtype(result = "()")]
pub struct Message {
  pub msg: String,
}
/// Handler for Identified message.
impl Handler<Message> for RemexSession {
  type Result = ();
  fn handle(&mut self, msg: Message, _: &mut Context<Self>) -> Self::Result {
    info!("session message: {}", &msg.msg);
    if !self.authenticated {
      return;
    }
    // send message to peer
    self.framed.write(ClientResponse::Message(msg.msg));
  }
}

/// Message for chat server communications
///
/// New chat session is created
#[derive(Message)]
#[rtype(result = "()")]
pub struct Command {
  pub cmd: String,
}
/// Handler for Identified message.
impl Handler<Command> for RemexSession {
  type Result = ();
  fn handle(&mut self, cmd: Command, _: &mut Context<Self>) -> Self::Result {
    info!("session command: {}", &cmd.cmd);
    if !self.authenticated {
      return;
    }
    // send message to peer
    self.framed.write(ClientResponse::Command(cmd.cmd));
  }
}

#[allow(dead_code)]
/// `RemexSession` actor is responsible for tcp peer communications.
pub struct RemexSession {
  /// unique session id
  id: String,
  /// unique client id
  client_id: Option<u64>,
  /// machine name
  name: Option<String>,
  /// is client authenticated
  authenticated: bool,
  /// is client identified
  identified: bool,
  /// this is address of Remex server
  addr: Addr<Server>,
  /// this is address of Remex db
  db: Addr<crate::db::Db>,
  /// Client must send ping at least once per 10 seconds, otherwise we drop
  /// connection.
  hb: Instant,
  /// Framed wrapper
  framed: actix::io::FramedWrite<ClientResponse, WriteHalf<TcpStream>, ClientCodec>,
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
    self.addr.do_send(server::msg::Disconnect {
      id: self.id.clone(),
    });
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
      Ok(ClientRequest::Command(cmd)) => {
        // Send Command message to Remex server and wait for response
        info!("Receive message");
        self
          .addr
          .send(server::msg::Command {
            id: self.id.clone(),
            command: cmd,
          })
          .into_actor(self)
          .then(|res, act, _| {
            match res {
              Ok(res) => {
                act.framed.write(ClientResponse::Message(res));
              }
              _ => info!("Something is wrong"),
            }
            actix::fut::ready(())
          })
          .wait(ctx)
        // .wait(ctx) pauses all events in context,
        // so actor wont receive any new messages until it get list of rooms back
      }
      Ok(ClientRequest::IdentifySecret(sec, name)) => {
        // TODO: implement way to synchronize secret with a client once their secret is expired
        // by using a username and password, or something like that.
        if sec == crate::SECRET {
          info!("Correct secret. Session authenticated for {}, {}", self.id, &name);
          self.authenticated = true;
          self.identified = true;
          self.name = Some(name.clone());

          // register self in Remex server. `AsyncContext::wait` register
          // future within context, but context waits until this future resolves
          // before processing any other events.
          let addr = ctx.address();
          self
            .addr
            .send(server::msg::Connect {
              id: None,
              client_name: self.name.clone().unwrap(),
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
      Ok(ClientRequest::IdentifyId(id, name)) => {
        // TODO: implement way to synchronize secret with a client once their secret is expired
        // by using a username and password, or something like that.
        self.name = Some(name.clone());

        // register self in Remex server. `AsyncContext::wait` register
        // future within context, but context waits until this future resolves
        // before processing any other events.
        let addr = ctx.address();
        self
          .addr
          .send(server::msg::Connect {
            id: Some(id),
            client_name: self.name.clone().unwrap(),
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
      // we update heartbeat time on ping from peer
      Ok(ClientRequest::Ping) => self.hb = Instant::now(),
      _ => ctx.stop(),
    }
  }
}

/// Helper methods
impl RemexSession {
  pub fn new(
    server: Addr<crate::actors::server::Server>,
    db: Addr<crate::db::Db>,
    framed: actix::io::FramedWrite<ClientResponse, WriteHalf<TcpStream>, ClientCodec>,
  ) -> RemexSession {
    RemexSession {
      id: uuid::Uuid::new_v4().to_string(),
      client_id: None,
      name: None,
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
      if Instant::now().duration_since(act.hb) > Duration::new(10, 0) {
        // heartbeat timed out
        info!("Client heartbeat failed, disconnecting!");

        // notify Remex server
        act.addr.do_send(server::msg::Disconnect { id: act.id.clone() });

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
pub async fn tcp_server(s: &str, db: Addr<crate::db::Db>, server: Addr<Server>) {
  // Create server listener
  let addr = net::SocketAddr::from_str(s).unwrap();

  let listener = TcpListener::bind(&addr).await.unwrap();

  while let Ok((stream, _)) = listener.accept().await {
    let db = db.clone();
    let server = server.clone();
    RemexSession::create(|ctx| {
      let (r, w) = split(stream);
      RemexSession::add_stream(FramedRead::new(r, ClientCodec), ctx);
      RemexSession::new(server, db, actix::io::FramedWrite::new(w, ClientCodec, ctx))
    });
  }
}
