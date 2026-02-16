use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Serialize)]
#[diesel(table_name = crate::db::schema::groups)]
pub struct Group {
  pub id: String,
  pub group_name: String,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::groups)]
pub struct NewGroup {
  pub id: String,
  pub group_name: String,
}

#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = crate::db::schema::groups)]
pub struct UpdateGroup {
  id: String,
  group_name: String,
}
