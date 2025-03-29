pub async fn add_log(
  pool: &sqlx::PgPool,
  client: &str,
  message: &str,
  time_logged: chrono::NaiveDateTime,
) -> anyhow::Result<i64> {
  let rec = sqlx::query!(
    r#"
INSERT INTO logs( client, message, time_logged )
VALUES ( $1, $2, $3 )
RETURNING id
        "#,
    client,
    message,
    time_logged
  )
  .fetch_one(pool)
  .await?;

  Ok(rec.id as i64)
}
