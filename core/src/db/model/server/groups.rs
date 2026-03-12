use diesel::prelude::*;
use serde::{
  Deserialize,
  Serialize,
};
use utoipa::ToSchema;

#[allow(unused_imports)]
use crate::db::model::server as model;
use crate::db::schema::server as schema;

#[derive(Queryable, Selectable, Serialize, Identifiable, ToSchema)]
#[diesel(table_name = schema::groups)]
pub struct GroupSRV {
  pub id: String,
  pub group_name: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = schema::groups)]
pub struct NewGroupSRV {
  pub id: String,
  pub group_name: String,
}

#[allow(dead_code)]
#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = schema::groups)]
pub struct UpdateGroupSRV {
  id: String,
  group_name: String,
}
