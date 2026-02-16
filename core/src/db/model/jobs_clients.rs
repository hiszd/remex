use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Serialize)]
#[diesel(table_name = crate::db::schema::jobs_clients)]
pub struct JobClients {
  pub job_id: String,
  pub client_id: String,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::jobs_clients)]
pub struct NewJobClients {
  pub job_id: String,
  pub client_id: String,
}

#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = crate::db::schema::jobs_clients)]
pub struct UpdateJobClients {
  pub job_id: String,
  pub client_id: String,
}
