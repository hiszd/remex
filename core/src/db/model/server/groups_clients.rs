use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::db::model::server as model;
use crate::db::schema::server as schema;

#[derive(Identifiable, Queryable, Selectable, Associations)]
#[diesel(belongs_to(model::groups::Group))]
#[diesel(belongs_to(model::clients::Client))]
#[diesel(table_name = schema::groups_clients)]
pub struct GroupClients {
  pub id: usize,
  pub group_id: String,
  pub client_id: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = schema::groups_clients)]
pub struct NewGroupClients {
  pub group_id: String,
  pub client_id: String,
  #[serde(rename = "createdAt")]
  pub created_at: chrono::NaiveDateTime,
  #[serde(rename = "updatedAt")]
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = schema::groups_clients)]
pub struct UpdateGroupClients {
  group_id: String,
  client_id: String,
}
