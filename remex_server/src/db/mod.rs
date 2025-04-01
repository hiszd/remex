use std::path::Path;

use actix::dev::ContextFutureSpawner;
use actix::{
  Actor, ActorContext, ActorFutureExt, AsyncContext, Context, Handler, Message, WrapFuture,
};
use futures::FutureExt;
use tracing::{error, info};

pub mod clients;
pub mod logs;

pub struct Db {
  pub pool: sqlx::PgPool,
  pub server: actix::Addr<crate::server::RemexServer>,
}

impl Db {
  pub async fn migrate(&self) {
    info!("migrating db {}", cfg!(debug_assertions));

    // Migrate the database
    let migrations = if !cfg!(debug_assertions) {
      // Productions migrations dir
      let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
      Path::new(&crate_dir).join("./migrations/prod")
    } else {
      // Development migrations dir
      let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
      Path::new(&crate_dir).join("./migrations/dev")
    };

    sqlx::migrate::Migrator::new(migrations)
      .await
      .unwrap()
      .run(&<sqlx::PgPool>::clone(&self.pool))
      .await
      .unwrap();
  }

  pub async fn connect(&mut self) {
  }

  pub async fn get_logs(&self) {
  }

  pub async fn get_cmds(&self) {
  }

  pub async fn new_log(&self, client: &str, message: &str, time_logged: chrono::NaiveDateTime) {
    logs::add_log(&self.pool.clone(), client, message, time_logged).await.unwrap();
  }
  pub async fn push_cmd() {
    // TODO: implement command push
  }
}

impl Actor for Db {
  type Context = Context<Db>;

  fn started(&mut self, ctx: &mut Context<Self>) -> () {
    // register self in Remex server. `AsyncContext::wait` register
    // future within context, but context waits until this future resolves
    // before processing any other events.
    let addr = ctx.address();
    self
      .server
      .send(crate::server::DbConnect { addr })
      .into_actor(self)
      .then(|res, _act, ctx| {
        match res {
          Ok(_) => {}
          // something is wrong with Remex server
          _ => ctx.stop(),
        }
        actix::fut::ready(())
      })
      .wait(ctx);
  }

  fn stopped(&mut self, _ctx: &mut Context<Self>) { self.pool.close(); }
}

#[derive(Message)]
#[rtype(result = "Vec<String>")]
pub struct GetLogs {}
impl Handler<GetLogs> for Db {
  type Result = Vec<String>;
  fn handle(&mut self, _msg: GetLogs, _ctx: &mut Context<Self>) -> Self::Result {
    let _lgs = futures::executor::block_on(async {
      sqlx::query!("SELECT * FROM logs").fetch_all(&self.pool).await.unwrap()
    });
    vec!["bob".to_owned()]
  }
}

#[derive(Debug)]
pub struct NewClient {
  pub id: Option<String>,
  pub clientname: String,
  pub addr: actix::Addr<crate::session::RemexSession>,
}
impl Message for NewClient {
  type Result = Result<(), anyhow::Error>;
}
impl Handler<NewClient> for Db {
  type Result = Result<(), anyhow::Error>;
  fn handle(&mut self, msg: NewClient, ctx: &mut Context<Self>) -> Self::Result {
    let clientname1 = msg.clientname.clone();
    let id1 = msg.id.clone();
    let addr = msg.addr.clone();
    let pool = self.pool.clone();
    let serv = self.server.clone();
    let futr = Box::pin(async move {
      if id1.is_none() {
        tracing::info!("Generating new id");
        let id = clients::generate_id(pool.clone()).await.unwrap();
        tracing::info!("Generated id: {}", id);
        let b = clients::add_client(pool, id, clientname1).await;

        match b {
          Ok(client) => {
            tracing::info!("Client added with id: {}", &client.id);
            serv.do_send(crate::server::DbClientIdentified {
              id: client.client_id,
              clientname: client.clientname.clone(),
              addr,
            });
          }
          Err(e) => error!("127 - db error: {}", e),
        }
      } else {
        tracing::info!("Using existing id");
        let b = clients::get_client(&pool, id1.clone().unwrap()).await;
        match b {
          Ok(client) => {
            serv.do_send(crate::server::DbClientIdentified {
              id: client.client_id,
              clientname: client.clientname.clone(),
              addr,
            });
          }
          Err(e) => error!("139 - db error: {} when getting client: {:?}", e, id1.clone()),
        }
      }
    });
    let fut = actix::fut::wrap_future::<_, Self>(futr);
    ctx.spawn(fut);
    Ok(())
  }
}

#[derive(Message)]
#[rtype(result = "()")]
struct NewLog {
  client: String,
  message: String,
  time_logged: chrono::NaiveDateTime,
}
impl Handler<NewLog> for Db {
  type Result = ();
  fn handle(&mut self, msg: NewLog, _ctx: &mut Context<Self>) {
    futures::executor::block_on(self.new_log(&msg.client, &msg.message, msg.time_logged));
  }
}
