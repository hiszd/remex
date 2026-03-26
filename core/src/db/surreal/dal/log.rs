use surrealdb::engine::remote::ws::Client as SurrealClient;
use surrealdb::Surreal;

use crate::db::surreal::models::Log;

pub struct LogDal;

impl LogDal {
    pub fn new() -> Self {
        Self
    }

    pub async fn create(&self, db: &Surreal<SurrealClient>, log: &Log) -> Result<Log, surrealdb::Error> {
        let created: Option<Log> = db
            .create("logs")
            .content(log.clone())
            .await?;
        
        created.ok_or_else(|| {
            let msg = "Failed to create log: no result returned".to_string();
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg)
        })
    }

    pub async fn read(&self, db: &Surreal<SurrealClient>, id: &str) -> Result<Log, surrealdb::Error> {
        let result: Option<Log> = db.select(("logs", id)).await?;
        
        result.ok_or_else(|| {
            let msg = format!("Log not found: {}", id);
            tracing::error!("{}", msg);
            surrealdb::Error::not_found(msg, None)
        })
    }

    pub async fn update(&self, db: &Surreal<SurrealClient>, log: &Log) -> Result<Log, surrealdb::Error> {
        let id = log.id.as_ref().ok_or_else(|| {
            let msg = "Log has no ID".to_string();
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg)
        })?;
        
        let updated: Option<Log> = db
            .update(("logs", id.as_str()))
            .content(log.clone())
            .await?;
        
        updated.ok_or_else(|| {
            let msg = format!("Failed to update log: {}", id);
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg)
        })
    }

    pub async fn delete(&self, db: &Surreal<SurrealClient>, id: &str) -> Result<(), surrealdb::Error> {
        let _: Option<Log> = db.delete(("logs", id)).await?;
        tracing::info!("Log deleted: {}", id);
        Ok(())
    }

    pub async fn upsert(&self, db: &Surreal<SurrealClient>, log: &Log) -> Result<Log, surrealdb::Error> {
        let id = log.id.as_ref().ok_or_else(|| {
            let msg = "Log has no ID for upsert".to_string();
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg)
        })?;
        
        let upserted: Option<Log> = db
            .upsert(("logs", id.as_str()))
            .content(log.clone())
            .await?;
        
        upserted.ok_or_else(|| {
            let msg = format!("Failed to upsert log: {}", id);
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg)
        })
    }

    pub async fn list(&self, db: &Surreal<SurrealClient>) -> Result<Vec<Log>, surrealdb::Error> {
        let logs: Vec<Log> = db.select("logs").await?;
        Ok(logs)
    }

    pub async fn find_by_execution(&self, db: &Surreal<SurrealClient>, execution_id: &str) -> Result<Vec<Log>, surrealdb::Error> {
        let execution_id = execution_id.to_string();
        let mut result = db
            .query("SELECT * FROM logs WHERE execution_id = $execution_id ORDER BY created_at DESC")
            .bind(("execution_id", execution_id))
            .await?;
        
        let logs: Vec<Log> = result.take(0)?;
        Ok(logs)
    }

    pub async fn find_by_client(&self, db: &Surreal<SurrealClient>, client_id: &str) -> Result<Vec<Log>, surrealdb::Error> {
        let client_id = client_id.to_string();
        let mut result = db
            .query("SELECT * FROM logs WHERE client_id = $client_id ORDER BY created_at DESC")
            .bind(("client_id", client_id))
            .await?;
        
        let logs: Vec<Log> = result.take(0)?;
        Ok(logs)
    }
}

impl Default for LogDal {
    fn default() -> Self {
        Self::new()
    }
}
