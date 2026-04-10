use remex_core::db::DbError;
use surrealdb::{
  engine::local::Db,
  Surreal,
};

pub mod endpoint;
pub mod remex;

struct Person {
  id: u8,
  name: String,
  age: u8,
}

pub async fn migrate(db: &Surreal<Db>) -> Result<(), DbError> {
  endpoint::Session::migrate(db).await?;
  // remex::Job::migrate(db).await?;
  Ok(())
}
