use surrealdb::engine::remote::ws::Client as SurrealClient;
use surrealdb::Surreal;

use crate::db::surreal::models::Execution;

pub struct ExecutionDal;

impl ExecutionDal {
    pub fn new() -> Self {
        Self
    }

    pub async fn create(&self, db: &Surreal<SurrealClient>, execution: &Execution) -> Result<Execution, surrealdb::Error> {
        let created: Option<Execution> = db
            .create("executions")
            .content(execution.clone())
            .await?;
        
        created.ok_or_else(|| {
            let msg = "Failed to create execution: no result returned".to_string();
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg)
        })
    }

    pub async fn read(&self, db: &Surreal<SurrealClient>, id: &str) -> Result<Execution, surrealdb::Error> {
        let result: Option<Execution> = db.select(("executions", id)).await?;
        
        result.ok_or_else(|| {
            let msg = format!("Execution not found: {}", id);
            tracing::error!("{}", msg);
            surrealdb::Error::not_found(msg, None)
        })
    }

    pub async fn update(&self, db: &Surreal<SurrealClient>, execution: &Execution) -> Result<Execution, surrealdb::Error> {
        let id = execution.id.as_ref().ok_or_else(|| {
            let msg = "Execution has no ID".to_string();
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg)
        })?;
        
        let updated: Option<Execution> = db
            .update(("executions", id.as_str()))
            .content(execution.clone())
            .await?;
        
        updated.ok_or_else(|| {
            let msg = format!("Failed to update execution: {}", id);
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg)
        })
    }

    pub async fn delete(&self, db: &Surreal<SurrealClient>, id: &str) -> Result<(), surrealdb::Error> {
        let _: Option<Execution> = db.delete(("executions", id)).await?;
        tracing::info!("Execution deleted: {}", id);
        Ok(())
    }

    pub async fn upsert(&self, db: &Surreal<SurrealClient>, execution: &Execution) -> Result<Execution, surrealdb::Error> {
        let id = execution.id.as_ref().ok_or_else(|| {
            let msg = "Execution has no ID for upsert".to_string();
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg)
        })?;
        
        let upserted: Option<Execution> = db
            .upsert(("executions", id.as_str()))
            .content(execution.clone())
            .await?;
        
        upserted.ok_or_else(|| {
            let msg = format!("Failed to upsert execution: {}", id);
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg)
        })
    }

    pub async fn list(&self, db: &Surreal<SurrealClient>) -> Result<Vec<Execution>, surrealdb::Error> {
        let executions: Vec<Execution> = db.select("executions").await?;
        Ok(executions)
    }

    pub async fn find_by_client(&self, db: &Surreal<SurrealClient>, client_id: &str) -> Result<Vec<Execution>, surrealdb::Error> {
        let mut result = db
            .query("SELECT * FROM executions WHERE client_id = $client_id ORDER BY created_at DESC")
            .bind(("client_id", client_id.to_string()))
            .await?;
        
        let executions: Vec<Execution> = result.take(0)?;
        Ok(executions)
    }

    pub async fn find_by_job(&self, db: &Surreal<SurrealClient>, job_id: &str) -> Result<Vec<Execution>, surrealdb::Error> {
        let mut result = db
            .query("SELECT * FROM executions WHERE job_id = $job_id ORDER BY created_at DESC")
            .bind(("job_id", job_id.to_string()))
            .await?;
        
        let executions: Vec<Execution> = result.take(0)?;
        Ok(executions)
    }
}

impl Default for ExecutionDal {
    fn default() -> Self {
        Self::new()
    }
}
