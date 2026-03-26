pub mod surreal;

pub use surreal::*;

pub type Db = surreal::Db;

static DB: std::sync::OnceLock<surreal::Db> = std::sync::OnceLock::new();

pub fn get_db() -> &'static surreal::Db {
  DB.get()
    .expect("Database not initialized. Call db::connect() or db::connect_with_config() first.")
}

pub fn set_db(db: surreal::Db) { DB.set(db).expect("Database already initialized"); }

pub async fn connect() -> Result<Db, surrealdb::Error> {
  let db = surreal::connect_default().await?;
  set_db(db.clone());
  surreal::migrate(&db).await?;
  Ok(db)
}

pub async fn connect_with_config(config: &surreal::SurrealConfig) -> Result<Db, surrealdb::Error> {
  let db = surreal::connect(config).await?;
  set_db(db.clone());
  surreal::migrate(&db).await?;
  Ok(db)
}
