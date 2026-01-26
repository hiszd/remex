use std::path::Path;

use actix::{Actor, AsyncContext, Context, Handler, Message};
use rand::Rng;
use tracing::{error, info};

use crate::db::{
  self,
  containers::{groups::GroupCont, jobs::JobCont},
};

pub mod actions;
pub mod containers;
pub mod model;
pub mod util;

#[derive(Debug, Clone)]
pub enum Pools {
  Sqlite(sqlx::SqlitePool),
  Postgres(sqlx::PgPool),
}

#[derive(Clone)]
pub struct Db {
  pub pool: Pools,
  pub server: actix::Addr<crate::actors::server::Server>,
}

#[allow(async_fn_in_trait)]
pub trait DbMigrate {
  async fn migrate(&self);
}
impl DbMigrate for Db {
  async fn migrate(&self) {
    info!("migrating db");
    // Migrate the database
    match &self.pool.clone() {
      Pools::Postgres(p) => {
        let migrations = {
          let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
          Path::new(&crate_dir).join("../migrations/server")
        };
        sqlx::migrate::Migrator::new(migrations).await.unwrap().run(p).await.unwrap();
      }
      Pools::Sqlite(p) => {
        let migrations = {
          let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
          Path::new(&crate_dir).join("../migrations/endpoint")
        };
        sqlx::migrate::Migrator::new(migrations).await.unwrap().run(p).await.unwrap();
      }
    }
  }
}

impl Actor for Db {
  type Context = Context<Db>;

  fn started(&mut self, ctx: &mut Context<Self>) {
    {
      let db = self.clone();
      let futr = async move { db.migrate().await };
      let fut = actix::fut::wrap_future::<_, Self>(futr);
      ctx.wait(fut);
    }
    {
      let db2 = self.clone();
      let futr2 = async move {
        let job = actions::jobs::requests::get_job_complete(
          db2.pool.clone(),
          sqlx::types::Uuid::parse_str("03d82934-3bf2-45f1-a14c-7bace871d5a4").unwrap(),
        )
        .await
        .unwrap();
        println!("job: {:?}", job);
        tracing::info!("testing table join");
        match db2.pool.clone() {
          Pools::Postgres(pool) => {
            let mut g = GroupCont::new(&pool, "test_group".to_string()).await;
            println!("gotten group: {:?}", g);
            if g.clients.is_empty() {
              tracing::info!("group has no clients");
              let c = actions::clients::requests::get_client(
                db::Pools::Postgres(pool.clone()),
                None,
                Some("devel".to_string()),
              )
              .await;
              match c {
                Ok(c) => {
                  g.add_client(&pool, c).await;
                }
                Err(e) => match e {
                  sqlx::Error::RowNotFound => {
                    tracing::error!("Client with the name \"devel\" not found");
                  }
                  _ => {
                    tracing::error!("db error: {}", e);
                  }
                },
              }
            } else {
              for c in g.clients {
                println!("client: {}", c.client_name);
              }
            }
            let mut j = JobCont::new(
              &pool,
              "test_job".to_string(),
              "test".to_string(),
              "disabled".to_string(),
              "bash".to_string(),
            )
            .await;
          }
          _ => panic!("Client pool is not allowed for this purpose"),
        }
      };
      let fut2 = actix::fut::wrap_future::<_, Self>(futr2);
      ctx.spawn(fut2);
    }
  }

  fn stopped(&mut self, ctx: &mut Context<Self>) {
    match self.pool.clone() {
      Pools::Postgres(p) => {
        let futr = async move {
          p.close().await;
        };
        let fut = actix::fut::wrap_future::<_, Self>(futr);
        ctx.spawn(fut);
      }
      Pools::Sqlite(p) => {
        let futr = async move {
          p.close().await;
        };
        let fut = actix::fut::wrap_future::<_, Self>(futr);
        ctx.spawn(fut);
      }
    }
  }
}

#[derive(Message)]
#[rtype(result = "Vec<String>")]
pub struct GetLogs {}
impl Handler<GetLogs> for Db {
  type Result = Vec<String>;
  fn handle(&mut self, _msg: GetLogs, _ctx: &mut Context<Self>) -> Self::Result {
    match &self.pool.clone() {
      Pools::Sqlite(p) => {
        futures::executor::block_on(async {
          sqlx::query("SELECT * FROM logs").fetch_all(p).await.unwrap()
        });
      }
      Pools::Postgres(p) => {
        futures::executor::block_on(async {
          sqlx::query("SELECT * FROM logs").fetch_all(p).await.unwrap()
        });
      }
    }
    // FIXME: this should use the output of the match
    vec!["bob".to_owned()]
  }
}

#[derive(Debug)]
pub struct NewClient {
  pub id: Option<String>,
  pub client_name: String,
  pub addr: actix::Addr<crate::actors::session::RemexSession>,
}
impl Message for NewClient {
  type Result = Result<(), anyhow::Error>;
}
impl Handler<NewClient> for Db {
  type Result = Result<(), anyhow::Error>;
  fn handle(&mut self, msg: NewClient, ctx: &mut Context<Self>) -> Self::Result {
    let clientname1 = msg.client_name.clone();
    let id1 = msg.id.clone();
    let addr = msg.addr.clone();
    let pool = self.pool.clone();
    let serv = self.server.clone();
    let secret: String =
      rand::rng().sample_iter(&rand::distr::Alphanumeric).take(32).map(char::from).collect();
    let futr = Box::pin(async move {
      if id1.is_none() {
        let b = actions::clients::commands::add_client(pool, clientname1, secret).await;

        match b {
          Ok(client) => {
            tracing::info!("Client added with id: {}", &client.id);
            serv.do_send(crate::actors::server::DbClientIdentified {
              id: client.id,
              client_name: client.client_name.clone(),
              secret: client.secret.clone(),
              addr,
            });
          }
          Err(e) => error!("132 - db error: {}", e),
        }
      } else {
        tracing::info!("Using existing id");
        let b = actions::clients::requests::get_client(
          pool,
          Some(sqlx::types::Uuid::parse_str(id1.clone().unwrap().as_str()).unwrap()),
          None,
        )
        .await;
        match b {
          Ok(client) => {
            serv.do_send(crate::actors::server::DbClientIdentified {
              id: client.id,
              client_name: client.client_name.clone(),
              secret: client.secret.clone(),
              addr,
            });
          }
          Err(e) => {
            if let sqlx::Error::RowNotFound = e {
              tracing::error!("client not found");
              addr.do_send(crate::actors::session::Disconnect {
                reason: crate::codec::DisconnectReason::InvalidClientId,
              });
            }
          }
        }
      }
    });
    let fut = actix::fut::wrap_future::<_, Self>(futr);
    ctx.spawn(fut);
    Ok(())
  }
}
