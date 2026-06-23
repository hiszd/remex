use actix::MessageResponse;
use async_trait::async_trait;
use serde::{
  Deserialize,
  Serialize,
};
use surrealdb::{
  engine::any::Any,
  types::{
    Datetime,
    RecordId,
    SurrealValue,
    ToSql,
  },
  Surreal,
};

pub mod adapters;
pub mod connection;
pub mod model;

#[derive(thiserror::Error, Debug)]
pub enum DbError {
  #[error(transparent)]
  SurrealDb(#[from] surrealdb::Error),
  #[error("Operation failed: {0}")]
  OperationFailed(String),
  #[error("No Database Connection")]
  NoDatabaseConnection(String),
}

/// Object-safe database operator seam.
///
/// Methods take `&self` so the trait can be used as a trait object
/// (`Box<dyn DbOperator<Record = X, Input = Y>>`), allowing callers
/// to swap in an in-memory adapter for testing.
#[async_trait]
pub trait DbOperator: Send + Sync {
  type Record: Send + Sync + 'static;
  type Input: Send + Sync + 'static;

  /// Insert a new record from input data. Returns the created record (with id).
  async fn create(&self, input: Self::Input) -> Result<Self::Record, DbError>;
  /// Fetch a record by string id. Returns `None` if not found.
  async fn read(&self, id: &str) -> Result<Option<Self::Record>, DbError>;
  /// Replace a record's content. Returns the updated record.
  async fn update(&self, id: &str, input: Self::Input) -> Result<Self::Record, DbError>;
  /// List all records.
  async fn list(&self) -> Result<Vec<Self::Record>, DbError>;
  /// Delete a record by string id.
  async fn delete(&self, id: &str) -> Result<(), DbError>;
}

pub async fn migrate(db: &Surreal<Any>) -> Result<(), DbError> {
  let remex = db.clone();
  remex.use_ns("remex").use_db("remex").await?;
  model::clients::Client::migrate(&remex).await?;
  model::executions::Execution::migrate(&remex).await?;
  model::groups::Group::migrate(&remex).await?;
  model::jobs::Job::migrate(&remex).await?;
  model::users::User::migrate(&remex).await?;
  model::refresh_tokens::RefreshToken::migrate(&remex).await?;
  model::audit::AuditLog::migrate(&remex).await?;
  let config = db.clone();
  config.use_ns("remex").use_db("config").await?;
  model::config::Config::migrate(&config).await?;
  Ok(())
}

pub async fn get_endpoint_bearer_token(
  id: RecordId,
  db: &Surreal<Any>,
) -> Result<Option<BearerGrantResponse>, DbError> {
  let mut res = db
    .query(format!("ACCESS endpoint GRANT FOR RECORD {};", id.to_sql()))
    .await?;
  let token: Option<BearerGrantResponse> = res.take(0)?;
  Ok(token)
}

#[derive(Serialize, Deserialize, Debug, SurrealValue, Clone, MessageResponse)]
pub struct BearerGrantResponse {
  pub ac: String,
  pub creation: Datetime,
  pub expiration: Datetime,
  pub grant: GrantDetails,
  pub id: String,
  pub revocation: Option<Datetime>,
  pub subject: Subject,
  #[serde(rename = "type")]
  pub grant_type: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, SurrealValue, Clone)]
pub struct GrantDetails {
  pub id: String,
  pub key: String,
}

#[derive(Serialize, Deserialize, Debug, SurrealValue, Clone)]
pub struct Subject {
  pub record: RecordId,
}

#[derive(Serialize, Deserialize, Debug, SurrealValue, Clone)]
pub struct BearerToken {
  pub key: String,
}
