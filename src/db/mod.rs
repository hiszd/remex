use std::path::Path;

use actix::{Actor, AsyncContext, Context};
use tracing::{debug, info};

pub mod clients;
pub mod logs;

pub struct Db {
  pub pool: sqlx::SqlitePool,
  pub server: actix::Addr<crate::server::Server>,
}

pub async fn migrate(pool: sqlx::SqlitePool) {
  info!("Migrating db");

  let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

  sqlx::migrate::Migrator::new(Path::new(&crate_dir).join("./migrations"))
    .await
    .unwrap()
    .run(&<sqlx::SqlitePool>::clone(&pool))
    .await
    .unwrap();
}

impl Db {
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

  fn started(&mut self, _ctx: &mut Context<Self>) -> () {
  }

  fn stopped(&mut self, ctx: &mut Context<Self>) {
    let pool = self.pool.clone();
    let futr = Box::pin(async move {
      pool.close().await;
    });
    let fut = actix::fut::wrap_future::<_, Self>(futr);
    ctx.spawn(fut);
  }
}
