use std::sync::LazyLock;

use remex_core::db::DbError;
use surrealdb::{
  engine::{
    local::Db,
    remote::ws::Client,
  },
  Surreal,
};

pub static LOCAL_DB: LazyLock<surrealdb::Surreal<Db>> = LazyLock::new(surrealdb::Surreal::init);

pub async fn get_local_remex() -> Result<Surreal<Db>, DbError> {
  let db = LOCAL_DB.clone();
  db.use_ns("remex").use_db("remex").await?;
  Ok(db)
}

pub async fn get_local_endpoint() -> Result<Surreal<Db>, DbError> {
  let db = LOCAL_DB.clone();
  db.use_ns("remex").use_db("endpoint").await?;
  Ok(db)
}

pub async fn get_local_config() -> Result<Surreal<Db>, DbError> {
  let db = LOCAL_DB.clone();
  db.use_ns("remex").use_db("config").await?;
  Ok(db)
}

pub static REMOTE_DB: LazyLock<surrealdb::Surreal<Client>> =
  LazyLock::new(surrealdb::Surreal::init);

pub async fn get_remote_remex() -> Result<Surreal<Client>, DbError> {
  let db = REMOTE_DB.clone();
  db.use_ns("remex").use_db("remex").await?;
  Ok(db)
}

pub mod endpoint;
pub mod last_action;
pub mod remex;

pub async fn migrate(db: &Surreal<Db>) -> Result<(), DbError> {
  endpoint::Session::migrate(db).await?;
  last_action::LastAction::migrate(db).await?;
  remex::JobCache::migrate(db).await?;
  remex::ExecutionCache::migrate(db).await?;
  Ok(())
}
