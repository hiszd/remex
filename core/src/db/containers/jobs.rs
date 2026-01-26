use crate::db::{
  actions::jobs::{commands::upsert_job, requests::get_job_complete},
  model::{clients::ClientsModel, jobs::JobsComplete},
  Pools,
};

#[allow(dead_code)]
#[derive(thiserror::Error, Debug)]
pub enum JobContError {
  #[error("Job not found")]
  JobNotFound,
  #[error("Use of the Client pool is not allowed for this purpose")]
  ClientPoolNotAllowed,
  #[error("Sqlx Error: {0}")]
  SqlxError(String),
}

#[derive(Debug, Clone)]
pub struct JobCont {
  pub job: JobsComplete,
  pub clients: Vec<ClientsModel>,
  updated_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
}

impl JobCont {
  pub async fn new(
    pool: &sqlx::Pool<sqlx::Postgres>,
    job_name: String,
    job_type: String,
    job_status: String,
    job_shell: String,
  ) -> JobCont {
    let job = upsert_job(pool, &job_name, &job_type, &job_status, &job_shell).await.unwrap();
    let jobcomp: JobsComplete =
      get_job_complete(Pools::Postgres(pool.clone()), job.id).await.unwrap();
    JobCont {
      job: jobcomp,
      clients: Vec::new(),
      updated_at: sqlx::types::chrono::Utc::now(),
    }
  }

  // pub async fn update(&mut self, pool: Pools) -> Result<(), JobContError> {
  //   match pool {
  //     Pools::Postgres(p) => {
  //       if self.updated_at < sqlx::types::chrono::Utc::now() {
  //         let qry = get_job_assigned(Pools::Postgres(p.clone()), self.job.id).await;
  //         return match qry {
  //           Ok(rec) => {
  //             let g = rec.0;
  //             let clnts = rec.1;
  //             // NOTE: if rec has been updated more recently than this record
  //             if self.updated_at < g.updated_at {
  //               self.updated_at = chrono::Utc::now();
  //               self.job = g;
  //               self.clients = clnts.0;
  //             }
  //             Ok(())
  //           }
  //           Err(e) => {
  //             self.updated_at = chrono::Utc::now();
  //             match e {
  //               sqlx::Error::RowNotFound => {
  //                 tracing::error!("job not found");
  //                 Err(JobContError::JobNotFound)
  //               }
  //               _ => {
  //                 tracing::error!("db error: {}", e);
  //                 Err(JobContError::SqlxError(e.to_string()))
  //               }
  //             }
  //           }
  //         };
  //       }
  //       Ok(())
  //     }
  //     _ => Err(JobContError::ClientPoolNotAllowed),
  //   }
  // }

  pub async fn add_client(&mut self, pool: &sqlx::Pool<sqlx::Postgres>, client: ClientsModel) {
    match crate::db::actions::jobs::commands::add_client(
      pool,
      self.job.id,
      sqlx::types::Uuid::parse_str(&client.id.to_string()).unwrap(),
    )
    .await
    {
      Ok(_) => {
        self.updated_at = chrono::Utc::now();
        self.clients.push(client);
      }
      Err(e) => {
        tracing::error!("db error: {}", e);
      }
    }
  }

  pub async fn remove_client(&mut self, pool: &sqlx::Pool<sqlx::Postgres>, client: ClientsModel) {
    match crate::db::actions::jobs::commands::remove_client(pool, self.job.id, client.id).await {
      Ok(_) => {
        self.updated_at = chrono::Utc::now();
        self.clients.retain(|c| c.id != client.id);
      }
      Err(e) => {
        tracing::error!("db error: {}", e);
      }
    }
  }
}
