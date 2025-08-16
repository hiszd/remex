use std::str::FromStr;

use actix::Actor;
use actix::Addr;
use actix::AsyncContext;
use common::db;
use common::db::Db;
use common::server;
use common::session;
use common::sessionmap;
use sqlx::postgres::PgConnectOptions;
use sqlx::PgPool;
use tracing::info;

//SERVER
use self::server::Server;

const ADDRESS: &str = "127.0.0.1:4269";

#[actix_web::main]
async fn main() {
  // TODO: find a way to log this to file for certain severity levels as well as log to terminal
  // when over a certain severity.
  tracing_subscriber::fmt::init();

  let dbpath = std::env::var("DATABASE_URL").unwrap();
  info!("DB Path: {}", dbpath);
  let options = PgConnectOptions::from_str(dbpath.as_str()).unwrap();

  let pool = PgPool::connect_with(options).await.unwrap();
  common::db::migrate(pool.clone()).await;

  let mut db: Option<Addr<Db>> = None;

  let server = Server::create(|ctx| {
    let dbaddr = crate::db::Db::new(pool.clone(), ctx.address()).start();

    db = Some(dbaddr.clone());

    Server {
      db: dbaddr,
      sessions: sessionmap::SessionMap::default(),
    }
  });
  session::tcp_server(ADDRESS, server, db.unwrap()).await;
}
