//! `ClientSession` is an actor, it manages peer tcp connection and
//! proxies commands from peer to `RemexServer`.

use std::{
  io,
  net,
  str::FromStr,
  time::{
    Duration,
    Instant,
  },
};

use actix::prelude::*;
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
  db::{
    self,
    surreal::{
      dal::{
        ClientDal,
        ExecutionDal,
        JobDal,
        LogDal,
      },
      models::{
        Client,
        Execution,
        Job,
        Log,
      },
    },
  },
  jwt,
  utils,
};

pub mod msg;

#[allow(dead_code)]
/// `RemexSession` actor is responsible for tcp peer communications.
pub struct RemexSession {
  /// unique session id
  id: String,
  /// unique client id
  client_id: Option<String>,
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
  fn handle(&mut self, msg: Result<ClientRequest, io::Error>, _ctx: &mut Context<Self>) {
    match msg {
      Ok(m) => match (m, self.authenticated) {
        (ClientRequest::ConnectionRequest(codec::ConnectionRequest::Identify(iden)), _) => {
          use codec::IdentifyType;
          let client: anyhow::Result<Client> = match iden {
            IdentifyType::Secret(sec, name, hw_hash) => {
              info!("Client attempting to connect with server secret: {}", &sec);
              if sec == self.server_secret {
                info!("Secret match for client: {}, {}", &name, &hw_hash);
                let secret = utils::generate_secret(false);
                let dal = ClientDal::new();
                let db = db::get_db();
                futures::executor::block_on(async {
                  let db = db.read().await;
                  match dal.find_by_hardware_hash(&db, &hw_hash).await {
                    Ok(Some(c)) => Ok(c),
                    Ok(None) => {
                      info!("Existing Client not found, creating new one");
                      let new_client = Client::new(
                        secret,
                        name.clone(),
                        hw_hash,
                      );
                      dal.create(&db, &new_client).await.map_err(|e| anyhow::anyhow!("{}", e))
                    }
                    Err(e) => Err(anyhow::anyhow!("Database error: {}", e)),
                  }
                })
              } else {
                info!("Secret mismatch");
                Err(anyhow::anyhow!("Secret mismatch"))
              }
            }
            IdentifyType::ClientSecret(sec, name, id, hw_hash) => {
              info!(
                "Client attempting to connect with client secret: {}, name: {}, id: {}",
                &sec, &name, &id
              );
              let dal = ClientDal::new();
              let db = db::get_db();
              let c = futures::executor::block_on(async {
                let db = db.read().await;
                dal.read(&db, &id).await
              });
              if let Ok(clnt) = c {
                if clnt.client_name == name && clnt.hardware_hash == hw_hash && clnt.secret == sec {
                  Ok(clnt)
                } else {
                  Err(anyhow::anyhow!("Client data mismatch"))
                }
              } else {
                Err(anyhow::anyhow!("Client not found"))
              }
            }
          };
          match client {
            Ok(clnt) => {
              if let Some(client_id_str) = &clnt.id {
                self.client_id = Some(client_id_str.clone());
                self.name = Some(clnt.client_name.clone());
                self.authenticated = true;
                tracing::info!("Client {} authenticated.", &clnt.client_name);
                
                let jwt_claims = jwt::EndpointClaims::new(
                  client_id_str.clone(),
                  clnt.client_name.clone(),
                  clnt.hardware_hash.clone(),
                );
                let jwt_token = jwt::generate_token(&jwt_claims).unwrap_or_default();
                
                self.framed.write(codec::ServerResponse::ConnectionResponse(
                  codec::ConnectionResponse::Authenticated(client_id_str.clone(), clnt.secret, jwt_token),
                ));
              } else {
                tracing::error!("Client ID not found");
                self.framed.write(codec::ServerResponse::ConnectionResponse(
                  codec::ConnectionResponse::Disconnect(codec::DisconnectReason::AuthFailed),
                ));
                self.framed.close();
              }
            }
            Err(e) => {
              tracing::error!("Client creation error: {}", e);
              self.framed.write(codec::ServerResponse::ConnectionResponse(
                codec::ConnectionResponse::Disconnect(codec::DisconnectReason::AuthFailed),
              ));
              self.framed.close();
            }
          }
        }
        (ClientRequest::Ping, _) => {
          self.hb = Instant::now();
        }
        (ClientRequest::RefreshJwt, true) => {
          tracing::info!("Received JWT refresh request");
          if let Some(client_id) = &self.client_id {
            if let Some(name) = &self.name {
              let hw_hash = "refresh".to_string();
              let jwt_claims = jwt::EndpointClaims::new(
                client_id.clone(),
                name.clone(),
                hw_hash,
              );
              if let Ok(jwt_token) = jwt::generate_token(&jwt_claims) {
                self.framed.write(codec::ServerResponse::JwtRefreshed(jwt_token));
                tracing::info!("JWT refreshed for client: {}", client_id);
              } else {
                tracing::error!("Failed to generate JWT token for refresh");
              }
            }
          }
        }
        (ClientRequest::JobsRequest(j), true) => {
          use codec::JobsRequest;
          match j {
            JobsRequest::All => {
              tracing::info!("Received request to send along all related jobs");
              let job_dal = JobDal::new();
              let db = db::get_db();
              let jobs: Vec<Job> = futures::executor::block_on(async {
                let db = db.read().await;
                job_dal.list(&db).await.unwrap_or_default()
              });
              self.framed.write(codec::ServerResponse::JobsResponse(
                codec::JobsResponse::ReceiveJobs(jobs),
              ));
            }
            JobsRequest::SendExecutions(job_id, executions, logs) => {
              tracing::info!(
                "Received executions for job: {}, executions: {:?}, logs: {:?}",
                &job_id,
                &executions,
                &logs
              );

              let db = db::get_db();
              let exec_dal = ExecutionDal::new();
              let log_dal = LogDal::new();

              futures::executor::block_on(async {
                let db = db.read().await;
                for execution in &executions {
                  let mut exec = execution.clone();
                  exec.job_id = Some(job_id.clone());
                  if let Err(e) = exec_dal.upsert(&db, &exec).await {
                    tracing::error!(
                      "Failed to upsert execution: {}",
                      e
                    );
                  }
                }

                for log in &logs {
                  if let Err(e) = log_dal.upsert(&db, log).await {
                    tracing::error!("Failed to upsert log: {}", e);
                  }
                }
              });
            }
            JobsRequest::UpdateJob(job) => {
              tracing::info!("Received update for job: {}", &job.job_name,);
              let job_dal = JobDal::new();
              let db = db::get_db();
              futures::executor::block_on(async {
                let db = db.read().await;
                let _ = job_dal.upsert(&db, &job).await;
              });
            }
          }
        }
        s => {
          tracing::info!("Ignored Client request: {:#?}", &s);
        }
      },
      Err(e) => info!("Client error: {}", e),
    }
  }
}
/// Helper methods
impl RemexSession {
  pub fn new(
    secret: String,
    server: Addr<actors::server::RemexServer>,
    framed: actix::io::FramedWrite<codec::ServerResponse, WriteHalf<TcpStream>, codec::ClientCodec>,
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
pub async fn tcp_server(s: &str, secret: &str, server: Addr<RemexServer>) {
  // Create server listener
  let addr = net::SocketAddr::from_str(s).unwrap();

  let listener = TcpListener::bind(&addr).await.unwrap();

  while let Ok((stream, _)) = listener.accept().await {
    let server = server.clone();
    RemexSession::create(|ctx| {
      let (r, w) = split(stream);
      RemexSession::add_stream(FramedRead::new(r, codec::ClientCodec), ctx);
      RemexSession::new(
        secret.into(),
        server,
        actix::io::FramedWrite::new(w, codec::ClientCodec, ctx),
      )
    });
  }
}
