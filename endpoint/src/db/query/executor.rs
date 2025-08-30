use thiserror::Error;

pub struct ExecutorDb {
  pub id: String,
  pub name: String,
  pub command: String,
  pub status: String,
  pub active: bool,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

#[derive(Error, Debug)]
pub enum Error {
  #[error("SQLX error: {0}")]
  Sqlx(sqlx::Error),
  #[error("Executor SQLX Error: {0}")]
  Other(String),
}

pub async fn add_executor(
  pool: &sqlx::PgPool,
  id: String,
  name: String,
  command: String,
) -> Result<ExecutorDb, Error> {
  let query = format!(
    r#"
INSERT INTO executors( id, name, command )
VALUES ( {id:?}, {name:?}, {command:?} )
RETURNING *
        "#
  );
  let rec = sqlx::query_as(query.as_str()).fetch_one(pool).await;
  let r: remex_core::db::server::model::executor::ExecutorModel = match rec {
    Ok(rc) => rc,
    Err(e) => {
      tracing::error!("executors 62 - db error: {}", e);
      return Err(Error::Sqlx(e));
    }
  };

  Ok(ExecutorDb {
    id: r.id,
    name: r.name,
    command: r.command,
    status: r.status,
    active: r.active,
    created_at: r.created_at,
    updated_at: r.updated_at,
  })
}

pub async fn get_executor(pool: &sqlx::PgPool, id: String) -> Result<ExecutorDb, Error> {
  let qry = sqlx::query_as(format!("SELECT * FROM executors WHERE id = '{id}'").as_str())
    .fetch_one(pool)
    .await;
  match qry {
    Ok(rec) => {
      let r: remex_core::db::server::model::executor::ExecutorModel = rec;
      Ok(ExecutorDb {
        id: r.id,
        name: r.name,
        command: r.command,
        status: r.status,
        active: r.active,
        created_at: r.created_at,
        updated_at: r.updated_at,
      })
    }
    Err(e) => match e {
      sqlx::Error::RowNotFound => {
        tracing::error!("executor not found");
        Err(Error::Sqlx(e))
      }
      _ => {
        tracing::error!("executors 97 - db error: {}", e);
        Err(Error::Sqlx(e))
      }
    },
  }
}

pub async fn get_executor_from_machineid(
  pool: &sqlx::PgPool,
  machineid: String,
) -> Result<Vec<ExecutorDb>, Error> {
  let qry = sqlx::query_as(
    format!(
      r#"
          SELECT
  e.*
FROM
  clients_executors ce
JOIN
  executors e ON ce.executor_id = e.id
WHERE
  ce.machineid = '{}';
          "#,
      machineid
    )
    .as_str(),
  )
  .fetch_all(pool)
  .await;
  match qry {
    Ok(rec) => {
      let r: Vec<remex_core::db::server::model::executor::ExecutorModel> = rec;
      Ok(
        r.iter()
          .map(|r| ExecutorDb {
            id: r.id.clone(),
            name: r.name.clone(),
            command: r.command.clone(),
            status: r.status.clone(),
            active: r.active,
            created_at: r.created_at,
            updated_at: r.updated_at,
          })
          .collect(),
      )
    }
    Err(e) => match e {
      sqlx::Error::RowNotFound => {
        tracing::error!("executor not found");
        Err(Error::Sqlx(e))
      }
      _ => {
        tracing::error!("executors 121 - db error: {}", e);
        Err(Error::Sqlx(e))
      }
    },
  }
}
