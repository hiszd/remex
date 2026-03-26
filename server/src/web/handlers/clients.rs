use actix_web::{
  get,
  post,
  web,
  HttpResponse,
  Responder,
};
use remex_core::db::surreal::models::Client;
use serde::{
  Deserialize,
  Serialize,
};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct CreateClientForm {
  pub client_name: String,
  pub hardware_hash: String,
}

#[utoipa::path(
  get,
  path = "/clients",
  responses(
    (status = 200, description = "Clients found successfully", body = [Client]),
  ),
)]
#[get("/clients")]
pub async fn get_clients() -> impl Responder {
  HttpResponse::Ok().json(vec![] as Vec<Client>)
}

#[utoipa::path(
  post,
  path = "/clients/new",
  request_body = CreateClientForm,
  responses(
    (status = 201, description = "Client created successfully", body = Client),
  ),
)]
#[post("/clients/new")]
pub async fn create_client(_form: web::Json<CreateClientForm>) -> impl Responder {
  HttpResponse::Ok().json(Client {
    id: None,
    secret: String::new(),
    client_name: String::new(),
    hardware_hash: String::new(),
    created_at: None,
    updated_at: None,
  })
}

#[get("/clients/{id}")]
pub async fn get_client_by_id(_id: web::Path<String>) -> impl Responder {
  HttpResponse::Ok().json(Client {
    id: None,
    secret: String::new(),
    client_name: String::new(),
    hardware_hash: String::new(),
    created_at: None,
    updated_at: None,
  })
}
