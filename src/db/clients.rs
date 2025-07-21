pub struct Client {
  pub id: String,
  pub name: String,
  pub secret: String,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub async fn generate_id(pool: sqlx::SqlitePool) -> Result<String, anyhow::Error> {
  let qry = sqlx::query_as("SELECT id, name FROM clients ORDER BY id DESC").fetch_all(&pool).await;
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
      tracing::error!("clients 21 - db error: {}", e);
      Err(anyhow::anyhow!(e))
    }
  }
}

pub fn generate_secret() -> String {
  let mut secret: [u8; 32] = [42; 32];
  openssl::rand::rand_bytes(&mut secret).unwrap();
  openssl::base64::encode_block(&secret)
}

pub async fn add_client(
  pool: sqlx::SqlitePool,
  id: String,
  name: String,
  secret: String,
) -> Result<Client, sqlx::Error> {
  let rec = sqlx::query_as(
    format!(
      r#"
INSERT INTO clients( id, name, secret )
VALUES ( {:?}, {:?}, {:?} )
RETURNING *
        "#,
      id, name, secret
    )
    .as_str(),
  )
  .fetch_one(&pool)
  .await;
  let r: crate::model::clients::ClientModel = match rec {
    Ok(rc) => rc,
    Err(e) => {
      tracing::error!("clients 50 - db error: {}", e);
      return Err(e);
    }
  };

  Ok(Client {
    id: r.id,
    name: r.name,
    secret: r.secret,
    created_at: r.created_at,
    updated_at: r.updated_at,
  })
}

pub async fn get_client(pool: &sqlx::SqlitePool, id: String) -> Result<Client, sqlx::Error> {
  let qry = sqlx::query_as(format!("SELECT * FROM clients WHERE id = {:?}", id).as_str())
    .fetch_one(pool)
    .await;
  match qry {
    Ok(rec) => {
      let r: crate::model::clients::ClientModel = rec;
      Ok(Client {
        id: r.id,
        name: r.name,
        secret: r.secret,
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
        tracing::error!("clients 89 - db error: {}", e);
        Err(e)
      }
    },
  }
}
