use actix_cors::Cors;
use actix_web::{App, HttpServer};
use utoipa::OpenApi;

mod handlers;

#[derive(OpenApi)]
#[openapi(
    paths(handlers::clients::get_clients), // Register your functions here
    components(schemas(remex_core::db::model::server::clients::Client)) // Register your types
)]
struct ApiDoc;

#[allow(unused)]
pub fn generate_api() {
  tracing::info!("Generating openapi json");
  // ensure frontend directory exists
  if let Err(e) = std::fs::create_dir_all("./frontend") {
    tracing::error!("Failed to create frontend directory: {}", e);
  }
  // export openapi json
  std::fs::write(
    "./frontend/openapi.json",
    serde_json::to_string_pretty(&ApiDoc::openapi()).unwrap(),
  )
  .unwrap();
  tracing::info!("Exported openapi json");
}

pub fn start_web_server() -> actix_web::dev::Server {
  tracing::info!("Starting web server on 0.0.0.0:8989");

  generate_api(); // Generate the openapi.json file on startup

  HttpServer::new(move || {
    App::new()
      .service(
        utoipa_swagger_ui::SwaggerUi::new("/swagger-ui/{_:.*}")
          .url("/api-docs/openapi.json", ApiDoc::openapi()),
      )
      .service(handlers::clients::get_clients) // Register the API endpoint
      .wrap(
        Cors::default()
          .allowed_origin("http://localhost:5173") // Your Vue dev URL
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
