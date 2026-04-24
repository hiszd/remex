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
    SurrealValue,
    ToSql,
  },
  Surreal,
};

use crate::db::DbError;

#[derive(Serialize, Deserialize, Clone, SurrealValue)]
pub struct ConfigData {
  pub group_name: String,
  pub created_at: surrealdb::types::Datetime,
  pub updated_at: surrealdb::types::Datetime,
}

#[derive(Serialize, Deserialize, Clone, SurrealValue)]
pub struct Config {
  pub id: surrealdb::types::RecordId,
  pub group_name: String,
  pub created_at: surrealdb::types::Datetime,
  pub updated_at: surrealdb::types::Datetime,
}

impl Config {
  pub async fn migrate(db: &Surreal<Any>) -> Result<(), DbError> {
    // this will create the table in the database if it does not already exist
    db.query(
      r"
        USE NS remex DB config;
        DEFINE TABLE IF NOT EXISTS config SCHEMALESS;

        DEFINE FIELD IF NOT EXISTS created_at ON TABLE config TYPE datetime DEFAULT time::now() READONLY;
        DEFINE FIELD IF NOT EXISTS updated_at ON TABLE config TYPE datetime VALUE time::now() READONLY;
      ",
    )
    .await?
    .check()?;
    Ok(())
  }
}

impl crate::db::DbOperator<Config, ConfigData> for Config {
  async fn create(obj: ConfigData, db: &Surreal<Db>) -> Result<Option<Config>, DbError> {
    let s: Option<Config> = db
      .query(
        r"
        USE NS remex DB config;
        CREATE config CONTENT $data
      ",
      )
      .bind(("data", obj))
      .await?
      .check()?
      .take(1)?;
    if let Some(config) = s {
      Ok(Some(config.clone()))
    } else {
      Err(DbError::OperationFailed("Failed to create config".to_string()))
    }
  }
  async fn read(id: String, db: &Surreal<Db>) -> Result<Option<Config>, DbError> {
    Ok(
      db.query("USE NS remex DB config; SELECT * FROM config WHERE id = $id;")
        .bind(("id", id))
        .await?
        .check()?
        .take(1)?,
    )
  }
  async fn push(&mut self, db: &Surreal<Db>) -> Result<(), DbError> {
    let s: Option<Config> = db
      .query("USE NS remex DB config; UPSERT $id CONTENT $data")
      .bind(("id", self.id.to_sql()))
      .bind(("data", self.clone()))
      .await?
      .check()?
      .take(1)?;
    if let Some(config) = s {
      *self = config;
      Ok(())
    } else {
      Err(DbError::OperationFailed("Failed to upsert config".to_string()))
    }
  }

  async fn pull(&self, db: &Surreal<Db>) -> Result<Option<Config>, DbError> {
    Ok(
      db.query("USE NS remex DB config; SELECT * FROM $id;")
        .bind(("id", self.id.to_sql()))
        .await?
        .check()?
        .take(1)?,
    )
  }

  async fn delete(&self, db: &Surreal<Db>) -> Result<(), DbError> {
    db.query("USE NS remex DB config; DELETE $id;")
      .bind(("id", self.id.to_sql()))
      .await?
      .check()?;
    Ok(())
  }
}
