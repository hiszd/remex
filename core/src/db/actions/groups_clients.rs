use crate::db::{model::groups_clients::GroupsClientsModel, Connections};

/* **************************************************************************** */
/* *********************************** Queries ******************************** */
/* **************************************************************************** */

pub enum GroupsClientsSelector {
  GroupId(String),
  ClientId(String),
}

pub async fn get_groups_clients(
  pool: Connections,
  id: GroupsClientsSelector,
) -> Result<GroupsClientsModel, sqlx::Error> {
  let (sel, id) = match id {
    GroupsClientsSelector::GroupId(id) => ("group_id", id),
    GroupsClientsSelector::ClientId(id) => ("client_id", id),
  };
  let qry = match pool {
    Connections::Sqlite(p) => {
      sqlx::query_as(format!("SELECT * FROM groups_clients WHERE {} = {}", sel, id).as_str())
        .fetch_one(&p)
        .await
    }
    Connections::Postgres(p) => {
      sqlx::query_as(format!("SELECT * FROM groups WHERE {} = \'{}\'", sel, id).as_str())
        .fetch_one(&p)
        .await
    }
  };
  match qry {
    Ok(rec) => {
      let r: GroupsClientsModel = rec;
      Ok(r)
    }
    Err(e) => match e {
      sqlx::Error::RowNotFound => {
        tracing::error!("group not found");
        Err(e)
      }
      _ => {
        tracing::error!("db error: {}", e);
        Err(e)
      }
    },
  }
}

/* **************************************************************************** */
/* ********************************** Commands ******************************** */
/* **************************************************************************** */

pub async fn add_groups_clients(
  pool: Connections,
  group_id: String,
  client_id: String,
) -> anyhow::Result<GroupsClientsModel> {
  let qry = match pool {
    Connections::Sqlite(p) => {
      sqlx::query_as(
        format!(
          "
INSERT INTO groups_clients( group_id, client_id )
VALUES ( {}, {} )
RETURNING *
",
          group_id, client_id
        )
        .as_str(),
      )
      .fetch_one(&p)
      .await
    }
    Connections::Postgres(p) => {
      sqlx::query_as(
        format!(
          "
INSERT INTO groups_clients( group_id, client_id )
VALUES ( \'{}\', \'{}\' )
RETURNING *
",
          group_id, client_id
        )
        .as_str(),
      )
      .fetch_one(&p)
      .await
    }
  };
  let r: GroupsClientsModel = match qry {
    Ok(rc) => rc,
    Err(e) => {
      anyhow::bail!("db error: {}", e);
    }
  };

  Ok(r)
}
