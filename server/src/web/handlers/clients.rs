use actix_web::{
  get,
  post,
  web,
  HttpResponse,
  Responder,
};
use diesel::prelude::*;
use remex_core::db::{
  dal::clients::Client,
  model::server::{
    clients::{
      ClientSRV,
      NewClientSRV,
    },
    executions::ExecutionSRV,
  },
  schema::server::groups_clients,
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

#[utoipa::path(
  get,
  path = "/clients",
  responses(
    (status = 200, description = "Clients found successfully", body = [Client]),
  ),
)]
#[get("/clients")]
pub async fn get_clients() -> impl Responder {
  use remex_core::db::schema::server::clients;
  let mut pool = remex_core::db::establish_connection_postgres();
  let clients: Vec<Client> = clients::table
    .load::<ClientSRV>(&mut pool)
    .unwrap()
    .iter()
    .map(|c| c.clone().into())
    .collect();
  HttpResponse::Ok().json(clients)
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
  use remex_core::db::schema::server::clients;
  let mut pool = remex_core::db::establish_connection_postgres();

  let new_client = NewClientSRV {
    id: uuid::Uuid::new_v4().to_string(),
    secret: remex_core::utils::generate_secret(true),
    client_name: form.client_name.clone(),
    hardware_hash: form.hardware_hash.clone(),
  };

  let client = diesel::insert_into(clients::table)
    .values(&new_client)
    .get_result::<ClientSRV>(&mut pool)
    .unwrap();

  HttpResponse::Created().json(client)
}

#[derive(Serialize, ToSchema)]
pub struct ClientWithGroups {
  #[serde(flatten)]
  pub client: ClientSRV,
  pub group_ids: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ClientWithExecutions {
  #[serde(flatten)]
  pub client: ClientSRV,
  pub executions: Vec<ExecutionSRV>,
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
  use remex_core::db::schema::server::clients;
  let mut pool = remex_core::db::establish_connection_postgres();

  let client_id = id.into_inner();

  let client = clients::table
    .filter(clients::id.eq(&client_id))
    .first::<ClientSRV>(&mut pool)
    .optional()
    .unwrap();

  if let Some(client) = client {
    let group_ids: Vec<String> = groups_clients::table
      .filter(groups_clients::client_id.eq(&client_id))
      .select(groups_clients::group_id)
      .load::<Option<String>>(&mut pool)
      .unwrap()
      .into_iter()
      .flatten()
      .collect();

    HttpResponse::Ok().json(ClientWithGroups {
      client,
      group_ids,
    })
  } else {
    HttpResponse::NotFound().finish()
  }
}
