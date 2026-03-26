use actix_web::{
  delete,
  get,
  post,
  web,
  HttpResponse,
  Responder,
};
use remex_core::db::surreal::models::Job;
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
  pub job_shell: String,
  pub job_command: String,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateJobForm {
  pub job_name: Option<String>,
  pub job_type: Option<String>,
  pub job_status: Option<String>,
  pub job_shell: Option<String>,
  pub job_command: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct JobWithClients {
  pub job: Job,
  pub clients: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct JobWithGroups {
  pub job: Job,
  pub groups: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct ClientJobStatusResponse {
  pub client_id: String,
  pub client_name: String,
  pub status: String,
}

#[get("/jobs")]
pub async fn get_jobs() -> impl Responder {
  HttpResponse::Ok().json(vec![] as Vec<Job>)
}

#[get("/jobs/{id}")]
pub async fn get_job_by_id(_id: web::Path<String>) -> impl Responder {
  HttpResponse::Ok().json(Job {
    id: None,
    job_name: String::new(),
    job_type: remex_core::db::surreal::models::JobType::Instant,
    job_status: remex_core::db::surreal::models::JobStatus::Pending,
    job_shell: String::new(),
    job_command: String::new(),
    created_at: None,
    updated_at: None,
  })
}

#[post("/jobs/new")]
pub async fn create_job(_form: web::Json<CreateJobForm>) -> impl Responder {
  HttpResponse::Ok().json(Job {
    id: None,
    job_name: String::new(),
    job_type: remex_core::db::surreal::models::JobType::Instant,
    job_status: remex_core::db::surreal::models::JobStatus::Pending,
    job_shell: String::new(),
    job_command: String::new(),
    created_at: None,
    updated_at: None,
  })
}

#[post("/jobs/{id}")]
pub async fn update_job(
  _id: web::Path<String>,
  _form: web::Json<UpdateJobForm>,
) -> impl Responder {
  HttpResponse::Ok().json(Job {
    id: None,
    job_name: String::new(),
    job_type: remex_core::db::surreal::models::JobType::Instant,
    job_status: remex_core::db::surreal::models::JobStatus::Pending,
    job_shell: String::new(),
    job_command: String::new(),
    created_at: None,
    updated_at: None,
  })
}

#[delete("/jobs/{id}")]
pub async fn delete_job(_id: web::Path<String>) -> impl Responder {
  HttpResponse::Ok().json(())
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateJobGroupsForm {
  pub group_ids: Vec<String>,
}

#[post("/jobs/{id}/groups")]
pub async fn update_job_groups(
  _id: web::Path<String>,
  _form: web::Json<UpdateJobGroupsForm>,
) -> impl Responder {
  HttpResponse::Ok().json(())
}

#[derive(Deserialize, ToSchema)]
pub struct JobClientAction {
  pub client_ids: Vec<String>,
}

#[post("/jobs/{id}/clients/add")]
pub async fn add_clients_to_jobs(
  _id: web::Path<String>,
  _form: web::Json<JobClientAction>,
) -> impl Responder {
  HttpResponse::Ok().json(())
}

#[post("/jobs/{id}/clients/remove")]
pub async fn remove_clients_from_jobs(
  _id: web::Path<String>,
  _form: web::Json<JobClientAction>,
) -> impl Responder {
  HttpResponse::Ok().json(())
}

#[get("/jobs/{id}/client-statuses")]
pub async fn get_job_client_statuses(_id: web::Path<String>) -> impl Responder {
  HttpResponse::Ok().json(vec![] as Vec<ClientJobStatusResponse>)
}

#[get("/jobs/{id}/groups")]
pub async fn get_job_groups(_id: web::Path<String>) -> impl Responder {
  HttpResponse::Ok().json(vec![] as Vec<String>)
}
