use surrealdb::engine::remote::ws::Client as SurrealClient;
use surrealdb::Surreal;

use crate::db::surreal::models::Job;

pub struct JobDal;

impl JobDal {
    pub fn new() -> Self {
        Self
    }

    pub async fn create(&self, db: &Surreal<SurrealClient>, job: &Job) -> Result<Job, surrealdb::Error> {
        let created: Option<Job> = db
            .create("jobs")
            .content(job.clone())
            .await?;
        
        created.ok_or_else(|| {
            let msg = "Failed to create job: no result returned".to_string();
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg)
        })
    }

    pub async fn read(&self, db: &Surreal<SurrealClient>, id: &str) -> Result<Job, surrealdb::Error> {
        let result: Option<Job> = db.select(("jobs", id)).await?;
        
        result.ok_or_else(|| {
            let msg = format!("Job not found: {}", id);
            tracing::error!("{}", msg);
            surrealdb::Error::not_found(msg, None)
        })
    }

    pub async fn update(&self, db: &Surreal<SurrealClient>, job: &Job) -> Result<Job, surrealdb::Error> {
        let id = job.id.as_ref().ok_or_else(|| {
            let msg = "Job has no ID".to_string();
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg)
        })?;
        
        let updated: Option<Job> = db
            .update(("jobs", id.as_str()))
            .content(job.clone())
            .await?;
        
        updated.ok_or_else(|| {
            let msg = format!("Failed to update job: {}", id);
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg)
        })
    }

    pub async fn delete(&self, db: &Surreal<SurrealClient>, id: &str) -> Result<(), surrealdb::Error> {
        let _: Option<Job> = db.delete(("jobs", id)).await?;
        tracing::info!("Job deleted: {}", id);
        Ok(())
    }

    pub async fn upsert(&self, db: &Surreal<SurrealClient>, job: &Job) -> Result<Job, surrealdb::Error> {
        let id = job.id.as_ref().ok_or_else(|| {
            let msg = "Job has no ID for upsert".to_string();
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg)
        })?;
        
        let upserted: Option<Job> = db
            .upsert(("jobs", id.as_str()))
            .content(job.clone())
            .await?;
        
        upserted.ok_or_else(|| {
            let msg = format!("Failed to upsert job: {}", id);
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg)
        })
    }

    pub async fn list(&self, db: &Surreal<SurrealClient>) -> Result<Vec<Job>, surrealdb::Error> {
        let jobs: Vec<Job> = db.select("jobs").await?;
        Ok(jobs)
    }

    pub async fn find_pending(&self, db: &Surreal<SurrealClient>) -> Result<Vec<Job>, surrealdb::Error> {
        let mut result = db
            .query("SELECT * FROM jobs WHERE job_status = $status")
            .bind(("status", "pending".to_string()))
            .await?;
        
        let jobs: Vec<Job> = result.take(0)?;
        Ok(jobs)
    }

    pub async fn find_by_group(&self, db: &Surreal<SurrealClient>, group_id: &str) -> Result<Vec<Job>, surrealdb::Error> {
        let group_id = group_id.to_string();
        let mut result = db
            .query("SELECT jobs.* FROM jobs WHERE id IN (SELECT job_id FROM job_groups WHERE group_id = $group_id)")
            .bind(("group_id", group_id))
            .await?;
        
        let jobs: Vec<Job> = result.take(0)?;
        Ok(jobs)
    }
}

impl Default for JobDal {
    fn default() -> Self {
        Self::new()
    }
}
