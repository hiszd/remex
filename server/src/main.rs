use actix::Actor;
use actix::AsyncContext;
use remex_core as core;
use remex_core::actors::server::Server;
use remex_core::db::clients::Pools;
//SERVER
use sqlx::postgres::PgConnectOptions;
use sqlx::PgPool;

const ADDRESS: &str = "127.0.0.1:4269";

#[actix_web::main]
async fn main() {
  tracing_subscriber::fmt::init();

  let options = PgConnectOptions::new()
    .host("192.168.10.133")
    .username("postgres")
    .password("H@ck3r345")
    .database("remex");

  let pool = Pools::Postgres(PgPool::connect_with(options).await.unwrap());

  let server = Server::create(move |ctx| {
    let dbb = core::db::Db {
      pool: pool.clone(),
      server: ctx.address(),
    };

    Server {
      db: dbb.start(),
      sessions: remex_core::sessionmap::SessionMap::default(),
    }
  });
  remex_core::actors::session::tcp_server(ADDRESS, server).await;
}
