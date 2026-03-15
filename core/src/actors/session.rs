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
use diesel::{
  ExpressionMethods,
  JoinOnDsl,
  QueryDsl,
  RunQueryDsl,
  SelectableHelper,
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
  db::{
    self,
    dal::SrvDbOperator,
    model,
    schema,
  },
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
          let client: anyhow::Result<model::server::clients::ClientSRV> = match iden {
            IdentifyType::Secret(sec, name, hw_hash) => {
              info!("Client attempting to connect with server secret: {}", &sec);
              if sec == self.server_secret {
                info!("Secret match for client: {}, {}", &name, &hw_hash);
                let secret = utils::generate_secret(false);
                // create a new client or pull existing
                futures::executor::block_on(async {
                  let mut c = db::establish_connection_postgres();
                  use model::server::clients::{
                    ClientSRV,
                    NewClientSRV,
                  };
                  use schema::server::clients;
                  match clients::table
                    .select(ClientSRV::as_select())
                    .filter(clients::client_name.eq(&name))
                    .filter(clients::hardware_hash.eq(&hw_hash))
                    .get_result(&mut c)
                  {
                    Ok(c) => Ok(c),
                    Err(_) => {
                      info!("Existing Client not found");
                      Ok(
                        diesel::insert_into(clients::table)
                          .values(&NewClientSRV {
                            id: uuid::Uuid::new_v4().to_string(),
                            client_name: name.clone(),
                            secret,
                            hardware_hash: hw_hash,
                          })
                          .on_conflict_do_nothing()
                          .get_result(&mut c)
                          .unwrap(),
                      )
                    }
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
              // TODO: At some point it makes sense to have this be an in-memory cache of the 4 most
              // recent client queries
              // TESTING: need to decide wither or not I want to check the database for the client
              // that matches the name, hw hash, secret, and ID, or if I just want to pull the
              // match for the hw hash(or id maybe) and then compare the others after the fact.
              let c = futures::executor::block_on(async {
                let mut c = db::establish_connection_postgres();
                use model::server::clients::ClientSRV;
                use schema::server::clients;
                clients::table
                  .select(ClientSRV::as_select())
                  .filter(clients::client_name.eq(&name))
                  .filter(clients::hardware_hash.eq(&hw_hash))
                  .filter(clients::secret.eq(&sec))
                  .filter(clients::id.eq(&id))
                  .get_result(&mut c)
              });
              if let Ok(clnt) = c {
                Ok(clnt)
              } else {
                Err(anyhow::anyhow!("Client not found"))
              }
            }
          };
          match client {
            Ok(clnt) => {
              self.client_id = Some(clnt.id.clone());
              self.name = Some(clnt.client_name.clone());
              self.authenticated = true;
              tracing::info!("Client {} authenticated.", &clnt.client_name);
              self.framed.write(codec::ServerResponse::ConnectionResponse(
                codec::ConnectionResponse::Authenticated(clnt.id, clnt.secret),
              ));
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
        (ClientRequest::JobsRequest(j), true) => {
          use codec::JobsRequest;
          match j {
            JobsRequest::All => {
              tracing::info!("Received request to send along all related jobs");
              let mut conn = db::establish_connection_postgres();
              use crate::db::{
                model::server::jobs::JobSRV,
                schema::server::{
                  groups_clients,
                  jobs,
                  jobs_groups,
                },
              };
              let assigned_jobs: Vec<JobSRV> = jobs::table
                // Implicitly joins `jobs` and `jobs_groups` utilizing `diesel::joinable!`
                .inner_join(jobs_groups::table)
                // Explicitly joins `groups_clients` utilizing the shared `group_id`
                .inner_join(
                  groups_clients::table.on(jobs_groups::group_id.eq(groups_clients::group_id)),
                )
                .filter(groups_clients::client_id.eq(&self.client_id.clone().unwrap()))
                .select(JobSRV::as_select())
                .get_results(&mut conn)
                .unwrap();
              self.framed.write(codec::ServerResponse::JobsResponse(
                codec::JobsResponse::ReceiveJobs(
                  assigned_jobs
                    .iter()
                    .map(|j| crate::db::dal::jobs::Job::from(j.clone()))
                    .collect(),
                ),
              ));
            }
            JobsRequest::SendExecutions(job_id, executions, logs) => {
              tracing::info!(
                "Received executions for job: {}, executions: {:?}, logs: {:?}",
                &job_id,
                &executions,
                &logs
              )
            }
            JobsRequest::UpdateJob(job) => {
              tracing::info!("Received update for job: {}", &job.job_name,);
              job
                .upsert_srv(&mut db::establish_connection_postgres())
                .unwrap();
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
