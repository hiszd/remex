pub mod dal;
pub mod model;
pub mod schema;

// FIXME: I shouldn't have to seperate these migrations. Look into this.
pub const SERVER_MIGRATIONS: diesel_migrations::EmbeddedMigrations =
  diesel_migrations::embed_migrations!("../migrations/server");
pub const ENDPOINT_MIGRATIONS: diesel_migrations::EmbeddedMigrations =
  diesel_migrations::embed_migrations!("../migrations/endpoint");

#[derive(Debug, Clone, Copy)]
pub enum ConnectionType {
  Sqlite,
  Postgres,
}

impl From<&str> for ConnectionType {
  fn from(value: &str) -> ConnectionType {
    match value {
      "sqlite" => ConnectionType::Sqlite,
      "postgres" => ConnectionType::Postgres,
      _ => panic!("Unknown connection type"),
    }
  }
}

pub fn establish_connection_postgres() -> diesel::PgConnection {
  dotenvy::dotenv().ok();

  let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
  <diesel::PgConnection as diesel::Connection>::establish(&database_url)
    .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}
pub fn establish_connection_sqlite() -> diesel::SqliteConnection {
  dotenvy::dotenv().ok();

  let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
  <diesel::SqliteConnection as diesel::Connection>::establish(&database_url)
    .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
}

pub async fn migrate(dbtype: ConnectionType) -> anyhow::Result<()> {
  use diesel_migrations::MigrationHarness;
  tracing::info!("migrating db");
  // Migrate the database
  match dbtype {
    ConnectionType::Postgres => {
      let mut c = establish_connection_postgres();
      if let Err(e) = c.run_pending_migrations(SERVER_MIGRATIONS) {
        anyhow::bail!("{}", e)
      };
      Ok(())
    }
    ConnectionType::Sqlite => {
      let mut c = establish_connection_sqlite();
      match c.run_pending_migrations(ENDPOINT_MIGRATIONS) {
        Err(e) => anyhow::bail!("{}", e),
        _ => Ok(()),
      }
    }
  }
}
