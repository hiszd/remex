use diesel::prelude::*;
use serde::{
  Deserialize,
  Serialize,
};
use utoipa::ToSchema;

#[allow(unused_imports)]
use crate::db::{
  dal::groups::Group,
  model::server as model,
  schema::server as schema,
};

#[derive(Queryable, Selectable, Serialize, Identifiable, ToSchema, Clone)]
#[diesel(table_name = schema::groups)]
pub struct GroupSRV {
  pub id: String,
  pub group_name: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Queryable, Insertable, Serialize, Deserialize, ToSchema, Clone)]
#[diesel(table_name = schema::groups)]
pub struct NewGroupSRV {
  pub id: String,
  pub group_name: String,
  pub created_at: Option<chrono::NaiveDateTime>,
  pub updated_at: Option<chrono::NaiveDateTime>,
}

impl From<GroupSRV> for NewGroupSRV {
  fn from(group: GroupSRV) -> Self {
    NewGroupSRV {
      id: group.id,
      group_name: group.group_name,
      created_at: Some(group.created_at),
      updated_at: Some(group.updated_at),
    }
  }
}
impl From<Group> for NewGroupSRV {
  fn from(group: Group) -> Self {
    NewGroupSRV {
      id: group.id,
      group_name: group.group_name,
      created_at: Some(group.created_at),
      updated_at: Some(group.updated_at),
    }
  }
}

#[allow(dead_code)]
#[derive(Deserialize, AsChangeset, Clone)]
#[diesel(table_name = schema::groups)]
pub struct UpdateGroupSRV {
  id: String,
  group_name: String,
}

impl From<Group> for UpdateGroupSRV {
  fn from(group: Group) -> Self {
    UpdateGroupSRV {
      id: group.id,
      group_name: group.group_name,
    }
  }
}
impl From<GroupSRV> for UpdateGroupSRV {
  fn from(group: GroupSRV) -> Self {
    UpdateGroupSRV {
      id: group.id,
      group_name: group.group_name,
    }
  }
}

#[allow(dead_code)]
#[derive(Deserialize, AsChangeset, Clone)]
#[diesel(table_name = schema::groups)]
#[diesel(check_for_backend(diesel::postgres::Pg))]
pub struct UpsertGroupSRV {
  pub id: Option<String>,
  pub group_name: Option<String>,
  pub created_at: Option<chrono::NaiveDateTime>,
  pub updated_at: Option<chrono::NaiveDateTime>,
}

impl From<Group> for UpsertGroupSRV {
  fn from(group: Group) -> Self {
    UpsertGroupSRV {
      id: Some(group.id),
      group_name: Some(group.group_name),
      created_at: Some(group.created_at),
      updated_at: Some(group.updated_at),
    }
  }
}

impl From<GroupSRV> for UpsertGroupSRV {
  fn from(group: GroupSRV) -> Self {
    UpsertGroupSRV {
      id: Some(group.id),
      group_name: Some(group.group_name),
      created_at: Some(group.created_at),
      updated_at: Some(group.updated_at),
    }
  }
}
