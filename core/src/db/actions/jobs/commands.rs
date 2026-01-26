use crate::db::{
  model::{jobs::JobsModel, jobs_clients::JobsClientsModel, jobs_groups::JobsGroupsModel},
  util::cast_uuid,
  Pools,
};

pub async fn add_job(
  pool: Pools,
  job_name: String,
  job_type: String,
  job_status: String,
  job_shell: String,
) -> anyhow::Result<JobsModel> {
  let qry = sea_query::Query::insert()
    .into_table("jobs")
    .columns(vec!["job_name", "job_type", "job_status", "job_shell"])
    .values_panic(vec![job_name.into(), job_type.into(), job_status.into(), job_shell.into()])
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
  let r: crate::db::model::jobs::JobsModel = match res {
    Ok(rc) => rc,
    Err(e) => {
      anyhow::bail!("db error: {}", e);
    }
  };
  Ok(r)
}

pub async fn upsert_job(
  pool: &sqlx::Pool<sqlx::Postgres>,
  job_name: &str,
  job_type: &str,
  job_status: &str,
  job_shell: &str,
) -> Result<JobsModel, sqlx::Error> {
  let q = sea_query::Query::insert()
    .into_table("jobs")
    .columns(["job_name", "job_type", "job_status", "job_shell"])
    .values_panic([job_name.into(), job_type.into(), job_status.into(), job_shell.into()])
    .on_conflict(
      sea_query::OnConflict::column("job_name")
        // Performs the "no-op" update: id = EXCLUDED.id
        .update_column("job_name")
        .to_owned(),
    )
    .returning_all()
    .to_string(sea_query::PostgresQueryBuilder);
  tracing::info!("{}", q);
  match sqlx::query_as(&q).fetch_one(pool).await {
    Ok(rec) => Ok(rec),
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

pub async fn add_client(
  pool: &sqlx::Pool<sqlx::Postgres>,
  job_id: sqlx::types::Uuid,
  client_id: sqlx::types::Uuid,
) -> anyhow::Result<JobsClientsModel> {
  let qry = sea_query::Query::insert()
    .into_table("jobs_clients")
    .columns(["job_id", "client_id"])
    .values_panic([job_id.to_string().into(), client_id.to_string().into()])
    .on_conflict(sea_query::OnConflict::new().do_nothing().to_owned())
    .returning_all()
    .to_string(sea_query::PostgresQueryBuilder);
  match sqlx::query_as(&qry).fetch_one(pool).await {
    Ok(rc) => Ok(rc),
    Err(e) => Err(e.into()),
  }
}

pub async fn add_group(
  pool: &sqlx::Pool<sqlx::Postgres>,
  job_id: sqlx::types::Uuid,
  group_id: sqlx::types::Uuid,
) -> anyhow::Result<JobsGroupsModel> {
  let qry = sea_query::Query::insert()
    .into_table("jobs_groups")
    .columns(["job_id", "group_id"])
    .values_panic([job_id.to_string().into(), group_id.to_string().into()])
    .on_conflict(sea_query::OnConflict::new().do_nothing().to_owned())
    .returning_all()
    .to_string(sea_query::PostgresQueryBuilder);
  match sqlx::query_as(&qry).fetch_one(pool).await {
    Ok(rc) => Ok(rc),
    Err(e) => Err(e.into()),
  }
}

pub async fn remove_client(
  pool: &sqlx::Pool<sqlx::Postgres>,
  job_id: sqlx::types::Uuid,
  client_id: sqlx::types::Uuid,
) -> anyhow::Result<()> {
  let qry = sea_query::Query::delete()
    .from_table("jobs_clients")
    .and_where(sea_query::Expr::col("job_id").eq(cast_uuid(job_id)))
    .and_where(sea_query::Expr::col("client_id").eq(cast_uuid(client_id)))
    .to_string(sea_query::PostgresQueryBuilder);
  match sqlx::query(&qry).execute(pool).await {
    Err(e) => Err(e.into()),
    _ => Ok(()),
  }
}

pub async fn remove_group(
  pool: &sqlx::Pool<sqlx::Postgres>,
  job_id: sqlx::types::Uuid,
  group_id: sqlx::types::Uuid,
) -> anyhow::Result<()> {
  let qry = sea_query::Query::delete()
    .from_table("jobs_groups")
    .and_where(sea_query::Expr::col("job_id").eq(cast_uuid(job_id)))
    .and_where(sea_query::Expr::col("group_id").eq(cast_uuid(group_id)))
    .to_string(sea_query::PostgresQueryBuilder);
  match sqlx::query(&qry).execute(pool).await {
    Err(e) => Err(e.into()),
    _ => Ok(()),
  }
}

pub async fn remove_job(
  pool: &sqlx::Pool<sqlx::Postgres>,
  job_id: sqlx::types::Uuid,
) -> anyhow::Result<()> {
  let qry = sea_query::Query::delete()
    .from_table("jobs")
    .and_where(sea_query::Expr::col("id").eq(cast_uuid(job_id)))
    .to_string(sea_query::PostgresQueryBuilder);
  match sqlx::query(&qry).execute(pool).await {
    Err(e) => Err(e.into()),
    _ => Ok(()),
  }
}
