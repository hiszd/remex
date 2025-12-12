use crate::db::clients::{Client, Pools};

/* **************************************************************************** */
/* *********************************** Queries ******************************** */
/* **************************************************************************** */

pub async fn generate_id(pool: Pools) -> Result<String, anyhow::Error> {
  let qry = match pool {
    Pools::Sqlite(p) => {
      sqlx::query_as("SELECT id, client_name FROM clients ORDER BY id DESC").fetch_all(&p).await
    }
    Pools::Postgres(p) => {
      sqlx::query_as("SELECT id, client_name FROM clients ORDER BY id DESC").fetch_all(&p).await
    }
  };
  match qry {
    Ok(rec) => {
      let r: Vec<(String, String)> = rec;
      let id = uuid::Uuid::new_v4().to_string();
      match r.iter().find(|c| c.0 == id) {
        Some(_) => Err(anyhow::anyhow!("id already exists")),
        None => Ok(id),
      }
    }
    Err(e) => {
      tracing::error!("db error: {}", e);
      Err(anyhow::anyhow!(e))
    }
  }
}

pub async fn get_client(pool: Pools, client_id: String) -> Result<Client, sqlx::Error> {
  let qry = match pool {
    Pools::Sqlite(p) => {
      sqlx::query_as(format!("SELECT * FROM clients WHERE id = {}", client_id).as_str())
        .fetch_one(&p)
        .await
    }
    Pools::Postgres(p) => {
      sqlx::query_as(format!("SELECT * FROM clients WHERE id = \'{}\'", client_id).as_str())
        .fetch_one(&p)
        .await
    }
  };
  match qry {
    Ok(rec) => {
      let r: crate::db::model::clients::ClientModel = rec;
      Ok(Client {
        id: r.id,
        name: r.client_name,
        created_at: r.created_at,
        updated_at: r.updated_at,
      })
    }
    Err(e) => match e {
      sqlx::Error::RowNotFound => {
        tracing::error!("client not found");
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

pub async fn add_client(
  pool: Pools,
  client_id: String,
  client_name: String,
) -> anyhow::Result<Client> {
  let qry = match pool {
    Pools::Sqlite(p) => {
      sqlx::query_as(
        format!(
          "
INSERT INTO clients( id, client_name )
VALUES ( {}, {} )
RETURNING *
",
          client_id, client_name
        )
        .as_str(),
      )
      .fetch_one(&p)
      .await
    }
    Pools::Postgres(p) => {
      sqlx::query_as(
        format!(
          "
INSERT INTO clients( id, client_name )
VALUES ( \'{}\', \'{}\' )
RETURNING *
",
          client_id, client_name
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

  Ok(Client {
    id: r.id,
    name: r.client_name,
    created_at: r.created_at,
    updated_at: r.updated_at,
  })
}
