use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[allow(unused_imports)]
use crate::db::model::server as model;
use crate::db::schema::server as schema;

#[derive(Debug, Queryable, Selectable, Serialize, Deserialize, Clone, ToSchema)]
#[diesel(table_name = schema::jobs)]
pub struct Job {
  pub id: String,
  pub job_name: String,
  pub job_type: String,
  pub job_status: String,
  pub job_shell: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Queryable, Insertable, Serialize, Deserialize, ToSchema)]
#[diesel(table_name = schema::jobs)]
pub struct NewJob {
  pub id: String,
  pub job_name: String,
  pub job_type: String,
  pub job_status: String,
  pub job_shell: String,
}

#[derive(Deserialize, AsChangeset, ToSchema)]
#[diesel(table_name = schema::jobs)]
pub struct UpdateJob {
  pub id: String,
  pub job_name: String,
  pub job_type: String,
  pub job_status: String,
  pub job_shell: String,
}
