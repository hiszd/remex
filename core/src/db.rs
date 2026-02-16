use actix::{Actor, AsyncContext, Context, Handler, Message};
use diesel::{QueryDsl, RunQueryDsl, SelectableHelper};
use diesel_migrations::MigrationHarness;
use rand::Rng;
use tracing::{error, info};

pub const SERVER_MIGRATIONS: diesel_migrations::EmbeddedMigrations =
  diesel_migrations::embed_migrations!("../migrations/server");
pub const SHARED_MIGRATIONS: diesel_migrations::EmbeddedMigrations =
  diesel_migrations::embed_migrations!("../migrations/shared");

pub mod actions;
pub mod model;
pub mod schema;

#[derive(Debug, Clone, Copy)]
pub enum ConnectionType {
  Sqlite,
  Postgres,
}

impl Into<ConnectionType> for &str {
  fn into(self) -> ConnectionType {
    match self {
      "sqlite" => ConnectionType::Sqlite,
      "postgres" => ConnectionType::Postgres,
      _ => panic!("Unknown connection type"),
    }
  }
}

pub fn establish_connection_postgres() -> diesel::PgConnection {
  dotenvy::dotenv().ok();

  let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
  <diesel::PgConnection as diesel::Connection>::establish(&database_url)
    .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}
pub fn establish_connection_sqlite() -> diesel::SqliteConnection {
  dotenvy::dotenv().ok();

  let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
  <diesel::SqliteConnection as diesel::Connection>::establish(&database_url)
    .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

#[derive(Clone)]
pub struct Db {
  pub dburl: &'static str,
  pub dbtype: ConnectionType,
  pub server: actix::Addr<crate::actors::server::Server>,
}

#[allow(async_fn_in_trait)]
pub trait DbMigrate {
  async fn migrate(&mut self);
}
impl DbMigrate for Db {
  async fn migrate(&mut self) {
    info!("migrating db");
    // Migrate the database
    match self.dbtype {
      ConnectionType::Postgres => {
        let mut c = establish_connection_postgres();
        c.run_pending_migrations(SHARED_MIGRATIONS).unwrap();
        c.run_pending_migrations(SERVER_MIGRATIONS).unwrap();
      }
      ConnectionType::Sqlite => {
        let mut c = establish_connection_sqlite();
        c.run_pending_migrations(SHARED_MIGRATIONS).unwrap();
      }
    }
  }
}

impl Actor for Db {
  type Context = Context<Db>;

  fn started(&mut self, ctx: &mut Context<Self>) {
    let mut db = self.clone();
    let futr = async move { db.migrate().await };
    let fut = actix::fut::wrap_future::<_, Self>(futr);
    ctx.spawn(fut);
  }
}

#[derive(Debug, Clone)]
pub enum ActorAddr {
  Server(actix::Addr<crate::actors::server::Server>),
  Session(actix::Addr<crate::actors::session::RemexSession>),
}

#[derive(Message)]
#[rtype(result = "Vec<String>")]
pub struct RequestLogs {
  pub addr: ActorAddr,
}
impl Handler<RequestLogs> for Db {
  type Result = Vec<String>;
  fn handle(&mut self, msg: RequestLogs, _ctx: &mut Context<Self>) -> Self::Result {
    use crate::db::model::logs::Log;
    use crate::db::schema::logs::dsl::*;
    let addr = msg.addr.clone();
    futures::executor::block_on(async {
      let p = &mut establish_connection_sqlite();
      let l = logs.select(Log::as_select()).load(p).unwrap();
      match addr {
        ActorAddr::Server(ad) => {
          ad.send(crate::actors::server::ReceiveLogs {}).await.unwrap();
        }
        ActorAddr::Session(ad) => {
          ad.send(crate::actors::session::ReceiveLogs {}).await.unwrap();
        }
      }
      addr.send()
    });
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
    let client_id = if let Some(id) = msg.id.clone() {
      id
    } else {
      uuid::Uuid::new_v4().to_string()
    };
    let client_name = msg.client_name.clone();
    let id1 = msg.id.clone();
    let addr = msg.addr.clone();
    let pool = self.dbtype.clone();
    let serv = self.server.clone();
    let secret: String =
      rand::rng().sample_iter(&rand::distr::Alphanumeric).take(32).map(char::from).collect();
    let futr = Box::pin(async move {
      if id1.is_none() {
        let b = actions::clients::add_client(pool, client, secret).await;

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
        // NOTE: RESTART WORK HERE
        let b = actions::clients::get_client(pool, client_id).await;
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
