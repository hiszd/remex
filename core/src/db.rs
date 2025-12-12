use std::path::Path;

use actix::{Actor, AsyncContext, Context, Handler, Message};
use tracing::{error, info};

use crate::db::clients::Pools;

pub mod actions;
pub mod clients;
pub mod logs;
pub mod model;

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
          Path::new(&crate_dir).join("../migrations/postgres")
        };
        sqlx::migrate::Migrator::new(migrations).await.unwrap().run(p).await.unwrap();
      }
      Pools::Sqlite(p) => {
        let migrations = {
          let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
          Path::new(&crate_dir).join("../migrations/sqlite")
        };
        sqlx::migrate::Migrator::new(migrations).await.unwrap().run(p).await.unwrap();
      }
    }
  }
}

impl Actor for Db {
  type Context = Context<Db>;

  fn started(&mut self, ctx: &mut Context<Self>) {
    let db = self.clone();
    let futr = async move { db.migrate().await };
    let fut = actix::fut::wrap_future::<_, Self>(futr);
    ctx.spawn(fut);
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
    let futr = Box::pin(async move {
      if id1.is_none() {
        tracing::info!("Generating new id");
        let id = actions::clients::generate_id(pool.clone()).await.unwrap();
        tracing::info!("Generated id: {}", id);
        let b = actions::clients::add_client(pool, id, clientname1).await;

        match b {
          Ok(client) => {
            tracing::info!("Client added with id: {}", &client.id);
            serv.do_send(crate::actors::server::DbClientIdentified {
              id: client.id,
              client_name: client.name.clone(),
              addr,
            });
          }
          Err(e) => error!("132 - db error: {}", e),
        }
      } else {
        tracing::info!("Using existing id");
        let b = actions::clients::get_client(pool, id1.clone().unwrap()).await;
        match b {
          Ok(client) => {
            serv.do_send(crate::actors::server::DbClientIdentified {
              id: client.id,
              client_name: client.name.clone(),
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
