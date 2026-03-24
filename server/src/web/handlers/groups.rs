use actix_web::{
  get,
  post,
  web,
  HttpResponse,
  Responder,
};
use diesel::prelude::*;
use remex_core::db::model::server::{
  clients::ClientSRV,
  groups::GroupSRV,
  jobs::JobSRV,
};
use serde::{
  Deserialize,
  Serialize,
};
use utoipa::ToSchema;

use crate::web::handlers::jobs::data_gathering::{
  get_group_job_status,
  ClientStatusSummary,
};

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
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
  pub client_count: i64,
}

impl From<(GroupSRV, i64)> for Group {
  fn from((group, client_count): (GroupSRV, i64)) -> Self {
    Group {
      id: group.id,
      group_name: group.group_name,
      created_at: group.created_at,
      updated_at: group.updated_at,
      client_count,
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
  use remex_core::db::schema::server::{
    groups,
    groups_clients,
  };
  let mut pool = remex_core::db::establish_connection_postgres();

  let all_groups: Vec<GroupSRV> = groups::table
    .load::<GroupSRV>(&mut pool)
    .unwrap();

  let client_counts: Vec<(Option<String>, i64)> = groups_clients::table
    .group_by(groups_clients::group_id)
    .select((groups_clients::group_id, diesel::dsl::count(groups_clients::client_id)))
    .load::<(Option<String>, i64)>(&mut pool)
    .unwrap_or_default();

  let count_map: std::collections::HashMap<String, i64> = client_counts
    .into_iter()
    .filter_map(|(group_id, count)| group_id.map(|id| (id, count)))
    .collect();

  let groups_with_counts: Vec<Group> = all_groups
    .into_iter()
    .map(|g| {
      let count = count_map.get(&g.id).copied().unwrap_or(0);
      (g, count).into()
    })
    .collect();

  HttpResponse::Ok().json(groups_with_counts)
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
    model::server::{
      groups::NewGroupSRV,
      groups_clients::NewGroupClientsSRV,
    },
    schema::server::{
      groups,
      groups_clients,
    },
  };
  let mut pool = remex_core::db::establish_connection_postgres();

  let group_id = uuid::Uuid::now_v7().to_string();

  let new_group = NewGroupSRV {
    id: group_id.clone(),
    group_name: form.group_name.clone(),
    created_at: None,
    updated_at: None,
  };

  let group = diesel::insert_into(groups::table)
    .values(&new_group)
    .get_result::<GroupSRV>(&mut pool)
    .unwrap();

  let mut client_count = 0i64;

  if let Some(ref client_ids) = form.client_ids {
    if !client_ids.is_empty() {
      let client_associations: Vec<NewGroupClientsSRV> = client_ids
        .iter()
        .map(|client_id| NewGroupClientsSRV {
          group_id: group_id.clone(),
          client_id: client_id.clone(),
        })
        .collect();

      diesel::insert_into(groups_clients::table)
        .values(&client_associations)
        .execute(&mut pool)
        .unwrap();
      
      client_count = client_ids.len() as i64;
    }
  }

  HttpResponse::Created().json(Group::from((group, client_count)))
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
  use remex_core::db::schema::server::{
    groups,
    groups_clients,
  };
  let mut pool = remex_core::db::establish_connection_postgres();

  let group_id = path.group_id.clone();

  let group = groups::table
    .filter(groups::id.eq(&group_id))
    .first::<GroupSRV>(&mut pool)
    .optional()
    .unwrap();

  match group {
    Some(g) => {
      let client_count: i64 = groups_clients::table
        .filter(groups_clients::group_id.eq(&group_id))
        .count()
        .get_result::<i64>(&mut pool)
        .unwrap_or(0);
      HttpResponse::Ok().json(Group::from((g, client_count)))
    }
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
pub async fn add_clients_to_group(path: web::Path<GroupPath>, form: web::Json<AddClientsToGroupForm>) -> impl Responder {
  use remex_core::db::{
    model::server::groups_clients::NewGroupClientsSRV,
    schema::server::groups_clients,
  };
  let mut pool = remex_core::db::establish_connection_postgres();

  let group_id = path.group_id.clone();

  if form.client_ids.is_empty() {
    return HttpResponse::BadRequest().json("No client IDs provided");
  }

  let client_associations: Vec<NewGroupClientsSRV> = form.client_ids
    .iter()
    .map(|client_id| NewGroupClientsSRV {
      group_id: group_id.clone(),
      client_id: client_id.clone(),
    })
    .collect();

  match diesel::insert_into(groups_clients::table)
    .values(&client_associations)
    .execute(&mut pool)
  {
    Ok(_) => HttpResponse::Ok().json("Clients added to group successfully"),
    Err(e) => {
      tracing::error!("Failed to add clients to group: {}", e);
      HttpResponse::InternalServerError().json("Failed to add clients to group")
    }
  }
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
pub async fn remove_clients_from_group(path: web::Path<GroupPath>, form: web::Json<RemoveClientsFromGroupForm>) -> impl Responder {
  use remex_core::db::schema::server::groups_clients;
  let mut pool = remex_core::db::establish_connection_postgres();

  let group_id = path.group_id.clone();

  if form.client_ids.is_empty() {
    return HttpResponse::BadRequest().json("No client IDs provided");
  }

  match diesel::delete(groups_clients::table)
    .filter(groups_clients::group_id.eq(&group_id))
    .filter(groups_clients::client_id.eq_any(&form.client_ids))
    .execute(&mut pool)
  {
    Ok(_) => HttpResponse::Ok().json("Clients removed from group successfully"),
    Err(e) => {
      tracing::error!("Failed to remove clients from group: {}", e);
      HttpResponse::InternalServerError().json("Failed to remove clients from group")
    }
  }
}
