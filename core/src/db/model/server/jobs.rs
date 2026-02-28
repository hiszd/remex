use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::db::model::server as model;
use crate::db::schema::server as schema;

#[derive(Debug, Queryable, Selectable, Serialize, Deserialize, Clone)]
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

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = schema::jobs)]
pub struct NewJob {
  pub id: String,
  pub job_name: String,
  pub job_type: String,
  pub job_status: String,
  pub job_shell: String,
}

#[allow(dead_code)]
#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = schema::jobs)]
pub struct UpdateJob {
  id: String,
  job_name: String,
  job_type: String,
  job_status: String,
  job_shell: String,
}
