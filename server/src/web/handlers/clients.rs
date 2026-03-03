use actix_web::get;
use diesel::RunQueryDsl;
use remex_core::db::model::server::clients::Client;

#[utoipa::path(
  get,
  path = "clients",
  responses(
    (status = 200, description = "Clients found successfully", body = [Client]),
  ),
)]
#[get("/clients")]
async fn get_clients() -> impl actix_web::Responder {
  use remex_core::db::model::server::clients::Client;
  use remex_core::db::schema::server::clients;
  let mut pool = remex_core::db::establish_connection_postgres();
  let clients = clients::table.load::<Client>(&mut pool).unwrap();
  actix_web::HttpResponse::Ok().json(clients)
}
