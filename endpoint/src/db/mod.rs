use std::path::Path;

pub mod model;

pub async fn migrate(pool: sqlx::SqlitePool) {
  tracing::warn!("Migrating db");

  let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();

  sqlx::migrate::Migrator::new(Path::new(&crate_dir).join("./migrations"))
    .await
    .unwrap()
    .run(&<sqlx::SqlitePool>::clone(&pool))
    .await
    .unwrap();
}
