use crate::db::{model::executions::ExecutionsModel, Pools};

pub async fn add_execution(
  pool: Pools,
  job_id: String,
  client_id: String,
  executed_at: chrono::DateTime<chrono::Utc>,
  execution_result: String,
) -> anyhow::Result<ExecutionsModel> {
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
  let r: crate::db::model::executions::ExecutionsModel = match qry {
    Ok(rc) => rc,
    Err(e) => {
      anyhow::bail!("db error: {}", e);
    }
  };

  Ok(r)
}
