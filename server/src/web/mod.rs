use actix_cors::Cors;
use actix_web::{
  App,
  HttpServer,
};
use utoipa::OpenApi;

mod handlers;

#[derive(OpenApi)]
#[openapi(
  paths(
    handlers::clients::get_clients,
    handlers::clients::create_client,
    handlers::clients::get_client_by_id,
    handlers::jobs::get_jobs,
    handlers::jobs::get_job_by_id,
    handlers::jobs::create_job,
    handlers::jobs::update_job,
    handlers::jobs::delete_job,
    handlers::jobs::update_job_groups,
    handlers::jobs::add_clients_to_jobs,
    handlers::jobs::remove_clients_from_jobs,
    handlers::jobs::get_job_client_statuses,
    handlers::jobs::get_job_groups,
    handlers::groups::get_groups,
    handlers::groups::create_group,
    handlers::groups::get_group_by_id,
    handlers::groups::get_group_clients,
    handlers::groups::get_group_jobs,
    handlers::groups::get_group_job_status_handler,
    handlers::groups::add_clients_to_group,
    handlers::groups::remove_clients_from_group
  ),
  components(schemas(
    remex_core::db::model::server::clients::ClientSRV,
    handlers::clients::CreateClientForm,
    remex_core::db::model::server::jobs::JobSRV,
    remex_core::db::model::server::jobs::UpdateJobSRV,
    handlers::jobs::JobWithClients,
    handlers::jobs::CreateJobForm,
    handlers::jobs::JobClientAction,
    handlers::jobs::UpdateJobGroupsForm,
    handlers::jobs::JobGroupPath,
    handlers::jobs::ClientJobStatusResponse,
    handlers::jobs::JobWithGroups,
    handlers::groups::Group,
    handlers::groups::GroupWithClients,
    handlers::groups::GroupJobStatusResponse,
    handlers::groups::GroupPath,
    handlers::groups::GroupJobPath,
    handlers::groups::CreateGroupForm,
    handlers::groups::AddClientsToGroupForm,
    handlers::groups::RemoveClientsFromGroupForm,
    handlers::jobs::data_gathering::ClientStatusSummary,
    handlers::jobs::data_gathering::GroupJobStatusMetadata
  ))
)]
struct ApiDoc;

#[allow(unused)]
pub fn generate_api() {
  tracing::info!("Generating openapi json");
  // ensure frontend directory exists
  if let Err(e) = std::fs::create_dir_all("./server/frontend") {
    tracing::error!("Failed to create frontend directory: {}", e);
  }
  // export openapi json
  std::fs::write(
    "./server/frontend/openapi.json",
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
