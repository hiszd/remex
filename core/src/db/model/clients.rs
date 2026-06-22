use serde::{
  Deserialize,
  Serialize,
};
use surrealdb::{
  engine::any::Any,
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

impl From<(String, ClientData)> for Client {
  fn from((id, data): (String, ClientData)) -> Self {
    Client {
      id: surrealdb::types::RecordId::new("client", id.as_str()),
      client_name: data.client_name,
      secret: data.secret,
      hardware_hash: data.hardware_hash,
      last_seen: data.last_seen,
      connection_history: data.connection_history,
      created_at: surrealdb::types::Datetime::default(),
      updated_at: surrealdb::types::Datetime::default(),
    }
  }
}

use crate::impl_surreal_db_operator;

impl_surreal_db_operator!(pub SurrealClientRepo, Client, ClientData, "client", "remex", "remex");
