use std::path::Path;

use actix::{Actor, AsyncContext, Context, Handler, Message};
use tracing::info;

pub mod clients;
pub mod logs;

pub struct Db {
  pub pool: sqlx::SqlitePool,
  pub server: actix::Addr<crate::server::Server>,
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
      .run(&<sqlx::SqlitePool>::clone(&self.pool))
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
