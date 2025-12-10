pub async fn add_log(
  pool: &sqlx::SqlitePool,
  client: &str,
  message: &str,
  time_logged: chrono::NaiveDateTime,
) -> anyhow::Result<String> {
  let rec: super::model::logs::LogModel = sqlx::query_as(
    format!(
      "
INSERT INTO logs( client, message, time_logged )
VALUES ( {}, {}, {} )
RETURNING id
        ",
      client, message, time_logged
    )
    .as_str(),
  )
  .fetch_one(pool)
  .await
  .unwrap();

  Ok(rec.id)
}
