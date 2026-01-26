use crate::db::{model::logs::LogsModel, Pools};

pub async fn add_log(
  pool: Pools,
  client_id: String,
  execution_id: Option<String>,
) -> anyhow::Result<LogsModel> {
  let qry = match pool {
    Pools::Sqlite(p) => {
      sqlx::query_as(
        format!(
          "
INSERT INTO logs( client_id{} )
VALUES ( {}{} )
RETURNING *
",
          if execution_id.is_some() {
            ", execution_id"
          } else {
            ""
          },
          client_id,
          if let Some(e) = execution_id.clone() {
            format!(", {}", e)
          } else {
            "".to_string()
          }
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
INSERT INTO logs( client_id{} )
VALUES ( \'{}\'\'{}\' )
RETURNING *
",
          if execution_id.is_some() {
            ", execution_id"
          } else {
            ""
          },
          client_id,
          if let Some(e) = execution_id.clone() {
            format!(", \'{}\'", e)
          } else {
            "".to_string()
          }
        )
        .as_str(),
      )
      .fetch_one(&p)
      .await
    }
  };
  let r: crate::db::model::logs::LogsModel = match qry {
    Ok(rc) => rc,
    Err(e) => {
      anyhow::bail!("db error: {}", e);
    }
  };

  Ok(r)
}
