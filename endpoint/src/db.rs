use remex_core::db::DbError;
use surrealdb::{
  engine::local::Db,
  Surreal,
};

pub mod endpoint;

pub async fn migrate(db: &Surreal<Db>) -> Result<(), DbError> {
  endpoint::Session::migrate(db).await?;
  // remex::Job::migrate(db).await?;
  Ok(())
}
