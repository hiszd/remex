use actix_web::{
  get,
  post,
  web,
  HttpResponse,
  Responder,
};
use diesel::prelude::*;
use remex_core::db::{
  dal::{
    job_status::ClientJobStatusMetadata,
    jobs::Job,
    SrvDbOperator,
  },
  model::server::{
    clients::ClientSRV,
    groups::GroupSRV,
    jobs::{
      JobSRV,
      NewJobSRV,
      UpdateJobSRV,
    },
  },
};
use serde::{
  Deserialize,
  Serialize,
};
use utoipa::ToSchema;

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
pub struct JobWithClients {
  #[serde(flatten)]
  pub job: JobSRV,
  pub clients: Vec<ClientSRV>,
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
  use remex_core::db::schema::server::{
    clients,
    groups_clients,
    jobs,
    jobs_groups,
  };
  let mut pool = remex_core::db::establish_connection_postgres();

  let all_jobs = jobs::table.load::<JobSRV>(&mut pool).unwrap();

  let mut results = Vec::new();
  for job in all_jobs {
    let job_clients = clients::table
      .inner_join(groups_clients::table.on(groups_clients::client_id.eq(clients::id.nullable())))
      .inner_join(jobs_groups::table.on(jobs_groups::group_id.eq(groups_clients::group_id)))
      .filter(jobs_groups::job_id.eq(&job.id))
      .select(clients::all_columns)
      .distinct()
      .load::<ClientSRV>(&mut pool)
      .unwrap();

    results.push(JobWithClients {
      job,
      clients: job_clients,
    });
  }

  HttpResponse::Ok().json(results)
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
  use remex_core::db::schema::server::{
    clients,
    groups_clients,
    jobs,
    jobs_groups,
  };
  let mut pool = remex_core::db::establish_connection_postgres();

  let job = jobs::table
    .filter(jobs::id.eq(id.into_inner()))
    .first::<JobSRV>(&mut pool)
    .optional()
    .unwrap();

  if let Some(job) = job {
    let job_clients = clients::table
      .inner_join(groups_clients::table.on(groups_clients::client_id.eq(clients::id.nullable())))
      .inner_join(jobs_groups::table.on(jobs_groups::group_id.eq(groups_clients::group_id)))
      .filter(jobs_groups::job_id.eq(&job.id))
      .select(clients::all_columns)
      .distinct()
      .load::<ClientSRV>(&mut pool)
      .unwrap();

    HttpResponse::Ok().json(JobWithClients {
      job,
      clients: job_clients,
    })
  } else {
    HttpResponse::NotFound().finish()
  }
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
  use remex_core::db::{
    model::server::jobs_groups::NewJobGroupSRV,
    schema::server::jobs_groups,
  };
  let mut pool = remex_core::db::establish_connection_postgres();

  let new_job_id = uuid::Uuid::now_v7().to_string();
  let new_job: Job = Job::new(
    new_job_id.clone(),
    form.job_name.clone(),
    form.job_type.clone(),
    form.job_status.clone().into(),
    form.job_shell.clone(),
    form.job_command.clone(),
  );

  new_job.create_srv(&mut pool).unwrap();

  if let Some(group_ids) = &form.group_ids {
    for gid in group_ids {
      let new_job_group = NewJobGroupSRV {
        job_id: new_job_id.clone(),
        group_id: gid.clone(),
      };
      diesel::insert_into(jobs_groups::table)
        .values(&new_job_group)
        .execute(&mut pool)
        .unwrap();
    }
  }

  HttpResponse::Created().json(new_job)
}

