use surrealdb::engine::remote::ws::Client as SurrealClient;
use surrealdb::Surreal;

use crate::db::surreal::models::{Group, GroupClient, JobGroup};

pub struct GroupDal;

impl GroupDal {
    pub fn new() -> Self {
        Self
    }

    pub async fn create(&self, db: &Surreal<SurrealClient>, group: &Group) -> Result<Group, surrealdb::Error> {
        let created: Option<Group> = db
            .create("groups")
            .content(group.clone())
            .await?;
        
        created.ok_or_else(|| {
            let msg = "Failed to create group: no result returned".to_string();
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg)
        })
    }

    pub async fn read(&self, db: &Surreal<SurrealClient>, id: &str) -> Result<Group, surrealdb::Error> {
        let result: Option<Group> = db.select(("groups", id)).await?;
        
        result.ok_or_else(|| {
            let msg = format!("Group not found: {}", id);
            tracing::error!("{}", msg);
            surrealdb::Error::not_found(msg, None)
        })
    }

    pub async fn update(&self, db: &Surreal<SurrealClient>, group: &Group) -> Result<Group, surrealdb::Error> {
        let id = group.id.as_ref().ok_or_else(|| {
            let msg = "Group has no ID".to_string();
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg)
        })?;
        
        let updated: Option<Group> = db
            .update(("groups", id.as_str()))
            .content(group.clone())
            .await?;
        
        updated.ok_or_else(|| {
            let msg = format!("Failed to update group: {}", id);
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg)
        })
    }

    pub async fn delete(&self, db: &Surreal<SurrealClient>, id: &str) -> Result<(), surrealdb::Error> {
        let _: Option<Group> = db.delete(("groups", id)).await?;
        tracing::info!("Group deleted: {}", id);
        Ok(())
    }

    pub async fn upsert(&self, db: &Surreal<SurrealClient>, group: &Group) -> Result<Group, surrealdb::Error> {
        let id = group.id.as_ref().ok_or_else(|| {
            let msg = "Group has no ID for upsert".to_string();
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg)
        })?;
        
        let upserted: Option<Group> = db
            .upsert(("groups", id.as_str()))
            .content(group.clone())
            .await?;
        
        upserted.ok_or_else(|| {
            let msg = format!("Failed to upsert group: {}", id);
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg)
        })
    }

    pub async fn list(&self, db: &Surreal<SurrealClient>) -> Result<Vec<Group>, surrealdb::Error> {
        let groups: Vec<Group> = db.select("groups").await?;
        Ok(groups)
    }

    pub async fn add_client(&self, db: &Surreal<SurrealClient>, group_id: &str, client_id: &str) -> Result<GroupClient, surrealdb::Error> {
        let group_client = GroupClient::new(
            Some(group_id.to_string()),
            Some(client_id.to_string()),
        );
        
        let created: Option<GroupClient> = db
            .create("groups_clients")
            .content(group_client)
            .await?;
        
        created.ok_or_else(|| {
            let msg = "Failed to add client to group".to_string();
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg)
        })
    }

    pub async fn remove_client(&self, db: &Surreal<SurrealClient>, group_id: &str, client_id: &str) -> Result<(), surrealdb::Error> {
        let grp_id = group_id.to_string();
        let clt_id = client_id.to_string();
        let _: Vec<GroupClient> = db
            .query("DELETE FROM groups_clients WHERE group_id = $group_id AND client_id = $client_id")
            .bind(("group_id", grp_id))
            .bind(("client_id", clt_id))
            .await?
            .take(0)?;
        tracing::info!("Removed client {} from group {}", client_id, group_id);
        Ok(())
    }

    pub async fn get_clients(&self, db: &Surreal<SurrealClient>, group_id: &str) -> Result<Vec<String>, surrealdb::Error> {
        let group_id = group_id.to_string();
        let mut result = db
            .query("SELECT client_id FROM groups_clients WHERE group_id = $group_id")
            .bind(("group_id", group_id))
            .await?;
        
        let group_clients: Vec<GroupClient> = result.take(0)?;
        Ok(group_clients.into_iter().filter_map(|gc| gc.client_id).collect())
    }

    pub async fn add_job(&self, db: &Surreal<SurrealClient>, job_id: &str, group_id: &str) -> Result<JobGroup, surrealdb::Error> {
        let job_group = JobGroup::new(
            Some(job_id.to_string()),
            Some(group_id.to_string()),
        );
        
        let created: Option<JobGroup> = db
            .create("jobs_groups")
            .content(job_group)
            .await?;
        
        created.ok_or_else(|| {
            let msg = "Failed to add job to group".to_string();
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg)
        })
    }

    pub async fn remove_job(&self, db: &Surreal<SurrealClient>, job_id: &str, group_id: &str) -> Result<(), surrealdb::Error> {
        let jb_id = job_id.to_string();
        let grp_id = group_id.to_string();
        let _: Vec<JobGroup> = db
            .query("DELETE FROM jobs_groups WHERE job_id = $job_id AND group_id = $group_id")
            .bind(("job_id", jb_id))
            .bind(("group_id", grp_id))
            .await?
            .take(0)?;
        tracing::info!("Removed job {} from group {}", job_id, group_id);
        Ok(())
    }
}

impl Default for GroupDal {
    fn default() -> Self {
        Self::new()
    }
}
