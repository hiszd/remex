use crate::db::{model::jobs::JobModel, Pools};

/* **************************************************************************** */
/* *********************************** Queries ******************************** */
/* **************************************************************************** */

pub async fn get_job(pool: Pools, job_id: String) -> Result<JobModel, sqlx::Error> {
  let qry = match pool {
    Pools::Sqlite(p) => {
      sqlx::query_as(format!("SELECT * FROM jobs WHERE id = {}", job_id).as_str())
        .fetch_one(&p)
        .await
    }
    Pools::Postgres(p) => {
      sqlx::query_as(format!("SELECT * FROM jobs WHERE id = \'{}\'", job_id).as_str())
        .fetch_one(&p)
        .await
    }
  };
  match qry {
    Ok(rec) => {
      let r: crate::db::model::jobs::JobModel = rec;
      Ok(r)
    }
    Err(e) => match e {
      sqlx::Error::RowNotFound => {
        tracing::error!("job not found");
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

pub async fn add_job(
  pool: Pools,
  job_name: String,
  job_type: String,
  job_status: Option<String>,
  job_shell: String,
) -> anyhow::Result<JobModel> {
  let status = if let Some(s) = job_status {
    s
  } else {
    "active".to_string()
  };
  let qry = match pool {
    Pools::Sqlite(p) => {
      sqlx::query_as(
        format!(
          "
INSERT INTO jobs( job_name, job_type, job_status, job_shell )
VALUES ( {}, {}, {}, {} )
RETURNING *
",
          job_name, job_type, status, job_shell
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
INSERT INTO jobs( job_name, job_type, job_status, job_shell )
VALUES ( \'{}\', \'{}\', \'{}\', \'{}\' )
RETURNING *
",
          job_name, job_type, status, job_shell
        )
        .as_str(),
      )
      .fetch_one(&p)
      .await
    }
  };
  let r: crate::db::model::jobs::JobModel = match qry {
    Ok(rc) => rc,
    Err(e) => {
      anyhow::bail!("db error: {}", e);
    }
  };

  Ok(r)
}
