use std::path::Path;

use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use tracing::{error, info};

pub struct Db {
  pub filename: String,
  pool: Option<SqlitePool>,
}

impl Db {
  pub async fn new(filename: String) -> Self {
    Self {
      filename: filename.clone(),
      pool: match Db::connect(&filename).await {
        Ok(pool) => Some(pool),
        Err(e) => {
          error!("failed to connect to db: {} with error {:?}", filename.clone(), e);
          None
        }
      },
    }
  }

  async fn connect(filename: &str) -> Result<SqlitePool, ()> {
    let options = SqliteConnectOptions::new().filename(filename).create_if_missing(true);

    match SqlitePool::connect_with(options).await {
      Ok(pool) => {
        info!("connected to db");
        Ok(pool)
      }
      Err(e) => {
        error!("failed to connect to db: {:?}", e);
        Err(())
      }
    }
  }

  pub async fn migrate(&self) {
    info!("migrating db {}", cfg!(debug_assertions));

    // Migrate the database
    let migrations = if !cfg!(debug_assertions) {
      // Productions migrations dir
      let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
      Path::new(&crate_dir).join("./migrations/prod")
    } else {
      // Development migrations dir
      let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
      Path::new(&crate_dir).join("./migrations/dev")
    };

    sqlx::migrate::Migrator::new(migrations)
      .await
      .unwrap()
      .run(&<Option<SqlitePool> as Clone>::clone(&self.pool).unwrap())
      .await
      .unwrap();
  }

  pub async fn get_logs(&self) {}

  pub async fn get_cmds(&self) {}

  pub async fn push_log() {
    // TODO: implement log push
  }
  pub async fn push_cmd() {
    // TODO: implement command push
  }
}

// TODO: need to implement way to name clients and identify them over time
struct LogEntry {
  client: String,
  message: String,
  time_logged: chrono::NaiveDateTime,
}
