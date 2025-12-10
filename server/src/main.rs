use actix::Actor;
use actix::AsyncContext;
use remex_core as core;
use remex_core::actors::server::Server;
use remex_core::db::clients::Pools;
//SERVER
use sqlx::migrate::MigrateDatabase;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;

const ADDRESS: &str = "127.0.0.1:4269";

#[actix_web::main]
async fn main() {
  tracing_subscriber::fmt::init();

  let filename = "main.db";

  if !sqlx::Sqlite::database_exists(filename).await.unwrap() {
    sqlx::Sqlite::create_database(filename).await.unwrap();
  }
  let options = SqliteConnectOptions::new().filename(filename);

  let pool = Pools::Sqlite(SqlitePool::connect_with(options).await.unwrap());

  let server = Server::create(move |ctx| {
    let dbb = core::db::Db {
      pool: pool.clone(),
      server: ctx.address(),
    }
    .start();

    Server {
      db: dbb,
      sessions: remex_core::sessionmap::SessionMap::default(),
    }
  });
  remex_core::actors::session::tcp_server(ADDRESS, server).await;
}
