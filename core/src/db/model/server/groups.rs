use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[allow(unused_imports)]
use crate::db::model::server as model;
use crate::db::schema::server as schema;

#[derive(Queryable, Serialize)]
#[diesel(table_name = schema::groups)]
pub struct Group {
  pub id: String,
  pub group_name: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = schema::groups)]
pub struct NewGroup {
  pub id: String,
  pub group_name: String,
}

#[allow(dead_code)]
#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = schema::groups)]
pub struct UpdateGroup {
  id: String,
  group_name: String,
}
