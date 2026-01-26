use crate::db::{
  model::jobs::{JobsComplete, JobsModel},
  util::cast_uuid,
  Pools,
};

pub async fn get_job(
  pool: Pools,
  job_id: Option<sqlx::types::Uuid>,
  job_name: Option<String>,
) -> Result<JobsModel, sqlx::Error> {
  let qry = sea_query::Query::select()
    .from("jobs")
    .column(sea_query::Asterisk)
    .apply_if(job_id, |q, v| {
      q.and_where(sea_query::Expr::col("id").eq(cast_uuid(v)));
    })
    .apply_if(job_name, |q, v| {
      q.and_where(sea_query::Expr::col("job_name").eq(v));
    })
    .to_owned();
  let qry = match pool {
    Pools::Sqlite(p) => {
      sqlx::query_as(&qry.to_string(sea_query::SqliteQueryBuilder)).fetch_one(&p).await
    }
    Pools::Postgres(p) => {
      sqlx::query_as(&qry.to_string(sea_query::PostgresQueryBuilder)).fetch_one(&p).await
    }
  };
  match qry {
    Ok(rec) => {
      let r: crate::db::model::jobs::JobsModel = rec;
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

pub async fn get_job_complete(
  pool: Pools,
  job_id: sqlx::types::Uuid,
) -> Result<JobsComplete, sqlx::Error> {
  let qry = super::builder::build_job_complete_query(job_id);
  let res = match pool {
    Pools::Sqlite(p) => {
      let row: Result<(sqlx::types::Json<JobsComplete>,), sqlx::Error> =
        sqlx::query_as(&qry.to_string(sea_query::SqliteQueryBuilder)).fetch_one(&p).await;
      row
    }
    Pools::Postgres(p) => {
      let row: Result<(sqlx::types::Json<JobsComplete>,), sqlx::Error> =
        sqlx::query_as(&qry.to_string(sea_query::PostgresQueryBuilder)).fetch_one(&p).await;
      row
    }
  };
  match res {
    Ok(rec) => Ok(rec.0 .0),
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

//   pub async fn get_job_assigned(
//     pool: Pools,
//     job_id: sqlx::types::Uuid,
//   ) -> Result<(JobsModel, (Vec<ClientsComplete>, Vec<GroupsComplete>)), sqlx::Error> {
//     let job_alias = Alias::new("j");
//     let qry = Query::select()
//       .expr_as(
//         Func::cust(Alias::new("jsonb_build_object")).args([
//           Expr::value("id"),
//           Expr::col((job_alias.clone(), Alias::new("id"))),
//           // ... other job fields ...
//           Expr::value("clients"),
//           Expr::in_subquery(build_clients_subquery()),
//           Expr::value("groups"),
//           Expr::subquery(build_groups_subquery()),
//         ]),
//         Alias::new("job_blob"),
//       )
//       .from_as(Alias::new("jobs"), job_alias)
//       .and_where(Expr::col((job_alias, Alias::new("id"))).eq(job_id))
//       .to_owned();
//     let res: (JobsModel, (Vec<ClientsComplete>, Vec<GroupsComplete>)) = match pool {
//       Pools::Sqlite(p) => {
//         let j: Vec<JobsModel> = sqlx::query_as(&jqry.to_string(sea_query::SqliteQueryBuilder))
//           .fetch_all(&p)
//           .await
//           .unwrap();
//         let c: Vec<ClientsComplete> =
//           sqlx::query_as(&cqry.to_string(sea_query::SqliteQueryBuilder))
//             .fetch_all(&p)
//             .await
//             .unwrap();
//         let g: Vec<ClientsComplete> =
//           sqlx::query_as(&gqry.to_string(sea_query::SqliteQueryBuilder))
//             .fetch_all(&p)
//             .await
//             .unwrap();
//         (j, (c, g))
//       }
//       Pools::Postgres(p) => {
//         let j: Vec<ClientsComplete> =
//           sqlx::query_as(&jqry.to_string(sea_query::PostgresQueryBuilder))
//             .fetch_all(&p)
//             .await
//             .unwrap();
//         let c: Vec<ClientsComplete> =
//           sqlx::query_as(&cqry.to_string(sea_query::PostgresQueryBuilder))
//             .fetch_all(&p)
//             .await
//             .unwrap();
//         let g: Vec<ClientsComplete> =
//           sqlx::query_as(&gqry.to_string(sea_query::PostgresQueryBuilder))
//             .fetch_all(&p)
//             .await
//             .unwrap();
//         c.iter().chain(g.iter()).cloned().collect()
//       }
//     };
//     match res {
//       Ok(rec) => Ok(rec.split_job_clients()),
//       Err(e) => match e {
//         sqlx::Error::RowNotFound => {
//           tracing::error!("job not found");
//           Err(e)
//         }
//         _ => {
//           tracing::error!("db error: {}", e);
//           Err(e)
//         }
//       },
//     }
//   }
