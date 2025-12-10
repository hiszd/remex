pub struct Client {
  pub id: String,
  pub name: String,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub enum Pools {
  Sqlite(sqlx::SqlitePool),
  Postgres(sqlx::PgPool),
}
