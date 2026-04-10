use actix_web::{
  App,
  HttpServer,
};

pub fn start_web_server() -> actix_web::dev::Server {
  tracing::info!("Starting web server on 0.0.0.0:8989");

  HttpServer::new(App::new)
    .disable_signals()
    .shutdown_timeout(5)
    .bind(("0.0.0.0", 8989))
    .unwrap()
    .run()
}
