pub async fn add_log(
  pool: &sqlx::PgPool,
  client: &str,
  message: &str,
  time_logged: chrono::NaiveDateTime,
) -> Result<String, sqlx::Error> {
  let rec: crate::model::logs::LogModel = sqlx::query_as(
    format!(
      "
INSERT INTO logs( client, message, time_logged )
VALUES ( {client}, {message}, {time_logged} )
RETURNING id
        "
    )
    .as_str(),
  )
  .fetch_one(pool)
  .await
  .unwrap();

  Ok(rec.id)
}
