use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::model::server as model;
use crate::db::schema::server as schema;

#[derive(Identifiable, Queryable, Selectable, Associations)]
#[diesel(belongs_to(model::jobs::Job))]
#[diesel(belongs_to(model::groups::Group))]
#[diesel(table_name = schema::jobs_groups)]
pub struct JobGroup {
  pub id: usize,
  pub job_id: String,
  pub group_id: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = schema::jobs_groups)]
pub struct NewJobGroup {
  pub job_id: String,
  pub group_id: String,
}

#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = schema::jobs_groups)]
pub struct UpdateJobGroup {
  job_id: String,
  group_id: String,
}
