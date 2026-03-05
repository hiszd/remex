use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::db::model::endpoint as model;
use crate::db::schema::endpoint as schema;

#[derive(Debug, Queryable, Identifiable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = schema::jobs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct JobCLT {
  pub id: String,
  pub job_name: String,
  pub job_type: String,
  pub job_status: String,
  pub job_status_message: Option<String>,
  pub job_shell: String,
  pub job_command: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = schema::jobs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewJobCLT {
  pub id: String,
  pub job_name: String,
  pub job_type: String,
  pub job_status: String,
  pub job_status_message: Option<String>,
  pub job_shell: String,
  pub job_command: String,
}

#[allow(dead_code)]
#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = schema::jobs)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct UpdateJobCLT {
  pub id: String,
  pub job_name: String,
  pub job_type: String,
  pub job_status: String,
  pub job_status_message: Option<String>,
  pub job_shell: String,
  pub job_command: String,
}
