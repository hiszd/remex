use diesel::prelude::*;
use serde::{
  Deserialize,
  Serialize,
};
use utoipa::ToSchema;

use crate::db::{
  model::server as model,
  schema::server as schema,
};

#[derive(
  Debug, Queryable, Associations, Selectable, Serialize, Identifiable, Deserialize, Clone, ToSchema,
)]
#[diesel(belongs_to(model::groups::GroupSRV, foreign_key = group_id))]
#[diesel(belongs_to(model::clients::ClientSRV, foreign_key = client_id))]
#[diesel(table_name = schema::groups_clients)]
pub struct GroupClientsSRV {
  pub id: usize,
  pub group_id: String,
  pub client_id: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = schema::groups_clients)]
pub struct NewGroupClientsSRV {
  pub group_id: String,
  pub client_id: String,
  #[serde(rename = "createdAt")]
  pub created_at: chrono::NaiveDateTime,
  #[serde(rename = "updatedAt")]
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = schema::groups_clients)]
pub struct UpdateGroupClientsSRV {
  group_id: String,
  client_id: String,
}
