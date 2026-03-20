use diesel::prelude::*;
use serde::{
  Deserialize,
  Serialize,
};
use utoipa::ToSchema;

#[allow(unused_imports)]
use crate::db::model::server as model;
use crate::db::{
  dal::clients::Client,
  schema::server as schema,
};

#[derive(Queryable, Selectable, Identifiable, Serialize, Clone, ToSchema)]
#[diesel(table_name = schema::clients)]
pub struct ClientSRV {
  pub id: String,
  pub secret: String,
  pub client_name: String,
  pub hardware_hash: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

impl From<Client> for ClientSRV {
  fn from(client: Client) -> Self {
    ClientSRV {
      id: client.id,
      secret: client.secret,
      client_name: client.client_name,
      hardware_hash: client.hardware_hash,
      created_at: client.created_at,
      updated_at: client.updated_at,
    }
  }
}

#[derive(Queryable, Insertable, Serialize, Deserialize, ToSchema)]
#[diesel(table_name = schema::clients)]
pub struct NewClientSRV {
  pub id: String,
  pub secret: String,
  pub client_name: String,
  pub hardware_hash: String,
  pub created_at: Option<chrono::NaiveDateTime>,
  pub updated_at: Option<chrono::NaiveDateTime>,
}

impl From<ClientSRV> for NewClientSRV {
  fn from(client: ClientSRV) -> Self {
    NewClientSRV {
      id: client.id,
      secret: client.secret,
      client_name: client.client_name,
      hardware_hash: client.hardware_hash,
      created_at: Some(client.created_at),
      updated_at: Some(client.updated_at),
    }
  }
}

impl From<Client> for NewClientSRV {
  fn from(client: Client) -> Self {
    NewClientSRV {
      id: client.id,
      secret: client.secret,
      client_name: client.client_name,
      hardware_hash: client.hardware_hash,
      created_at: Some(client.created_at),
      updated_at: Some(client.updated_at),
    }
  }
}

#[derive(Deserialize, AsChangeset)]
#[diesel(table_name = schema::clients)]
pub struct UpdateClientSRV {
  secret: Option<String>,
  client_name: Option<String>,
  hardware_hash: Option<String>,
}

impl From<ClientSRV> for UpdateClientSRV {
  fn from(client: ClientSRV) -> Self {
    UpdateClientSRV {
      secret: Some(client.secret),
      client_name: Some(client.client_name),
      hardware_hash: Some(client.hardware_hash),
    }
  }
}

impl From<Client> for UpdateClientSRV {
  fn from(client: Client) -> Self {
    UpdateClientSRV {
      secret: Some(client.secret),
      client_name: Some(client.client_name),
      hardware_hash: Some(client.hardware_hash),
    }
  }
}

#[allow(dead_code)]
#[derive(Deserialize, AsChangeset, Clone)]
#[diesel(table_name = schema::clients)]
#[diesel(check_for_backend(diesel::postgres::Pg))]
pub struct UpsertClientSRV {
  pub id: Option<String>,
  pub secret: Option<String>,
  pub client_name: Option<String>,
  pub hardware_hash: Option<String>,
  pub created_at: Option<chrono::NaiveDateTime>,
  pub updated_at: Option<chrono::NaiveDateTime>,
}

impl From<ClientSRV> for UpsertClientSRV {
  fn from(client: ClientSRV) -> Self {
    UpsertClientSRV {
      id: Some(client.id),
      secret: Some(client.secret),
      client_name: Some(client.client_name),
      hardware_hash: Some(client.hardware_hash),
      created_at: Some(client.created_at),
      updated_at: Some(client.updated_at),
    }
  }
}
impl From<Client> for UpsertClientSRV {
  fn from(client: Client) -> Self {
    UpsertClientSRV {
      id: Some(client.id),
      secret: Some(client.secret),
      client_name: Some(client.client_name),
      hardware_hash: Some(client.hardware_hash),
      created_at: Some(client.created_at),
      updated_at: Some(client.updated_at),
    }
  }
}
