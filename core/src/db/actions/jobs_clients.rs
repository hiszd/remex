use crate::db::{model::jobs_clients::JobsClientsModel, Connections};

/* **************************************************************************** */
/* *********************************** Queries ******************************** */
/* **************************************************************************** */

pub enum JobsClientsSelector {
  JobId(String),
  ClientId(String),
}

pub async fn get_jobs_clients(
  pool: Connections,
  id: JobsClientsSelector,
) -> Result<JobsClientsModel, sqlx::Error> {
  let (sel, id) = match id {
    JobsClientsSelector::JobId(id) => ("job_id", id),
    JobsClientsSelector::ClientId(id) => ("client_id", id),
  };
  let qry = match pool {
    Connections::Sqlite(p) => {
      sqlx::query_as(format!("SELECT * FROM jobs_clients WHERE {} = {}", sel, id).as_str())
        .fetch_one(&p)
        .await
    }
    Connections::Postgres(p) => {
      sqlx::query_as(format!("SELECT * FROM jobs_clients WHERE {} = \'{}\'", sel, id).as_str())
        .fetch_one(&p)
        .await
    }
  };
  match qry {
    Ok(rec) => {
      let r: JobsClientsModel = rec;
      Ok(r)
    }
    Err(e) => match e {
      sqlx::Error::RowNotFound => {
        tracing::error!("client not found");
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

pub async fn add_jobs_clients(
  pool: Connections,
  job_id: String,
  client_id: String,
) -> anyhow::Result<JobsClientsModel> {
  let qry = match pool {
    Connections::Sqlite(p) => {
      sqlx::query_as(
        format!(
          "
INSERT INTO clients_clients( job_id, client_id )
VALUES ( {}, {} )
RETURNING *
",
          job_id, client_id
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
INSERT INTO clients_clients( job_id, client_id )
VALUES ( \'{}\', \'{}\' )
RETURNING *
",
          job_id, client_id
        )
        .as_str(),
      )
      .fetch_one(&p)
      .await
    }
  };
  let r: JobsClientsModel = match qry {
    Ok(rc) => rc,
    Err(e) => {
      anyhow::bail!("db error: {}", e);
    }
  };

  Ok(r)
}
