use remex_core::db::surreal::connection::connect_with_jwt;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type Db = Arc<RwLock<Option<surrealdb::Surreal<surrealdb::engine::remote::ws::Client>>>>;

pub async fn connect(url: &str, namespace: &str, database: &str, jwt_token: &str) -> Result<Db, surrealdb::Error> {
    let db = connect_with_jwt(url, namespace, database, jwt_token).await?;
    let wrapped: Db = Arc::new(RwLock::new(Some(db.read().await.clone())));
    Ok(wrapped)
}

pub async fn get_client(db: &Db) -> Option<surrealdb::Surreal<surrealdb::engine::remote::ws::Client>> {
    let guard = db.read().await;
    guard.clone()
}

pub async fn disconnect(db: &Db) {
    let mut guard = db.write().await;
    *guard = None;
}
