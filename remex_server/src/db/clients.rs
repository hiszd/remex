use std::any::Any;

pub struct Client {
  pub id: i64,
  pub client_id: String,
  pub clientname: String,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub async fn generate_id(pool: sqlx::PgPool) -> anyhow::Result<String, anyhow::Error> {
  match sqlx::query!("SELECT generate_unique_client_id()").fetch_one(&pool).await {
    Ok(i) => Ok(i.generate_unique_client_id.unwrap().to_string()),
    Err(e) => {
      tracing::error!("db error: {}", e);
      return Err(anyhow::Error::from(e));
    }
  }
}

pub async fn add_client(
  pool: sqlx::PgPool,
  client_id: String,
  clientname: String,
) -> anyhow::Result<Client> {
  let rec = sqlx::query_as!(
    crate::model::clients::ClientModel,
    r#"
INSERT INTO clients( client_id, clientname )
VALUES ( $1, $2 )
RETURNING *
        "#,
    client_id,
    clientname,
  )
  .fetch_one(&pool)
  .await;
  let r = match rec {
    Ok(rc) => rc,
    Err(e) => {
      anyhow::bail!("db error: {}", e);
    }
  };

  Ok(Client {
    id: r.id,
    client_id: r.client_id,
    clientname: r.clientname,
    created_at: r.created_at.unwrap(),
    updated_at: r.updated_at.unwrap(),
  })
}

pub async fn get_client(pool: &sqlx::PgPool, client_id: String) -> Result<Client, sqlx::Error> {
  match sqlx::query!(r#"SELECT * FROM clients WHERE client_id = $1"#, client_id)
    .fetch_one(pool)
    .await
  {
    Ok(rec) => Ok(Client {
      id: rec.id,
      client_id: rec.client_id,
      clientname: rec.clientname,
      created_at: rec.created_at.unwrap(),
      updated_at: rec.updated_at.unwrap(),
    }),
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
