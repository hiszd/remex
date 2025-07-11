use actix::Actor;
use actix::AsyncContext;
use common::db;
use common::server;
use common::session;
use common::sessionmap;
use sqlx::migrate::MigrateDatabase;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
//SERVER
use tracing_subscriber;

use self::server::Server;

const ADDRESS: &str = "127.0.0.1:4269";

#[actix_web::main]
async fn main() {
  tracing_subscriber::fmt::init();

  let filename = "main.db";

  if !sqlx::Sqlite::database_exists(filename).await.unwrap() {
    sqlx::Sqlite::create_database(filename).await.unwrap();
  }
  let options = SqliteConnectOptions::new().filename(filename);

  let pool = SqlitePool::connect_with(options).await.unwrap();

  let server = Server::create(move |ctx| {
    let dbb = crate::db::Db {
      pool: pool.clone(),
      server: ctx.address(),
    }
    .start();

    Server {
      db: dbb,
      sessions: sessionmap::SessionMap::default(),
    }
  });
  session::tcp_server(ADDRESS, server).await;
}
