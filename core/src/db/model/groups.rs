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
pub struct GroupData {
  pub group_name: String,
  pub created_at: surrealdb::types::Datetime,
  pub updated_at: surrealdb::types::Datetime,
}

#[derive(Serialize, Deserialize, Clone, SurrealValue)]
pub struct Group {
  pub id: surrealdb::types::RecordId,
  pub group_name: String,
  pub created_at: surrealdb::types::Datetime,
  pub updated_at: surrealdb::types::Datetime,
}

impl Group {
  pub async fn migrate(db: &Surreal<Any>) -> Result<(), DbError> {
    // this will create the table in the database if it does not already exist
    db.query(
      r"
        USE NS remex DB remex;
        DEFINE TABLE IF NOT EXISTS group SCHEMAFULL;
        DEFINE FIELD IF NOT EXISTS group_name ON TABLE group TYPE string;

        DEFINE FIELD IF NOT EXISTS members ON TABLE group TYPE array<record<client>> DEFAULT [];

        DEFINE FIELD IF NOT EXISTS created_at ON TABLE group TYPE datetime DEFAULT time::now() READONLY;
        DEFINE FIELD IF NOT EXISTS updated_at ON TABLE group TYPE datetime VALUE time::now() READONLY;
      ",
    )
    .await?
    .check()?;
    Ok(())
  }
}

impl crate::db::DbOperator<Group, GroupData> for Group {
  async fn create(obj: GroupData, db: &Surreal<Db>) -> Result<Option<Group>, DbError> {
    let s: Option<Group> = db
      .query(
        r"
        USE NS remex DB remex;
        CREATE group CONTENT $data
      ",
      )
      .bind(("data", obj))
      .await?
      .check()?
      .take(1)?;
    if let Some(group) = s {
      Ok(Some(group.clone()))
    } else {
      Err(DbError::OperationFailed("Failed to create group".to_string()))
    }
  }
  async fn read(id: String, db: &Surreal<Db>) -> Result<Option<Group>, DbError> {
    Ok(
      db.query("USE NS remex DB remex; SELECT * FROM group WHERE id = $id;")
        .bind(("id", id))
        .await?
        .check()?
        .take(1)?,
    )
  }
  async fn push(&mut self, db: &Surreal<Db>) -> Result<(), DbError> {
    let s: Option<Group> = db
      .query("USE NS remex DB remex; UPSERT $id CONTENT $data")
      .bind(("id", self.id.to_sql()))
      .bind(("data", self.clone()))
      .await?
      .check()?
      .take(1)?;
    if let Some(group) = s {
      *self = group;
      Ok(())
    } else {
      Err(DbError::OperationFailed("Failed to upsert group".to_string()))
    }
  }

  async fn pull(&self, db: &Surreal<Db>) -> Result<Option<Group>, DbError> {
    Ok(
      db.query("USE NS remex DB remex; SELECT * FROM $id;")
        .bind(("id", self.id.to_sql()))
        .await?
        .check()?
        .take(1)?,
    )
  }

  async fn delete(&self, db: &Surreal<Db>) -> Result<(), DbError> {
    db.query("USE NS remex DB remex; DELETE $id;")
      .bind(("id", self.id.to_sql()))
      .await?
      .check()?;
    Ok(())
  }
}
