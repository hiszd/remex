use std::sync::Arc;

use surrealdb::{
  engine::remote::ws::{
    Client,
    Ws,
  },
  opt::auth::Root,
  Surreal,
};
use tokio::sync::RwLock;

use super::schema::SCHEMA_DEFINITIONS;

pub type Db = Arc<RwLock<Surreal<Client>>>;

pub struct SurrealConfig {
  pub url: String,
  pub username: String,
  pub password: String,
  pub namespace: String,
  pub database: String,
}

impl Default for SurrealConfig {
  fn default() -> Self {
    dotenvy::dotenv().ok();

    Self {
      url: std::env::var("SURREALDB_URL").unwrap_or_else(|_| "ws://192.168.10.87:8090".to_string()),
      username: std::env::var("SURREALDB_USER").unwrap_or_else(|_| "root".to_string()),
      password: std::env::var("SURREALDB_PASS").unwrap_or_else(|_| "H@ck3r345".to_string()),
      namespace: std::env::var("SURREALDB_NAMESPACE").unwrap_or_else(|_| "remex".to_string()),
      database: std::env::var("SURREALDB_DATABASE").unwrap_or_else(|_| "remex".to_string()),
    }
  }
}

pub async fn connect(config: &SurrealConfig) -> Result<Db, surrealdb::Error> {
  let db = Surreal::new::<Ws>(config.url.as_str()).await?;

  db.signin(Root {
    username: config.username.clone(),
    password: config.password.clone(),
  })
  .await?;

  db.use_ns(&config.namespace)
    .use_db(&config.database)
    .await?;

  Ok(Arc::new(RwLock::new(db)))
}

pub async fn connect_with_jwt(
  url: &str,
  namespace: &str,
  database: &str,
  jwt_token: &str,
) -> Result<Db, surrealdb::Error> {
  let db = Surreal::new::<Ws>(url).await?;

  db.authenticate(jwt_token.to_string()).await?;

  db.use_ns(namespace).use_db(database).await?;

  Ok(Arc::new(RwLock::new(db)))
}

pub async fn connect_default() -> Result<Db, surrealdb::Error> {
  let config = SurrealConfig::default();
  connect(&config).await
}

pub async fn init_schema(db: &Db) -> Result<(), surrealdb::Error> {
  let db = db.read().await;

  let queries: Vec<&str> = SCHEMA_DEFINITIONS
    .split(';')
    .map(|s| s.trim())
    .filter(|s| !s.is_empty())
    .collect();

  for query in queries {
    if !query.starts_with("--") && !query.is_empty() {
      let _ = db.query(query).await?;
    }
  }

  Ok(())
}

pub async fn migrate(db: &Db) -> Result<(), surrealdb::Error> { init_schema(db).await }
