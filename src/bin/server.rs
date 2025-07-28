use actix::Actor;
use actix::Addr;
use actix::AsyncContext;
use common::db;
use common::db::Db;
use common::server;
use common::session;
use common::sessionmap;
use sqlx::migrate::MigrateDatabase;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
//SERVER

use self::server::Server;

const ADDRESS: &str = "127.0.0.1:4269";

#[actix_web::main]
async fn main() {
    // TODO: find a way to log this to file for certain severity levels as well as log to terminal
    // when over a certain severity.
  tracing_subscriber::fmt::init();

  let filename = "main.db";

  if !sqlx::Sqlite::database_exists(filename).await.unwrap() {
    sqlx::Sqlite::create_database(filename).await.unwrap();
  }
  let options = SqliteConnectOptions::new().filename(filename);

  let pool = SqlitePool::connect_with(options).await.unwrap();
  common::db::migrate(pool.clone()).await;

  let mut db: Option<Addr<Db>> = None;

  let server = Server::create(|ctx| {
    let dbaddr = crate::db::Db {
      pool: pool.clone(),
      server: ctx.address(),
    }
    .start();

    db = Some(dbaddr.clone());

    Server {
      db: dbaddr,
      sessions: sessionmap::SessionMap::default(),
    }
  });
  session::tcp_server(ADDRESS, server, db.unwrap()).await;
}
