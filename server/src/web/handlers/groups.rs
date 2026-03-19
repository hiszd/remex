use actix_web::{
  get,
  post,
  web,
  HttpResponse,
  Responder,
};
use diesel::prelude::*;
use remex_core::db::{
  dal::job_status::{
    get_group_job_status,
    ClientStatusSummary,
  },
  model::server::{
    clients::ClientSRV,
    groups::GroupSRV,
    jobs::JobSRV,
  },
};
use serde::{
  Deserialize,
  Serialize,
};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
pub struct CreateGroupForm {
  pub group_name: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Group {
  pub id: String,
  pub group_name: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

impl From<GroupSRV> for Group {
  fn from(group: GroupSRV) -> Self {
    Group {
      id: group.id,
      group_name: group.group_name,
      created_at: group.created_at,
      updated_at: group.updated_at,
    }
  }
}

#[derive(Serialize, ToSchema)]
pub struct GroupWithClients {
  pub group: Group,
  pub clients: Vec<ClientSRV>,
}

#[derive(Serialize, ToSchema)]
pub struct GroupJobStatusResponse {
  pub group_id: String,
  pub job_id: String,
  pub status: String,
  pub client_statuses: Vec<ClientStatusSummary>,
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
  use remex_core::db::schema::server::groups;
  let mut pool = remex_core::db::establish_connection_postgres();

  let groups: Vec<Group> = groups::table
    .load::<GroupSRV>(&mut pool)
    .unwrap()
    .into_iter()
    .map(|g| g.into())
    .collect();

  HttpResponse::Ok().json(groups)
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
  use remex_core::db::{
    model::server::groups::NewGroupSRV,
    schema::server::groups,
  };
  let mut pool = remex_core::db::establish_connection_postgres();

  let new_group = NewGroupSRV {
    id: uuid::Uuid::now_v7().to_string(),
    group_name: form.group_name.clone(),
  };

  let group = diesel::insert_into(groups::table)
    .values(&new_group)
    .get_result::<GroupSRV>(&mut pool)
    .unwrap();

  HttpResponse::Created().json(Group::from(group))
}

#[derive(Deserialize, ToSchema)]
pub struct GroupPath {
  pub group_id: String,
}

#[utoipa::path(
  get,
  path = "/groups/{group_id}/clients",
  responses(
    (status = 200, description = "Clients in group found successfully", body = [ClientSRV]),
    (status = 404, description = "Group not found"),
  ),
  params(
    ("group_id" = String, Path, description = "Group ID")
  )
)]
#[get("/groups/{group_id}/clients")]
pub async fn get_group_clients(path: web::Path<GroupPath>) -> impl Responder {
  use remex_core::db::schema::server::groups_clients;
  let mut pool = remex_core::db::establish_connection_postgres();

  let group_id = path.group_id.clone();

  let client_ids: Vec<String> = groups_clients::table
    .filter(groups_clients::group_id.eq(&group_id))
    .select(groups_clients::client_id)
    .load::<Option<String>>(&mut pool)
    .unwrap()
    .into_iter()
    .flatten()
    .collect();

  let clients: Vec<ClientSRV> = remex_core::db::schema::server::clients::table
    .filter(
      remex_core::db::schema::server::clients::id
        .nullable()
        .eq_any(client_ids),
    )
    .load::<ClientSRV>(&mut pool)
    .unwrap_or_default();

  HttpResponse::Ok().json(clients)
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
pub async fn get_group_job_status_handler(path: web::Path<GroupJobPath>) -> impl Responder {
  let mut pool = remex_core::db::establish_connection_postgres();

  let group_id = path.group_id.clone();
  let job_id = path.job_id.clone();

  let result = get_group_job_status(&mut pool, &group_id, &job_id);

  match result {
    Ok((status, metadata)) => HttpResponse::Ok().json(GroupJobStatusResponse {
      group_id,
      job_id,
      status,
      client_statuses: metadata.client_statuses,
      total_clients: metadata.total_clients,
      completed_clients: metadata.completed_clients,
      failed_clients: metadata.failed_clients,
      running_clients: metadata.running_clients,
    }),
    Err(_) => HttpResponse::NotFound().finish(),
  }
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
pub async fn get_group_by_id(path: web::Path<GroupPath>) -> impl Responder {
  use remex_core::db::schema::server::groups;
  let mut pool = remex_core::db::establish_connection_postgres();

  let group_id = path.group_id.clone();

  let group = groups::table
    .filter(groups::id.eq(&group_id))
    .first::<GroupSRV>(&mut pool)
    .optional()
    .unwrap();

  match group {
    Some(g) => HttpResponse::Ok().json(Group::from(g)),
    None => {
      tracing::error!("Group {} not found", group_id);
      HttpResponse::NotFound().finish()
    }
  }
}

#[utoipa::path(
  get,
  path = "/groups/{group_id}/jobs",
  responses(
    (status = 200, description = "Jobs in group found successfully", body = [JobSRV]),
    (status = 404, description = "Group not found"),
  ),
  params(
    ("group_id" = String, Path, description = "Group ID")
  )
)]
#[get("/groups/{group_id}/jobs")]
pub async fn get_group_jobs(path: web::Path<GroupPath>) -> impl Responder {
  use remex_core::db::schema::server::{
    jobs,
    jobs_groups,
  };
  let mut pool = remex_core::db::establish_connection_postgres();

  let group_id = path.group_id.clone();

  let job_ids: Vec<String> = jobs_groups::table
    .filter(jobs_groups::group_id.eq(&group_id))
    .select(jobs_groups::job_id)
    .load::<Option<String>>(&mut pool)
    .unwrap()
    .into_iter()
    .flatten()
    .collect();

  let group_jobs: Vec<JobSRV> = jobs::table
    .filter(jobs::id.nullable().eq_any(job_ids))
    .load::<JobSRV>(&mut pool)
    .unwrap_or_default();

  HttpResponse::Ok().json(group_jobs)
}
