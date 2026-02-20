use actix::Actor;
use remex_core::actors::server::RemexServer;
pub mod secret;
//SERVER

const ADDRESS: &str = "127.0.0.1:4269";

#[actix_web::main]
async fn main() {
  tracing_subscriber::fmt::init();

  let server = RemexServer {
    sessions: remex_core::sessionmap::SessionMap::default(),
    migrated: false,
  }
  .start();
  remex_core::actors::session::tcp_server(ADDRESS, server).await;
}
