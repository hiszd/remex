use sea_query::{Expr, OnConflict, PostgresQueryBuilder, Query};

use crate::db::model::{
  clients::ClientsModel,
  groups::{GroupsModel, GroupsRow},
};

pub async fn get_group(
  pool: &sqlx::Pool<sqlx::Postgres>,
  group_id: sqlx::types::Uuid,
) -> Result<GroupsModel, sqlx::Error> {
  let qry = Query::select()
    .from("groups")
    .and_where(Expr::col("id").is_in(vec![group_id.to_string()]))
    .to_string(PostgresQueryBuilder);
  match sqlx::query_as(&qry).fetch_one(pool).await {
    Ok(rec) => Ok(rec),
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

pub async fn upsert_group(
  pool: &sqlx::Pool<sqlx::Postgres>,
  group_name: &str,
) -> Result<GroupsModel, sqlx::Error> {
  let q = Query::insert()
    .into_table("groups")
    .columns(["group_name"])
    .values_panic([group_name.into()])
    .on_conflict(
      OnConflict::column("group_name")
        // Performs the "no-op" update: id = EXCLUDED.id
        .update_column("group_name")
        .to_owned(),
    )
    .returning_all()
    .to_string(PostgresQueryBuilder);
  tracing::info!("{}", q);
  match sqlx::query_as(&q).fetch_one(pool).await {
    Ok(rec) => Ok(rec),
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

pub async fn get_group_clients(
  pool: &sqlx::Pool<sqlx::Postgres>,
  group_id: sqlx::types::Uuid,
) -> Result<(GroupsModel, Vec<ClientsModel>), sqlx::Error> {
  let query = Query::select()
    // Select standard group columns with aliases
    .column(("g", "id"))
    .column(("g", "group_name"))
    .column(("g", "created_at"))
    .column(("g", "updated_at"))
    // Complex Aggregate: Use Expr::cust for the Postgres-specific JSON logic
    .expr_as(
      Expr::cust(
        r#"COALESCE(
                json_agg(
                    json_build_object(
                        'id', "c"."id",
                        'secret', "c"."secret",
                        'client_name', "c"."client_name",
                        'createdAt', "c"."created_at",
                        'updatedAt', "c"."updated_at"
                    )
                ) FILTER (WHERE "c"."id" IS NOT NULL),
                '[]'::json
            )"#,
      ),
      "clients_json",
    )
    .from_as("groups", "g") // FROM groups g
    // 1. Join to the linking table (gc)
    .left_join(
      sea_query::TableRef::Table(sea_query::SeaRc::new("groups_clients")).alias("gc"),
      Expr::col(("g", "id")).equals(("gc", "group_id")),
    )
    // 2. Join to the client table (c)
    .left_join(
      sea_query::TableRef::Table(sea_query::SeaRc::new("clients")).alias("c"),
      Expr::col(("gc", "client_id")).equals(("c", "id")),
    )
    // WHERE g.id = $1
    .and_where(Expr::col(("g", "id")).eq(group_id.to_string()))
    // GROUP BY g.id
    .group_by_col(("g", "id"))
    .to_owned()
    .to_string(PostgresQueryBuilder);
  tracing::info!("{}", query);
  let (group, clients): (GroupsModel, Vec<ClientsModel>) =
    match sqlx::query_as::<sqlx::Postgres, GroupsRow>(&query).fetch_one(pool).await {
      Ok(rec) => rec.split_group_clients(),
      Err(e) => match e {
        sqlx::Error::RowNotFound => {
          tracing::error!("group not found");
          return Err(e);
        }
        _ => {
          tracing::error!("db error: {}", e);
          return Err(e);
        }
      },
    };
  Ok((group, clients))
}

pub async fn get_groups(
  pool: &sqlx::Pool<sqlx::Postgres>,
  group_ids: Vec<sqlx::types::Uuid>,
) -> Result<Vec<GroupsModel>, sqlx::Error> {
  let gids: Vec<String> = group_ids.iter().map(|x| x.to_string()).collect();
  let qry = sea_query::Query::select()
    .column(sea_query::Asterisk)
    .from("groups")
    .and_where(Expr::col("id").is_in(gids))
    .to_string(PostgresQueryBuilder);
  match sqlx::query_as(&qry).fetch_all(pool).await {
    Ok(rec) => Ok(rec),
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

pub async fn get_all_groups(
  pool: &sqlx::Pool<sqlx::Postgres>,
) -> Result<Vec<GroupsModel>, sqlx::Error> {
  let qry = sea_query::Query::select()
    .column(sea_query::Asterisk)
    .from("groups")
    .to_string(PostgresQueryBuilder);
  match sqlx::query_as(&qry).fetch_all(pool).await {
    Ok(rec) => Ok(rec),
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
