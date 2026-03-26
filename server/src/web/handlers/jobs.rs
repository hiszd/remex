use actix_web::{
  delete,
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
pub struct CreateJobForm {
  pub job_name: String,
  pub job_type: String,
  pub job_status: String,
  pub job_shell: String,
  pub job_command: String,
  pub group_ids: Option<Vec<String>>,
}

#[derive(Deserialize, ToSchema)]
pub struct JobClientAction {
  pub job_id: String,
  pub client_id: String,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateJobGroupsForm {
  pub group_ids: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct Job {
  pub id: String,
  pub job_name: String,
  pub job_type: String,
  pub job_status: String,
  pub job_shell: String,
  pub job_command: String,
  pub created_at: Option<String>,
  pub updated_at: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct JobWithClients {
  #[serde(flatten)]
  pub job: Job,
  pub clients: Vec<String>,
}

#[utoipa::path(
  get,
  path = "/jobs",
  responses(
    (status = 200, description = "Jobs found successfully", body = [JobWithClients]),
  ),
)]
#[get("/jobs")]
pub async fn get_jobs() -> impl Responder {
  // TODO: Implement job retrieval
  HttpResponse::Ok().json(vec![] as Vec<JobWithClients>)
}

#[utoipa::path(
  get,
  path = "/jobs/{id}",
  responses(
    (status = 200, description = "Job found successfully", body = JobWithClients),
    (status = 404, description = "Job not found"),
  ),
  params(
    ("id" = String, Path, description = "Job ID")
  )
)]
#[get("/jobs/{id}")]
pub async fn get_job_by_id(id: web::Path<String>) -> impl Responder {
  // TODO: Implement job lookup
  HttpResponse::NotFound().finish()
}

#[utoipa::path(
  post,
  path = "/jobs/new",
  request_body = CreateJobForm,
  responses(
    (status = 201, description = "Job created successfully", body = Job),
  ),
)]
#[post("/jobs/new")]
pub async fn create_job(form: web::Json<CreateJobForm>) -> impl Responder {
  // TODO: Implement job creation
  HttpResponse::Created().json(Job {
    id: "TODO".to_string(),
    job_name: form.job_name.clone(),
    job_type: form.job_type.clone(),
    job_status: form.job_status.clone(),
    job_shell: form.job_shell.clone(),
    job_command: form.job_command.clone(),
    created_at: None,
    updated_at: None,
  })
}

#[utoipa::path(
  post,
  path = "/jobs/update",
  responses(
    (status = 200, description = "Job updated successfully", body = Job),
    (status = 404, description = "Job not found"),
  ),
)]
#[post("/jobs/update")]
pub async fn update_job(_job_update: web::Json<serde_json::Value>) -> impl Responder {
  // TODO: Implement job update
  HttpResponse::NotFound().finish()
}

#[derive(Deserialize, ToSchema)]
pub struct JobGroupPath {
  pub job_id: String,
}

#[utoipa::path(
  post,
  path = "/jobs/{job_id}/groups",
  request_body = UpdateJobGroupsForm,
  responses(
    (status = 200, description = "Job groups updated successfully"),
    (status = 404, description = "Job not found"),
  ),
  params(
    ("job_id" = String, Path, description = "Job ID")
  )
)]
#[post("/jobs/{job_id}/groups")]
pub async fn update_job_groups(
  _path: web::Path<JobGroupPath>,
  _form: web::Json<UpdateJobGroupsForm>,
) -> impl Responder {
  // TODO: Implement job group update
  HttpResponse::NotFound().finish()
}

#[utoipa::path(
  post,
  path = "/jobs/addclients",
  request_body = [JobClientAction],
  responses(
    (status = 200, description = "Clients added to jobs successfully"),
  ),
)]
#[post("/jobs/addclients")]
pub async fn add_clients_to_jobs(_actions: web::Json<Vec<JobClientAction>>) -> impl Responder {
  // TODO: Implement add clients to jobs
  HttpResponse::Ok().finish()
}

#[utoipa::path(
  post,
  path = "/jobs/removeclients",
  request_body = [JobClientAction],
  responses(
    (status = 200, description = "Clients removed from jobs successfully"),
  ),
)]
#[post("/jobs/removeclients")]
pub async fn remove_clients_from_jobs(_actions: web::Json<Vec<JobClientAction>>) -> impl Responder {
  // TODO: Implement remove clients from jobs
  HttpResponse::Ok().finish()
}

#[derive(Serialize, ToSchema)]
pub struct ClientJobStatusResponse {
  pub client_id: String,
  pub client_name: String,
  pub status: String,
  pub latest_execution_id: Option<String>,
  pub latest_execution_timestamp: Option<String>,
  pub execution_count: i64,
}

#[utoipa::path(
  get,
  path = "/jobs/{job_id}/client-statuses",
  responses(
    (status = 200, description = "Client statuses for job found successfully", body = [ClientJobStatusResponse]),
    (status = 404, description = "Job not found"),
  ),
  params(
    ("job_id" = String, Path, description = "Job ID")
  )
)]
#[get("/jobs/{job_id}/client-statuses")]
pub async fn get_job_client_statuses(id: web::Path<String>) -> impl Responder {
  // TODO: Implement job client status retrieval
  let _job_id = id.into_inner();
  HttpResponse::Ok().json(vec![] as Vec<ClientJobStatusResponse>)
}

#[derive(Serialize, ToSchema)]
pub struct JobWithGroups {
  pub id: String,
  pub group_name: String,
  pub created_at: String,
  pub updated_at: String,
}

#[utoipa::path(
  get,
  path = "/jobs/{id}/groups",
  responses(
    (status = 200, description = "Groups for job found successfully", body = [JobWithGroups]),
    (status = 404, description = "Job not found"),
  ),
  params(
    ("id" = String, Path, description = "Job ID")
  )
)]
#[get("/jobs/{id}/groups")]
pub async fn get_job_groups(id: web::Path<String>) -> impl Responder {
  // TODO: Implement job groups retrieval
  let _id = id.into_inner();
  HttpResponse::Ok().json(vec![] as Vec<JobWithGroups>)
}

#[utoipa::path(
  delete,
  path = "/jobs/{id}",
  responses(
    (status = 204, description = "Job deleted successfully"),
    (status = 404, description = "Job not found"),
  ),
  params(
    ("id" = String, Path, description = "Job ID")
  )
)]
#[delete("/jobs/{id}")]
pub async fn delete_job(id: web::Path<String>) -> impl Responder {
  // TODO: Implement job deletion
  let _id = id.into_inner();
  HttpResponse::NoContent().finish()
}