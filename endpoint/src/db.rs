use std::sync::LazyLock;

use remex_core::db::DbError;
use surrealdb::{
  engine::local::Db,
  Surreal,
};

pub static LOCAL_DB: LazyLock<Surreal<Db>> = LazyLock::new(Surreal::init);

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
