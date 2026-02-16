use actix::Actor;
use actix::AsyncContext;
use remex_core as core;
use remex_core::actors::server::Server;
use remex_core::db::Connections;
//SERVER

pub const SHARED_MIGRATIONS: diesel_migrations::EmbeddedMigrations =
  diesel_migrations::embed_migrations!("../migrations/shared");
pub const SERVER_MIGRATIONS: diesel_migrations::EmbeddedMigrations =
  diesel_migrations::embed_migrations!("../migrations/server");

const ADDRESS: &str = "127.0.0.1:4269";

#[actix_web::main]
async fn main() {
  tracing_subscriber::fmt::init();
  let connection = &mut remex_core::db::establish_connection();

  let options = PgConnectOptions::new()
    .host("192.168.10.133")
    .username("postgres")
    .password("H@ck3r345")
    .database("remex");

  let pool = Connections::Postgres(PgPool::connect_with(options).await.unwrap());

  let server = Server::create(move |ctx| {
    let dbb = core::db::Db {
      dbtype: pool.clone(),
      server: ctx.address(),
    };

    Server {
      db: dbb.start(),
      sessions: remex_core::sessionmap::SessionMap::default(),
    }
  });
  remex_core::actors::session::tcp_server(ADDRESS, server).await;
}
