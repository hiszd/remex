use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::model::server as model;
use crate::db::schema::server as schema;

#[derive(Identifiable, Queryable, Selectable, Associations)]
#[diesel(belongs_to(model::jobs::JobSRV, foreign_key = job_id))]
#[diesel(belongs_to(model::groups::GroupSRV, foreign_key = group_id))]
#[diesel(table_name = schema::jobs_groups)]
pub struct JobGroupSRV {
  pub id: usize,
  pub job_id: String,
  pub group_id: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = schema::jobs_groups)]
pub struct NewJobGroupSRV {
  pub job_id: String,
  pub group_id: String,
}

#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = schema::jobs_groups)]
pub struct UpdateJobGroupSRV {
  job_id: String,
  group_id: String,
}
