use surrealdb::engine::remote::ws::Client as SurrealClient;
use surrealdb::Surreal;

use crate::db::surreal::models::Client;

pub struct ClientDal;

impl ClientDal {
    pub fn new() -> Self {
        Self
    }

    pub async fn create(&self, db: &Surreal<SurrealClient>, client: &Client) -> Result<Client, surrealdb::Error> {
        let query = "CREATE clients CONTENT $client RETURN *";
        let mut result = db
            .query(query)
            .bind(("client", client.clone()))
            .await?;
        
        let created: Option<Client> = result.take(0)?;
        
        created.ok_or_else(|| {
            let msg = "Failed to create client: no result returned".to_string();
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg)
        })
    }

    pub async fn read(&self, db: &Surreal<SurrealClient>, id: &str) -> Result<Client, surrealdb::Error> {
        let query = "SELECT * FROM clients WHERE string::slice(string::join('', id), 0, $len) = $search LIMIT 1";
        let id_len = id.len();
        let mut result = db
            .query(query)
            .bind(("search", id.to_string()))
            .bind(("len", id_len))
            .await?;
        
        let clients: Vec<Client> = result.take(0)?;
        
        clients.into_iter().next().ok_or_else(|| {
            let msg = format!("Client not found: {}", id);
            tracing::error!("{}", msg);
            surrealdb::Error::not_found(msg, None)
        })
    }

    pub async fn update(&self, db: &Surreal<SurrealClient>, client: &Client) -> Result<Client, surrealdb::Error> {
        let id = client.id.as_ref().ok_or_else(|| {
            let msg = "Client has no ID";
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg.to_string())
        })?;
        
        let query = "UPDATE clients SET secret = $secret, client_name = $client_name, hardware_hash = $hardware_hash, updated_at = time::now()::rfc3339() WHERE string::slice(string::join('', id), 0, $id_len) = $id RETURN *";
        let mut result = db
            .query(query)
            .bind(("id", id.clone()))
            .bind(("id_len", id.len()))
            .bind(("secret", client.secret.clone()))
            .bind(("client_name", client.client_name.clone()))
            .bind(("hardware_hash", client.hardware_hash.clone()))
            .await?;
        
        let updated: Vec<Client> = result.take(0)?;
        
        updated.into_iter().next().ok_or_else(|| {
            let msg = format!("Failed to update client: {}", id);
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg)
        })
    }

    pub async fn delete(&self, db: &Surreal<SurrealClient>, id: &str) -> Result<(), surrealdb::Error> {
        let query = "DELETE FROM clients WHERE string::slice(string::join('', id), 0, $len) = $id";
        let mut result = db
            .query(query)
            .bind(("id", id.to_string()))
            .bind(("len", id.len()))
            .await?;
        
        let _: Vec<Client> = result.take(0)?;
        tracing::info!("Client deleted: {}", id);
        Ok(())
    }

    pub async fn upsert(&self, db: &Surreal<SurrealClient>, client: &Client) -> Result<Client, surrealdb::Error> {
        let id = client.id.as_ref().ok_or_else(|| {
            let msg = "Client has no ID for upsert";
            tracing::error!("{}", msg);
            surrealdb::Error::internal(msg.to_string())
        })?;
        
        let query = "UPDATE clients SET secret = $secret, client_name = $client_name, hardware_hash = $hardware_hash, updated_at = time::now()::rfc3339() WHERE string::slice(string::join('', id), 0, $id_len) = $id RETURN *";
        let mut result = db
            .query(query)
            .bind(("id", id.clone()))
            .bind(("id_len", id.len()))
            .bind(("secret", client.secret.clone()))
            .bind(("client_name", client.client_name.clone()))
            .bind(("hardware_hash", client.hardware_hash.clone()))
            .await?;
        
        let upserted: Vec<Client> = result.take(0)?;
        
        if let Some(client) = upserted.into_iter().next() {
            Ok(client)
        } else {
            self.create(db, client).await
        }
    }

    pub async fn list(&self, db: &Surreal<SurrealClient>) -> Result<Vec<Client>, surrealdb::Error> {
        let mut result = db
            .query("SELECT * FROM clients")
            .await?;
        
        let clients: Vec<Client> = result.take(0)?;
        Ok(clients)
    }

    pub async fn find_by_hardware_hash(
        &self,
        db: &Surreal<SurrealClient>,
        hardware_hash: &str,
    ) -> Result<Option<Client>, surrealdb::Error> {
        let mut result = db
            .query("SELECT * FROM clients WHERE hardware_hash = $hardware_hash LIMIT 1")
            .bind(("hardware_hash", hardware_hash.to_string()))
            .await?;
        
        let clients: Vec<Client> = result.take(0)?;
        Ok(clients.into_iter().next())
    }
}

impl Default for ClientDal {
    fn default() -> Self {
        Self::new()
    }
}
