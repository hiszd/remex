use crate::db::{model::jobs_groups::JobsGroupsModel, Connections};

/* **************************************************************************** */
/* *********************************** Queries ******************************** */
/* **************************************************************************** */

pub enum JobsGroupsSelector {
  JobId(String),
  GroupId(String),
}

pub async fn get_jobs_groups(
  pool: Connections,
  id: JobsGroupsSelector,
) -> Result<JobsGroupsModel, sqlx::Error> {
  let (sel, id) = match id {
    JobsGroupsSelector::JobId(id) => ("job_id", id),
    JobsGroupsSelector::GroupId(id) => ("group_id", id),
  };
  let qry = match pool {
    Connections::Sqlite(p) => {
      sqlx::query_as(format!("SELECT * FROM jobs_groups WHERE {} = {}", sel, id).as_str())
        .fetch_one(&p)
        .await
    }
    Connections::Postgres(p) => {
      sqlx::query_as(format!("SELECT * FROM jobs_groups WHERE {} = \'{}\'", sel, id).as_str())
        .fetch_one(&p)
        .await
    }
  };
  match qry {
    Ok(rec) => {
      let r: JobsGroupsModel = rec;
      Ok(r)
    }
    Err(e) => match e {
      sqlx::Error::RowNotFound => {
        tracing::error!("group not found");
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

pub async fn add_jobs_groups(
  pool: Connections,
  job_id: String,
  group_id: String,
) -> anyhow::Result<JobsGroupsModel> {
  let qry = match pool {
    Connections::Sqlite(p) => {
      sqlx::query_as(
        format!(
          "
INSERT INTO groups_clients( job_id, group_id )
VALUES ( {}, {} )
RETURNING *
",
          job_id, group_id
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
INSERT INTO groups_clients( job_id, group_id )
VALUES ( \'{}\', \'{}\' )
RETURNING *
",
          job_id, group_id
        )
        .as_str(),
      )
      .fetch_one(&p)
      .await
    }
  };
  let r: JobsGroupsModel = match qry {
    Ok(rc) => rc,
    Err(e) => {
      anyhow::bail!("db error: {}", e);
    }
  };

  Ok(r)
}
