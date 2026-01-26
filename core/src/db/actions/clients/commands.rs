use crate::db::{model::clients::ClientsModel, util::cast_uuid, Pools};

pub async fn add_client(
  pool: Pools,
  client_name: String,
  secret: String,
) -> anyhow::Result<ClientsModel> {
  let qry = sea_query::Query::insert()
    .into_table("clients")
    .columns(vec!["client_name", "secret"])
    .values_panic(vec![client_name.into(), secret.into()])
    .returning_all()
    .to_owned();
  let res = match pool {
    Pools::Sqlite(p) => {
      sqlx::query_as(&qry.to_string(sea_query::SqliteQueryBuilder)).fetch_one(&p).await
    }
    Pools::Postgres(p) => {
      sqlx::query_as(&qry.to_string(sea_query::PostgresQueryBuilder)).fetch_one(&p).await
    }
  };
  let r: crate::db::model::clients::ClientsModel = match res {
    Ok(rc) => rc,
    Err(e) => {
      anyhow::bail!("db error: {}", e);
    }
  };
  Ok(r)
}

pub async fn remove_client(
  pool: &sqlx::Pool<sqlx::Postgres>,
  client_id: sqlx::types::Uuid,
) -> anyhow::Result<()> {
  let qry = sea_query::Query::delete()
    .from_table("clients")
    .and_where(sea_query::Expr::col("id").eq(cast_uuid(client_id)))
    .to_string(sea_query::PostgresQueryBuilder);
  match sqlx::query(&qry).execute(pool).await {
    Err(e) => Err(e.into()),
    _ => Ok(()),
  }
}
