use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Serialize)]
#[diesel(table_name = crate::db::schema::groups_clients)]
pub struct GroupClients {
  pub group_id: String,
  pub client_id: String,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Queryable, Insertable, Serialize, Deserialize)]
#[diesel(table_name = crate::db::schema::groups_clients)]
pub struct NewGroupClients {
  pub group_id: String,
  pub client_id: String,
  #[serde(rename = "createdAt")]
  pub created_at: chrono::DateTime<chrono::Utc>,
  #[serde(rename = "updatedAt")]
  pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = crate::db::schema::groups_clients)]
pub struct UpdateGroupClients {
  group_id: String,
  client_id: String,
}
