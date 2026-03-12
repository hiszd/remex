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
  model::server::clients::{
    ClientSRV,
    NewClientSRV,
  },
};
use serde::Deserialize;
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
