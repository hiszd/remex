use actix_cors::Cors;
use actix_web::{
  App,
  HttpServer,
};

mod handlers;

pub fn start_web_server() -> actix_web::dev::Server {
  tracing::info!("Starting web server on 0.0.0.0:8989");

  HttpServer::new(move || {
    App::new()
      .service(handlers::clients::get_clients)
      .service(handlers::clients::create_client)
      .service(handlers::clients::get_client_by_id)
      .service(handlers::jobs::get_jobs)
      .service(handlers::jobs::get_job_by_id)
      .service(handlers::jobs::create_job)
      .service(handlers::jobs::update_job)
      .service(handlers::jobs::delete_job)
      .service(handlers::jobs::update_job_groups)
      .service(handlers::jobs::add_clients_to_jobs)
      .service(handlers::jobs::remove_clients_from_jobs)
      .service(handlers::jobs::get_job_client_statuses)
      .service(handlers::jobs::get_job_groups)
      .service(handlers::groups::get_groups)
      .service(handlers::groups::create_group)
      .service(handlers::groups::get_group_by_id)
      .service(handlers::groups::get_group_clients)
      .service(handlers::groups::get_group_jobs)
      .service(handlers::groups::get_group_job_status_handler)
      .service(handlers::groups::add_clients_to_group)
      .service(handlers::groups::remove_clients_from_group)
      .wrap(
        Cors::default()
          .allowed_origin("http://localhost:5173")
          .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
          .allowed_headers(vec![
            actix_web::http::header::AUTHORIZATION,
            actix_web::http::header::ACCEPT,
          ])
          .allowed_header(actix_web::http::header::CONTENT_TYPE)
          .max_age(3600),
      )
  })
  .disable_signals()
  .shutdown_timeout(5)
  .bind(("0.0.0.0", 8989))
  .unwrap()
  .run()
}
