use crate::db::{
  actions::groups::requests::{get_group_clients, upsert_group},
  model::{clients::ClientsModel, groups::GroupsModel},
  Pools,
};

#[allow(dead_code)]
#[derive(thiserror::Error, Debug)]
pub enum GroupContError {
  #[error("Group not found")]
  GroupNotFound,
  #[error("Use of the Client pool is not allowed for this purpose")]
  ClientPoolNotAllowed,
  #[error("Sqlx Error: {0}")]
  SqlxError(String),
}

#[derive(Debug, Clone)]
pub struct GroupCont {
  pub group: GroupsModel,
  pub clients: Vec<ClientsModel>,
  updated_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
}

impl GroupCont {
  pub async fn new(pool: &sqlx::Pool<sqlx::Postgres>, group_name: String) -> GroupCont {
    let group = upsert_group(pool, &group_name).await.unwrap();
    let id = group.id;
    GroupCont {
      group,
      clients: get_group_clients(pool, id).await.unwrap().1,
      updated_at: sqlx::types::chrono::Utc::now(),
    }
  }

  pub async fn update(&mut self, pool: Pools) -> Result<(), GroupContError> {
    match pool {
      Pools::Postgres(p) => {
        if self.updated_at < sqlx::types::chrono::Utc::now() {
          let qry = get_group_clients(&p, self.group.id).await;
          return match qry {
            Ok(rec) => {
              let g = rec.0;
              let clnts = rec.1;
              // NOTE: if rec has been updated more recently than this record
              if self.updated_at < g.updated_at {
                self.updated_at = chrono::Utc::now();
                self.group = g;
                self.clients = clnts;
              }
              Ok(())
            }
            Err(e) => {
              self.updated_at = chrono::Utc::now();
              match e {
                sqlx::Error::RowNotFound => {
                  tracing::error!("group not found");
                  Err(GroupContError::GroupNotFound)
                }
                _ => {
                  tracing::error!("db error: {}", e);
                  Err(GroupContError::SqlxError(e.to_string()))
                }
              }
            }
          };
        }
        Ok(())
      }
      _ => Err(GroupContError::ClientPoolNotAllowed),
    }
  }

  pub async fn add_client(&mut self, pool: &sqlx::Pool<sqlx::Postgres>, client: ClientsModel) {
    match crate::db::actions::groups::commands::add_client(
      pool,
      self.group.id,
      sqlx::types::Uuid::parse_str(&client.id.to_string()).unwrap(),
    )
    .await
    {
      Ok(_) => {
        self.updated_at = chrono::Utc::now();
        self.clients.push(client);
      }
      Err(e) => {
        tracing::error!("db error: {}", e);
      }
    }
  }

  pub async fn remove_client(&mut self, pool: &sqlx::Pool<sqlx::Postgres>, client: ClientsModel) {
    match crate::db::actions::groups::commands::remove_client(pool, self.group.id, client.id).await
    {
      Ok(_) => {
        self.updated_at = chrono::Utc::now();
        self.clients.retain(|c| c.id != client.id);
      }
      Err(e) => {
        tracing::error!("db error: {}", e);
      }
    }
  }
}