#[utoipa::path(
  post,
  path = "/jobs/update",
  request_body = UpdateJobSRV,
  responses(
    (status = 200, description = "Job updated successfully", body = JobSRV),
    (status = 404, description = "Job not found"),
  ),
)]
#[post("/jobs/update")]
pub async fn update_job(job_update: web::Json<UpdateJobSRV>) -> impl Responder {
  use remex_core::db::schema::server::jobs;
  let mut pool = remex_core::db::establish_connection_postgres();

  let job_id = job_update.id.clone();
  let updated_job = diesel::update(jobs::table.filter(jobs::id.eq(job_id)))
    .set(job_update.into_inner())
    .get_result::<JobSRV>(&mut pool)
    .optional()
    .unwrap();

  if let Some(job) = updated_job {
    HttpResponse::Ok().json(job)
  } else {
    HttpResponse::NotFound().finish()
  }
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
  path: web::Path<JobGroupPath>,
  form: web::Json<UpdateJobGroupsForm>
) -> impl Responder {
  use remex_core::db::{
    model::server::jobs_groups::NewJobGroupSRV,
    schema::server::{
      jobs,
      jobs_groups,
    },
  };
  let mut pool = remex_core::db::establish_connection_postgres();

  let job_id = path.job_id.clone();

  let job_exists = jobs::table
    .filter(jobs::id.eq(&job_id))
    .select(jobs::id)
    .first::<String>(&mut pool)
    .optional()
    .unwrap()
    .is_some();

  if !job_exists {
    return HttpResponse::NotFound().finish();
  }

  diesel::delete(jobs_groups::table.filter(jobs_groups::job_id.eq(&job_id)))
    .execute(&mut pool)
    .unwrap();

  for group_id in &form.group_ids {
    let new_job_group = NewJobGroupSRV {
      job_id: job_id.clone(),
      group_id: group_id.clone(),
    };
    diesel::insert_into(jobs_groups::table)
      .values(&new_job_group)
      .execute(&mut pool)
      .unwrap();
  }

  HttpResponse::Ok().finish()
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
pub async fn add_clients_to_jobs(actions: web::Json<Vec<JobClientAction>>) -> impl Responder {
  use remex_core::db::{
    model::server::{
      groups::NewGroupSRV,
      groups_clients::NewGroupClientsSRV,
      jobs_groups::NewJobGroupSRV,
    },
    schema::server::{
      groups,
      groups_clients,
      jobs,
      jobs_groups,
    },
  };
  let mut pool = remex_core::db::establish_connection_postgres();

  for action in actions.into_inner() {
    // 1. Verify job exists
    let job_exists = jobs::table
      .filter(jobs::id.eq(&action.job_id))
      .select(jobs::id)
      .first::<String>(&mut pool)
      .optional()
      .unwrap()
      .is_some();

    if !job_exists {
      continue;
    }

    // 2. Find or create a group for this association
    let existing_group_id = jobs_groups::table
      .filter(jobs_groups::job_id.eq(&action.job_id))
      .select(jobs_groups::group_id)
      .first::<Option<String>>(&mut pool)
      .optional()
      .unwrap()
      .flatten();

    let group_id = if let Some(gid) = existing_group_id {
      gid
    } else {
      let new_group_id = uuid::Uuid::new_v4().to_string();
      let new_group = NewGroupSRV {
        id: new_group_id.clone(),
        group_name: format!("Group for Job {}", action.job_id),
      };

      diesel::insert_into(groups::table)
        .values(&new_group)
        .execute(&mut pool)
        .unwrap();

      let new_job_group = NewJobGroupSRV {
        job_id: action.job_id.clone(),
        group_id: new_group_id.clone(),
      };

      diesel::insert_into(jobs_groups::table)
        .values(&new_job_group)
        .execute(&mut pool)
        .unwrap();

      new_group_id
    };

    // 3. Add client to the group
    let new_assoc = NewGroupClientsSRV {
      group_id: group_id.clone(),
      client_id: action.client_id.clone(),
      created_at: chrono::Utc::now().naive_utc(),
      updated_at: chrono::Utc::now().naive_utc(),
    };

    diesel::insert_into(groups_clients::table)
      .values(&new_assoc)
      .execute(&mut pool)
      .unwrap();
  }

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
pub async fn remove_clients_from_jobs(actions: web::Json<Vec<JobClientAction>>) -> impl Responder {
  use remex_core::db::schema::server::{
    groups_clients,
    jobs_groups,
  };
  let mut pool = remex_core::db::establish_connection_postgres();

  for action in actions.into_inner() {
    let group_ids = jobs_groups::table
      .filter(jobs_groups::job_id.eq(&action.job_id))
      .select(jobs_groups::group_id)
      .load::<Option<String>>(&mut pool)
      .unwrap()
      .into_iter()
      .flatten()
      .collect::<Vec<String>>();

    diesel::delete(
      groups_clients::table
        .filter(groups_clients::group_id.nullable().eq_any(group_ids))
        .filter(groups_clients::client_id.nullable().eq(&action.client_id)),
    )
    .execute(&mut pool)
    .unwrap();
  }

  HttpResponse::Ok().finish()
}

#[derive(Serialize, ToSchema)]
pub struct ClientJobStatusResponse {
  pub client_id: String,
  pub client_name: String,
  pub status: String,
  pub latest_execution_id: Option<String>,
  pub latest_execution_timestamp: Option<chrono::NaiveDateTime>,
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
  use remex_core::db::dal::job_status::get_client_job_status;
  let mut pool = remex_core::db::establish_connection_postgres();

  let job_id = id.into_inner();

  let job_clients = remex_core::db::schema::server::clients::table
    .inner_join(
      remex_core::db::schema::server::groups_clients::table.on(
        remex_core::db::schema::server::groups_clients::client_id
          .eq(remex_core::db::schema::server::clients::id.nullable()),
      ),
    )
    .inner_join(
      remex_core::db::schema::server::jobs_groups::table.on(
        remex_core::db::schema::server::jobs_groups::group_id
          .eq(remex_core::db::schema::server::groups_clients::group_id),
      ),
    )
    .filter(remex_core::db::schema::server::jobs_groups::job_id.eq(&job_id))
    .select((
      remex_core::db::schema::server::clients::id,
      remex_core::db::schema::server::clients::client_name,
    ))
    .distinct()
    .load::<(String, String)>(&mut pool)
    .unwrap_or_default();

  let mut response = Vec::new();
  for (client_id, client_name) in job_clients {
    let (status, metadata) = get_client_job_status(&mut pool, &client_id, &job_id).unwrap_or((
      "pending".to_string(),
      ClientJobStatusMetadata {
        latest_execution_id: None,
        latest_execution_timestamp: None,
        execution_count: 0,
      },
    ));

    response.push(ClientJobStatusResponse {
      client_id,
      client_name,
      status,
      latest_execution_id: metadata.latest_execution_id,
      latest_execution_timestamp: metadata.latest_execution_timestamp,
      execution_count: metadata.execution_count,
    });
  }

  HttpResponse::Ok().json(response)
}

#[derive(Serialize, ToSchema)]
pub struct JobWithGroups {
  pub id: String,
  pub group_name: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
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
  use remex_core::db::schema::server::jobs_groups;
  let mut pool = remex_core::db::establish_connection_postgres();

  let job_id = id.into_inner();

  let group_ids: Vec<String> = jobs_groups::table
    .filter(jobs_groups::job_id.eq(&job_id))
    .select(jobs_groups::group_id)
    .load::<Option<String>>(&mut pool)
    .unwrap()
    .into_iter()
    .flatten()
    .collect();

  let groups: Vec<remex_core::db::model::server::groups::GroupSRV> =
    remex_core::db::schema::server::groups::table
      .filter(
        remex_core::db::schema::server::groups::id
          .nullable()
          .eq_any(group_ids),
      )
      .load::<remex_core::db::model::server::groups::GroupSRV>(&mut pool)
      .unwrap_or_default();

  let job_groups: Vec<JobWithGroups> = groups
    .into_iter()
    .map(|g| JobWithGroups {
      id: g.id,
      group_name: g.group_name,
      created_at: g.created_at,
      updated_at: g.updated_at,
    })
    .collect();

  HttpResponse::Ok().json(job_groups)
}
