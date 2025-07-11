pub struct Client {
  pub id: String,
  pub name: String,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub async fn generate_id(pool: sqlx::SqlitePool) -> Result<String, anyhow::Error> {
  let qry =
    sqlx::query_as("SELECT id, clientname FROM clients ORDER BY id DESC").fetch_all(&pool).await;
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

pub async fn add_client(
  pool: sqlx::SqlitePool,
  client_id: String,
  clientname: String,
) -> anyhow::Result<Client> {
  let rec = sqlx::query_as(
    format!(
      "
INSERT INTO clients( client_id, clientname )
VALUES ( {}, {} )
RETURNING *
        ",
      client_id, clientname
    )
    .as_str(),
  )
  .fetch_one(&pool)
  .await;
  let r: crate::model::clients::ClientModel = match rec {
    Ok(rc) => rc,
    Err(e) => {
      anyhow::bail!("db error: {}", e);
    }
  };

  Ok(Client {
    id: r.id,
    name: r.name,
    created_at: r.created_at,
    updated_at: r.updated_at,
  })
}

pub async fn get_client(pool: &sqlx::SqlitePool, client_id: String) -> Result<Client, sqlx::Error> {
  let qry =
    sqlx::query_as(format!("SELECT * FROM clients WHERE client_id = {}", client_id).as_str())
      .fetch_one(pool)
      .await;
  match qry {
    Ok(rec) => {
      let r: crate::model::clients::ClientModel = rec;
      Ok(Client {
        id: r.id,
        name: r.name,
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
