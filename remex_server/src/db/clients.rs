pub struct Client {
  pub id: i32,
  pub clientname: String,
  pub created_at: chrono::NaiveDateTime,
}

pub async fn add_client(pool: sqlx::PgPool, id: u64, clientname: String) {
  sqlx::query!(
    r#"
INSERT INTO clients( id, clientname )
VALUES ( $1, $2 )
RETURNING id
        "#,
    id as i32,
    clientname,
  )
  .fetch(&pool);
}

pub async fn get_client(pool: &sqlx::PgPool, id: i32) -> anyhow::Result<Client> {
  let rec = sqlx::query!(r#"SELECT * FROM clients WHERE id = $1"#, id).fetch_one(pool).await?;

  Ok(Client {
    id: rec.id,
    clientname: rec.clientname,
    created_at: rec.created_at.unwrap(),
  })
}
