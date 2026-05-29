use serde::{
  Deserialize,
  Serialize,
};
use surrealdb::{
  engine::{
    any::Any,
    local::Db,
  },
  types::ToSql,
  Surreal,
};

use crate::db::{
  DbError,
  SurrealValue,
};

#[derive(Serialize, Deserialize, Clone, SurrealValue)]
pub struct ClientData {
  pub client_name: String,
  pub secret: String,
  pub hardware_hash: String,
  pub last_seen: Option<surrealdb::types::Datetime>,
  pub connection_history: Vec<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone, SurrealValue)]
pub struct Client {
  pub id: surrealdb::types::RecordId,
  pub client_name: String,
  pub secret: String,
  pub hardware_hash: String,
  pub last_seen: Option<surrealdb::types::Datetime>,
  pub connection_history: Vec<serde_json::Value>,
  pub created_at: surrealdb::types::Datetime,
  pub updated_at: surrealdb::types::Datetime,
}

impl Client {
  pub async fn migrate(db: &Surreal<Any>) -> Result<(), DbError> {
    // this will create the table in the database if it does not already exist
    db.query(
      r"
        USE NS remex DB remex;
        DEFINE TABLE IF NOT EXISTS client SCHEMAFULL
          PERMISSIONS FOR select FULL;
        DEFINE FIELD IF NOT EXISTS client_name ON TABLE client TYPE string;
        DEFINE FIELD IF NOT EXISTS secret ON TABLE client TYPE string VALUE crypto::argon2::generate($value);
        DEFINE FIELD IF NOT EXISTS hardware_hash ON TABLE client TYPE string;
        DEFINE FIELD IF NOT EXISTS created_at ON TABLE client TYPE datetime DEFAULT time::now() READONLY;
        DEFINE FIELD IF NOT EXISTS updated_at ON TABLE client TYPE datetime VALUE time::now() READONLY;

        DEFINE FIELD IF NOT EXISTS last_seen ON TABLE client TYPE option<datetime>;
        DEFINE FIELD IF NOT EXISTS connection_history ON TABLE client TYPE array<object> DEFAULT [];
        // TODO: Limit connection_history to last 100 entries via EVENT or application code

        DEFINE INDEX IF NOT EXISTS idx_hardware_hash ON TABLE client COLUMNS hardware_hash UNIQUE;

        DEFINE ACCESS IF NOT EXISTS endpoint ON DATABASE TYPE BEARER FOR RECORD DURATION FOR GRANT 1d;

        DEFINE EVENT IF NOT EXISTS audit_client ON TABLE client
        WHEN $event IN ['CREATE', 'UPDATE', 'DELETE']
        THEN {
          CREATE audit_log SET
            table_name = 'client',
            record_id = $after.id ?? $before.id,
            action = $event,
            before_snapshot = IF $event = 'CREATE' THEN {} ELSE $before END,
            after_snapshot = IF $event = 'DELETE' THEN {} ELSE $after END,
            changed_by = $auth.id;
        };
      ",
    )
    .await?
    .check()?;
    Ok(())
  }
}

impl crate::db::DbOperator<Client, ClientData> for Client {
  async fn create(obj: ClientData, db: &Surreal<Db>) -> Result<Option<Client>, DbError> {
    let s: Option<Client> = db
      .query(
        r"
        USE NS remex DB remex;
        CREATE client CONTENT $data
      ",
      )
      .bind(("data", obj))
      .await?
      .check()?
      .take(1)?;
    if let Some(client) = s {
      Ok(Some(client.clone()))
    } else {
      Err(DbError::OperationFailed("Failed to create client".to_string()))
    }
  }
  async fn read(id: String, db: &Surreal<Db>) -> Result<Option<Client>, DbError> {
    Ok(
      db.query("USE NS remex DB remex; SELECT * FROM client WHERE id = $id;")
        .bind(("id", id))
        .await?
        .check()?
        .take(1)?,
    )
  }
  async fn push(&mut self, db: &Surreal<Db>) -> Result<(), DbError> {
    let s: Option<Client> = db
      .query("USE NS remex DB remex; UPSERT $id CONTENT $data")
      .bind(("id", self.id.to_sql()))
      .bind(("data", self.clone()))
      .await?
      .check()?
      .take(1)?;
    if let Some(client) = s {
      *self = client;
      Ok(())
    } else {
      Err(DbError::OperationFailed("Failed to upsert client".to_string()))
    }
  }

  async fn pull(&self, db: &Surreal<Db>) -> Result<Option<Client>, DbError> {
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
