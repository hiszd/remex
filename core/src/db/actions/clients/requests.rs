use crate::db::{
  model::{clients::ClientsModel, groups::GroupsModel, groups_clients::GroupsClientsModel},
  util::cast_uuid,
  Pools,
};

pub async fn get_client(
  pool: Pools,
  client_id: Option<sqlx::types::Uuid>,
  client_name: Option<String>,
) -> Result<ClientsModel, sqlx::Error> {
  let qry = sea_query::Query::select()
    .from("clients")
    .column(sea_query::Asterisk)
    .apply_if(client_id, |q, v| {
      q.and_where(sea_query::Expr::col("id").eq(cast_uuid(v)));
    })
    .apply_if(client_name, |q, v| {
      q.and_where(sea_query::Expr::col("client_name").eq(v));
    })
    .to_owned();
  let res = match pool {
    Pools::Sqlite(p) => {
      sqlx::query_as(&qry.to_string(sea_query::SqliteQueryBuilder)).fetch_one(&p).await
    }
    Pools::Postgres(p) => {
      sqlx::query_as(&qry.to_string(sea_query::PostgresQueryBuilder)).fetch_one(&p).await
    }
  };
  match res {
    Ok(rec) => {
      let r: crate::db::model::clients::ClientsModel = rec;
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

pub async fn get_client_groups(
  pool: &sqlx::Pool<sqlx::Postgres>,
  client_id: sqlx::types::Uuid,
) -> Result<Vec<GroupsModel>, sqlx::Error> {
  let qry = sea_query::Query::select()
    .from("groups_clients")
    .column(sea_query::Asterisk)
    .and_where(cast_uuid(client_id).eq(client_id.to_string()))
    .to_string(sea_query::PostgresQueryBuilder);
  match sqlx::query_as(&qry).fetch_all(pool).await {
    Ok(rec) => {
      let r: Vec<GroupsClientsModel> = rec;
      let group_ids: Vec<sqlx::types::Uuid> = r.iter().map(|x| x.group_id).collect();
      let g: Vec<GroupsModel> =
        crate::db::actions::groups::requests::get_groups(pool, group_ids).await.unwrap();
      Ok(g)
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
