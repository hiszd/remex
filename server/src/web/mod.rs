use actix_web::{
  App,
  HttpServer,
};

pub fn start_web_server() -> actix_web::dev::Server {
  HttpServer::new(App::new)
    .disable_signals()
    .shutdown_timeout(5)
    .bind(("0.0.0.0", 8989))
    .unwrap()
    .run()
}
