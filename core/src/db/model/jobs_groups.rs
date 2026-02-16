use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Serialize)]
#[diesel(table_name = crate::db::schema::jobs_groups)]
pub struct JobGroups {
  pub job_id: String,
  pub group_id: String,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::jobs_groups)]
pub struct NewJobGroups {
  pub job_id: String,
  pub group_id: String,
}

#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = crate::db::schema::jobs_groups)]
pub struct UpdateJobGroups {
  job_id: String,
  group_id: String,
}
