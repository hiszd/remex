use sea_query::{OnConflict, PostgresQueryBuilder};

use crate::db::model::{groups::GroupsModel, groups_clients::GroupsClientsModel};

pub async fn create_group(
  pool: &sqlx::Pool<sqlx::Postgres>,
  group_name: String,
) -> anyhow::Result<GroupsModel> {
  let mut bldr: sqlx::QueryBuilder<sqlx::Postgres> =
    sqlx::QueryBuilder::new("INSERT INTO groups( group_name ) ");
  bldr.push_values(vec![group_name.clone()], |mut b, v| {
    b.push_bind(v);
  });
  bldr.push(" RETURNING *;");
  match bldr.build_query_as().fetch_one(pool).await {
    Ok(rc) => Ok(rc),
    Err(e) => Err(e.into()),
  }
}

pub async fn add_client(
  pool: &sqlx::Pool<sqlx::Postgres>,
  group_id: sqlx::types::Uuid,
  client_id: sqlx::types::Uuid,
) -> anyhow::Result<GroupsClientsModel> {
  let qry = sea_query::Query::insert()
    .into_table("groups_clients")
    .columns(["group_id", "client_id"])
    .values_panic([group_id.to_string().into(), client_id.to_string().into()])
    .on_conflict(OnConflict::new().do_nothing().to_owned())
    .returning_all()
    .to_string(PostgresQueryBuilder);
  match sqlx::query_as(&qry).fetch_one(pool).await {
    Ok(rc) => Ok(rc),
    Err(e) => Err(e.into()),
  }
}

pub async fn remove_client(
  pool: &sqlx::Pool<sqlx::Postgres>,
  group_id: sqlx::types::Uuid,
  client_id: sqlx::types::Uuid,
) -> anyhow::Result<()> {
  let qry = sea_query::Query::delete()
    .from_table("groups_clients")
    .and_where(sea_query::Expr::col("client_id").eq(client_id.to_string()))
    .and_where(sea_query::Expr::col("group_id").eq(group_id.to_string()))
    .to_string(PostgresQueryBuilder);
  match sqlx::query(&qry).execute(pool).await {
    Err(e) => Err(e.into()),
    _ => Ok(()),
  }
}
