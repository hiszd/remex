use actix::MessageResponse;
use serde::{
  Deserialize,
  Serialize,
};
use surrealdb::{
  engine::{
    any::Any,
    local::Db,
  },
  types::{
    Datetime,
    RecordId,
    SurrealValue,
    ToSql,
  },
  Surreal,
};

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

/// Trait for database operations
///
/// The ReturnType is the return type of the operation, which would be the entire record. Id included
/// The InputType is the input type of the operation, or the data to be inserted. Id not included
/// DBOperator<ReturnType, InputType>
///
/// Example:
/// ```rust
/// ```
pub trait DbOperator<T, U>
where
  T: surrealdb::types::SurrealValue,
  U: surrealdb::types::SurrealValue,
{
  fn create(
    obj: U,
    db: &Surreal<Db>,
  ) -> impl std::future::Future<Output = Result<Option<T>, DbError>> + Send;
  fn read(
    id: String,
    db: &Surreal<Db>,
  ) -> impl std::future::Future<Output = Result<Option<T>, DbError>> + Send;
  fn push(
    &mut self,
    db: &Surreal<Db>,
  ) -> impl std::future::Future<Output = Result<(), DbError>> + Send;
  fn pull(
    &self,
    db: &Surreal<Db>,
  ) -> impl std::future::Future<Output = Result<Option<T>, DbError>> + Send;
  fn delete(
    &self,
    db: &Surreal<Db>,
  ) -> impl std::future::Future<Output = Result<(), DbError>> + Send;
}

pub async fn migrate(db: &Surreal<Any>) -> Result<(), DbError> {
  let remex = db.clone();
  remex.use_ns("remex").use_db("remex").await?;
  model::clients::Client::migrate(&remex).await?;
  model::executions::Execution::migrate(&remex).await?;
  model::groups::Group::migrate(&remex).await?;
  model::jobs::Job::migrate(&remex).await?;
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
  pub key: String, // This is the actual token string for the client
}

#[derive(Serialize, Deserialize, Debug, SurrealValue, Clone)]
pub struct Subject {
  pub record: RecordId,
}

#[derive(Serialize, Deserialize, Debug, SurrealValue, Clone)]
pub struct BearerToken {
  pub key: String,
}
