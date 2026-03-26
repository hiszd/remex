use actix_web::{
  get,
  post,
  web,
  HttpResponse,
  Responder,
};
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

#[derive(Serialize, ToSchema)]
pub struct Client {
  pub id: String,
  pub secret: String,
  pub client_name: String,
  pub hardware_hash: String,
  pub created_at: Option<String>,
  pub updated_at: Option<String>,
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
  // TODO: Implement client retrieval
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
pub async fn create_client(form: web::Json<CreateClientForm>) -> impl Responder {
  // TODO: Implement client creation
  HttpResponse::Created().json(Client {
    id: "TODO".to_string(),
    secret: "TODO".to_string(),
    client_name: form.client_name.clone(),
    hardware_hash: form.hardware_hash.clone(),
    created_at: None,
    updated_at: None,
  })
}

#[derive(Serialize, ToSchema)]
pub struct ClientWithGroups {
  #[serde(flatten)]
  pub id: String,
  pub group_ids: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ClientWithExecutions {
  #[serde(flatten)]
  pub id: String,
  pub executions: Vec<String>,
  pub group_ids: Vec<String>,
}

#[utoipa::path(
  get,
  path = "/clients/{id}",
  responses(
    (status = 200, description = "Client found successfully", body = ClientWithGroups),
    (status = 404, description = "Client not found"),
  ),
  params(
    ("id" = String, Path, description = "Client ID")
  )
)]
#[get("/clients/{id}")]
pub async fn get_client_by_id(id: web::Path<String>) -> impl Responder {
  // TODO: Implement client lookup
  HttpResponse::NotFound().finish()
}