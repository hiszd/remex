use std::env;

use actix::Actor;
//SERVER
use tracing_subscriber;

use self::server::RemexServer;

mod args;
mod db;
mod endpoint;
mod server;
mod session;
mod sessionmap;

const ADDRESS: &str = "127.0.0.1:4269";

pub const SECRET: &str = "tZs3U%hqY^o$&*y%4HcF8&RyAKevUbZnkTsrjCzPGxfare3Yn9c7shVZETfPDPUc8xR%N38a!TL%2$WbkFhZqmH#jvw&d3^mryPD8Y8TqHoJHwyKSTJeQB7vK7QkW#&B";

#[actix_web::main]
async fn main() {
  tracing_subscriber::fmt::init();

  let server = RemexServer {
    sessions: sessionmap::SessionMap::default(),
    db: None,
  }
  .start();

  let options = sqlx::postgres::PgConnectOptions::new()
    .database("remex")
    .host("localhost")
    .username(&env::var("POSTGRES_USER").expect("POSTGRES_USER must be set"))
    .password(&env::var("POSTGRES_PASSWORD").expect("POSTGRES_PASSWORD must be set"));
  let pool = sqlx::PgPool::connect_with(options).await.unwrap();
  let _dbb = crate::db::Db {
    pool: pool.clone(),
    server: server.clone(),
  }
  .start();
  session::tcp_server(ADDRESS, server).await;
}
