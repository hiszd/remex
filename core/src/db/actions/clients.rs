use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper};

use crate::db::{
  establish_connection,
  model::clients::{Client, NewClient, UpdateClient},
  ConnectionType,
};

/* **************************************************************************** */
/* *********************************** Queries ******************************** */
/* **************************************************************************** */

pub async fn get_client(
  ctype: ConnectionType,
  client_id: uuid::Uuid,
) -> Result<Client, diesel::result::Error> {
  use crate::db::schema::clients::dsl::*;
  let mut conn = establish_connection(ctype);
  let qry = clients.filter(id.eq(client_id)).select(Client::as_select()).get_result(&mut conn);
  match qry {
    Ok(rec) => Ok(rec),
    Err(e) => match e {
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

pub async fn add_client(
  pool: Connections,
  client_name: String,
  secret: String,
) -> anyhow::Result<ClientModel> {
  let qry = match pool {
    Connections::Sqlite(p) => {
      sqlx::query_as(
        format!(
          "
INSERT INTO clients( client_name, secret )
VALUES ( {}, {} )
RETURNING *
",
          client_name, secret
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
INSERT INTO clients( client_name, secret )
VALUES ( \'{}\', \'{}\' )
RETURNING *
",
          client_name, secret
        )
        .as_str(),
      )
      .fetch_one(&p)
      .await
    }
  };
  let r: crate::db::model::clients::ClientModel = match qry {
    Ok(rc) => rc,
    Err(e) => {
      anyhow::bail!("db error: {}", e);
    }
  };

  Ok(r)
}
