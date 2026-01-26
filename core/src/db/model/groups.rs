use serde::{Deserialize, Serialize};
use sqlx::{types::Uuid, FromRow};

use crate::db::model::clients::{ClientsComplete, ClientsModel};

#[allow(non_snake_case)]
#[derive(Debug, FromRow, Deserialize, Serialize, Clone)]
pub struct GroupsModel {
  pub id: Uuid,
  pub group_name: String,
  #[serde(rename = "createdAt")]
  pub created_at: chrono::DateTime<chrono::Utc>,
  #[serde(rename = "updatedAt")]
  pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[allow(non_snake_case)]
#[derive(Debug, sqlx::FromRow, Deserialize, Serialize, Clone)]
pub struct GroupsRow {
  pub id: sqlx::types::Uuid,
  pub group_name: String,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub updated_at: chrono::DateTime<chrono::Utc>,
  pub clients_json: sqlx::types::JsonValue,
}
impl GroupsRow {
  #[allow(dead_code)]
  pub fn split_group_clients(&self) -> (GroupsModel, Vec<ClientsModel>) {
    let c: Vec<ClientsModel> = serde_json::from_value(self.clients_json.clone()).unwrap();
    (
      GroupsModel {
        id: self.id,
        group_name: self.group_name.clone(),
        created_at: self.created_at,
        updated_at: self.updated_at,
      },
      c,
    )
  }
}

#[allow(non_snake_case)]
#[derive(Debug, sqlx::FromRow, Deserialize, Serialize, Clone)]
pub struct GroupsComplete {
  pub id: sqlx::types::Uuid,
  pub group_name: String,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub updated_at: chrono::DateTime<chrono::Utc>,
  pub clients: Vec<ClientsComplete>,
}

impl Into<GroupsComplete> for GroupsRow {
  fn into(self) -> GroupsComplete {
    let c: Vec<ClientsModel> = serde_json::from_value(self.clients_json.clone()).unwrap();
    GroupsComplete {
      id: self.id,
      group_name: self.group_name.clone(),
      created_at: self.created_at,
      updated_at: self.updated_at,
      clients: c,
    }
  }
}
