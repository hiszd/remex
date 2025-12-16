use crate::db::{model::executions::ExecutionModel, Pools};

/* **************************************************************************** */
/* *********************************** Queries ******************************** */
/* **************************************************************************** */

pub async fn get_execution(
  pool: Pools,
  execution_id: String,
) -> Result<ExecutionModel, sqlx::Error> {
  let qry = match pool {
    Pools::Sqlite(p) => {
      sqlx::query_as(format!("SELECT * FROM executions WHERE id = {}", execution_id).as_str())
        .fetch_one(&p)
        .await
    }
    Pools::Postgres(p) => {
      sqlx::query_as(format!("SELECT * FROM executions WHERE id = \'{}\'", execution_id).as_str())
        .fetch_one(&p)
        .await
    }
  };
  match qry {
    Ok(rec) => {
      let r: crate::db::model::executions::ExecutionModel = rec;
      Ok(r)
    }
    Err(e) => match e {
      sqlx::Error::RowNotFound => {
        tracing::error!("execution not found");
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

pub async fn add_execution(
  pool: Pools,
  job_id: String,
  client_id: String,
  executed_at: chrono::DateTime<chrono::Utc>,
  execution_result: String,
) -> anyhow::Result<ExecutionModel> {
  let qry = match pool {
    Pools::Sqlite(p) => {
      sqlx::query_as(
        format!(
          "
INSERT INTO executions( job_id, client_id, executed_at, execution_result )
VALUES ( {}, {}, {}, {} )
RETURNING *
",
          job_id, client_id, executed_at, execution_result
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
INSERT INTO executions( job_id, client_id, executed_at, execution_result )
VALUES ( \'{}\', \'{}\', \'{}\', \'{}\' )
RETURNING *
",
          job_id, client_id, executed_at, execution_result
        )
        .as_str(),
      )
      .fetch_one(&p)
      .await
    }
  };
  let r: crate::db::model::executions::ExecutionModel = match qry {
    Ok(rc) => rc,
    Err(e) => {
      anyhow::bail!("db error: {}", e);
    }
  };

  Ok(r)
}
