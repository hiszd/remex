use actix_web::{
  get,
  post,
  web,
  HttpResponse,
  Responder,
};
use remex_core::db::surreal::models::Group;
use serde::{
  Deserialize,
  Serialize,
};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct CreateGroupForm {
  pub group_name: String,
}

#[derive(Serialize, ToSchema)]
pub struct GroupWithClients {
  pub group: Group,
  pub clients: Vec<String>,
}

#[get("/groups")]
pub async fn get_groups() -> impl Responder {
  HttpResponse::Ok().json(vec![] as Vec<Group>)
}

#[post("/groups/new")]
pub async fn create_group(_form: web::Json<CreateGroupForm>) -> impl Responder {
  HttpResponse::Ok().json(Group {
    id: None,
    group_name: String::new(),
    created_at: None,
    updated_at: None,
  })
}

#[get("/groups/{id}")]
pub async fn get_group_by_id(_id: web::Path<String>) -> impl Responder {
  HttpResponse::Ok().json(Group {
    id: None,
    group_name: String::new(),
    created_at: None,
    updated_at: None,
  })
}

#[get("/groups/{id}/clients")]
pub async fn get_group_clients(_id: web::Path<String>) -> impl Responder {
  HttpResponse::Ok().json(vec![] as Vec<String>)
}

#[get("/groups/{id}/jobs")]
pub async fn get_group_jobs(_id: web::Path<String>) -> impl Responder {
  HttpResponse::Ok().json(vec![] as Vec<String>)
}

#[derive(Serialize, ToSchema)]
pub struct GroupJobStatusResponse {
  pub group_id: String,
  pub jobs: Vec<String>,
}

#[get("/groups/{id}/job-status")]
pub async fn get_group_job_status_handler(_id: web::Path<String>) -> impl Responder {
  HttpResponse::Ok().json(GroupJobStatusResponse {
    group_id: String::new(),
    jobs: vec![],
  })
}

#[derive(Deserialize, ToSchema)]
pub struct AddClientsToGroupForm {
  pub client_ids: Vec<String>,
}

#[post("/groups/{id}/clients")]
pub async fn add_clients_to_group(
  _id: web::Path<String>,
  _form: web::Json<AddClientsToGroupForm>,
) -> impl Responder {
  HttpResponse::Ok().json(())
}

#[derive(Deserialize, ToSchema)]
pub struct RemoveClientsFromGroupForm {
  pub client_ids: Vec<String>,
}

#[post("/groups/{id}/clients/remove")]
pub async fn remove_clients_from_group(
  _id: web::Path<String>,
  _form: web::Json<RemoveClientsFromGroupForm>,
) -> impl Responder {
  HttpResponse::Ok().json(())
}
