use serde::{
  Deserialize,
  Serialize,
};

use crate::db::model;

#[derive(Serialize, Deserialize)]
pub struct Group {
  pub id: String,
  pub group_name: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

impl From<model::server::groups::GroupSRV> for Group {
  fn from(group: model::server::groups::GroupSRV) -> Self {
    Group {
      id: group.id,
      group_name: group.group_name,
      created_at: group.created_at,
      updated_at: group.updated_at,
    }
  }
}
