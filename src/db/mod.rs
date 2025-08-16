use core::time::Duration;
use std::path::Path;
use std::time::Instant;

use actix::{Actor, AsyncContext, Context};
use tracing::{error, info, warn};

pub mod clients;
pub mod executors;
pub mod logs;

struct UpdateInfo {
  machineid: String,
  lastchecked: Instant,
  lastupdated: Instant,
}

pub struct Db {
  pub pool: sqlx::PgPool,
  pub server: actix::Addr<crate::server::Server>,
  updates: Vec<UpdateInfo>,
  lastchecked: Option<Instant>,
}

pub async fn migrate(pool: sqlx::PgPool) {
  warn!("Migrating db");

  let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

  sqlx::migrate::Migrator::new(Path::new(&crate_dir).join("./migrations"))
    .await
    .unwrap()
    .run(&<sqlx::PgPool>::clone(&pool))
    .await
    .unwrap();
}

impl Db {
  pub fn new(pool: sqlx::PgPool, server: actix::Addr<crate::server::Server>) -> Db {
    Db {
      pool,
      server,
      updates: Vec::new(),
      lastchecked: None,
    }
  }

  pub async fn connect(&mut self) {
  }

  pub async fn get_logs(&self) {
  }

  pub async fn get_cmds(&self) {
  }

  pub async fn new_log(&self, client: &str, message: &str, time_logged: chrono::NaiveDateTime) {
    logs::add_log(&self.pool.clone(), client, message, time_logged)
      .await
      .unwrap();
  }
  pub async fn push_cmd() {
    // TODO: implement command push
  }
  pub fn update(&self, ctx: &mut Context<Self>) {
    ctx.run_interval(Duration::new(10, 0), |act, ctx| {
      if act.lastchecked.is_none() {}

      act.lastchecked = Some(Instant::now());
    });
  }
}

impl Actor for Db {
  type Context = Context<Db>;

  fn started(&mut self, _ctx: &mut Context<Self>) {
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
