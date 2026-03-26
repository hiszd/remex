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

pub mod data_gathering;

#[derive(Deserialize, ToSchema)]
pub struct CreateGroupForm {
  pub group_name: String,
  pub client_ids: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Group {
  pub id: String,
  pub group_name: String,
  pub created_at: String,
  pub updated_at: String,
  pub client_count: i64,
}

#[derive(Serialize, ToSchema)]
pub struct GroupWithClients {
  pub group: Group,
  pub clients: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct GroupJobStatusResponse {
  pub group_id: String,
  pub job_id: String,
  pub status: String,
  pub client_statuses: Vec<serde_json::Value>,
  pub total_clients: i64,
  pub completed_clients: i64,
  pub failed_clients: i64,
  pub running_clients: i64,
}

#[utoipa::path(
  get,
  path = "/groups",
  responses(
    (status = 200, description = "Groups found successfully", body = [Group]),
  ),
)]
#[get("/groups")]
pub async fn get_groups() -> impl Responder {
  // TODO: Implement group retrieval
  HttpResponse::Ok().json(vec![] as Vec<Group>)
}

#[utoipa::path(
  post,
  path = "/groups/new",
  request_body = CreateGroupForm,
  responses(
    (status = 201, description = "Group created successfully", body = Group),
  ),
)]
#[post("/groups/new")]
pub async fn create_group(form: web::Json<CreateGroupForm>) -> impl Responder {
  // TODO: Implement group creation
  HttpResponse::Created().json(Group {
    id: "TODO".to_string(),
    group_name: form.group_name.clone(),
    created_at: "TODO".to_string(),
    updated_at: "TODO".to_string(),
    client_count: 0,
  })
}

#[derive(Deserialize, ToSchema)]
pub struct GroupPath {
  pub group_id: String,
}

#[utoipa::path(
  get,
  path = "/groups/{group_id}/clients",
  responses(
    (status = 200, description = "Clients in group found successfully", body = [String]),
    (status = 404, description = "Group not found"),
  ),
  params(
    ("group_id" = String, Path, description = "Group ID")
  )
)]
#[get("/groups/{group_id}/clients")]
pub async fn get_group_clients(_path: web::Path<GroupPath>) -> impl Responder {
  // TODO: Implement group clients retrieval
  HttpResponse::Ok().json(vec![] as Vec<String>)
}

#[derive(Deserialize, ToSchema)]
pub struct GroupJobPath {
  pub group_id: String,
  pub job_id: String,
}

#[utoipa::path(
  get,
  path = "/groups/{group_id}/jobs/{job_id}/status",
  responses(
    (status = 200, description = "Group job status found successfully", body = GroupJobStatusResponse),
    (status = 404, description = "Group or job not found"),
  ),
  params(
    ("group_id" = String, Path, description = "Group ID"),
    ("job_id" = String, Path, description = "Job ID")
  )
)]
#[get("/groups/{group_id}/jobs/{job_id}/status")]
pub async fn get_group_job_status_handler(_path: web::Path<GroupJobPath>) -> impl Responder {
  // TODO: Implement group job status
  HttpResponse::NotFound().finish()
}

#[utoipa::path(
  get,
  path = "/groups/{group_id}",
  responses(
    (status = 200, description = "Group found successfully", body = Group),
    (status = 404, description = "Group not found"),
  ),
  params(
    ("group_id" = String, Path, description = "Group ID")
  )
)]
#[get("/groups/{group_id}")]
pub async fn get_group_by_id(_path: web::Path<GroupPath>) -> impl Responder {
  // TODO: Implement group lookup
  HttpResponse::NotFound().finish()
}

#[utoipa::path(
  get,
  path = "/groups/{group_id}/jobs",
  responses(
    (status = 200, description = "Jobs in group found successfully", body = [String]),
    (status = 404, description = "Group not found"),
  ),
  params(
    ("group_id" = String, Path, description = "Group ID")
  )
)]
#[get("/groups/{group_id}/jobs")]
pub async fn get_group_jobs(_path: web::Path<GroupPath>) -> impl Responder {
  // TODO: Implement group jobs retrieval
  HttpResponse::Ok().json(vec![] as Vec<String>)
}

#[derive(Deserialize, ToSchema)]
pub struct AddClientsToGroupForm {
  pub client_ids: Vec<String>,
}

#[utoipa::path(
  post,
  path = "/groups/{group_id}/clients",
  request_body = AddClientsToGroupForm,
  responses(
    (status = 200, description = "Clients added to group successfully"),
    (status = 404, description = "Group not found"),
  ),
  params(
    ("group_id" = String, Path, description = "Group ID")
  )
)]
#[post("/groups/{group_id}/clients")]
pub async fn add_clients_to_group(_path: web::Path<GroupPath>, _form: web::Json<AddClientsToGroupForm>) -> impl Responder {
  // TODO: Implement add clients to group
  HttpResponse::Ok().json("Clients added to group successfully")
}

#[derive(Deserialize, ToSchema)]
pub struct RemoveClientsFromGroupForm {
  pub client_ids: Vec<String>,
}

#[utoipa::path(
  post,
  path = "/groups/{group_id}/clients/remove",
  request_body = RemoveClientsFromGroupForm,
  responses(
    (status = 200, description = "Clients removed from group successfully"),
    (status = 404, description = "Group not found"),
  ),
  params(
    ("group_id" = String, Path, description = "Group ID")
  )
)]
#[post("/groups/{group_id}/clients/remove")]
pub async fn remove_clients_from_group(_path: web::Path<GroupPath>, _form: web::Json<RemoveClientsFromGroupForm>) -> impl Responder {
  // TODO: Implement remove clients from group
  HttpResponse::Ok().json("Clients removed from group successfully")
}