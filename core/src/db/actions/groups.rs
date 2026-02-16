use crate::db::{model::groups::GroupModel, Connections};

/* **************************************************************************** */
/* *********************************** Queries ******************************** */
/* **************************************************************************** */

pub async fn get_group(pool: Connections, group_id: String) -> Result<GroupModel, sqlx::Error> {
  let qry = match pool {
    Connections::Sqlite(p) => {
      sqlx::query_as(format!("SELECT * FROM groups WHERE id = {}", group_id).as_str())
        .fetch_one(&p)
        .await
    }
    Connections::Postgres(p) => {
      sqlx::query_as(format!("SELECT * FROM groups WHERE id = \'{}\'", group_id).as_str())
        .fetch_one(&p)
        .await
    }
  };
  match qry {
    Ok(rec) => {
      let r: crate::db::model::groups::GroupModel = rec;
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

pub async fn add_group(pool: Connections, group_name: String) -> anyhow::Result<GroupModel> {
  let qry = match pool {
    Connections::Sqlite(p) => {
      sqlx::query_as(
        format!(
          "
INSERT INTO groups( group_name )
VALUES ( {} )
RETURNING *
",
          group_name
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
INSERT INTO groups( group_name )
VALUES ( \'{}\' )
RETURNING *
",
          group_name
        )
        .as_str(),
      )
      .fetch_one(&p)
      .await
    }
  };
  let r: crate::db::model::groups::GroupModel = match qry {
    Ok(rc) => rc,
    Err(e) => {
      anyhow::bail!("db error: {}", e);
    }
  };

  Ok(r)
}
