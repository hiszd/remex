use crate::db::{model::logs::LogModel, Connections};

/* *********************************** Queries ******************************** */
/* **************************************************************************** */
/* **************************************************************************** */

pub async fn get_log(pool: Connections, log_id: String) -> Result<LogModel, sqlx::Error> {
  let qry = match pool {
    Connections::Sqlite(p) => {
      sqlx::query_as(format!("SELECT * FROM logs WHERE id = {}", log_id).as_str())
        .fetch_one(&p)
        .await
    }
    Connections::Postgres(p) => {
      sqlx::query_as(format!("SELECT * FROM logs WHERE id = \'{}\'", log_id).as_str())
        .fetch_one(&p)
        .await
    }
  };
  match qry {
    Ok(rec) => {
      let r: crate::db::model::logs::LogModel = rec;
      Ok(r)
    }
    Err(e) => match e {
      sqlx::Error::RowNotFound => {
        tracing::error!("log not found");
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

pub async fn add_log(
  pool: Connections,
  client_id: String,
  execution_id: Option<String>,
) -> anyhow::Result<LogModel> {
  let qry = match pool {
    Connections::Sqlite(p) => {
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
    Connections::Postgres(p) => {
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
  let r: crate::db::model::logs::LogModel = match qry {
    Ok(rc) => rc,
    Err(e) => {
      anyhow::bail!("db error: {}", e);
    }
  };

  Ok(r)
}
