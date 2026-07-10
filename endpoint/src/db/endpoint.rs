use remex_core::{
  db::{
    BearerGrantResponse,
    DbError,
  },
  impl_surreal_db_operator,
};
use serde::{
  Deserialize,
  Serialize,
};
use surrealdb::{
  engine::local::Db,
  types::SurrealValue,
  Surreal,
};

#[derive(Debug, Serialize, Deserialize, SurrealValue, Clone)]
pub struct SessionData {
  pub client_id: Option<String>,
  pub client_name: Option<String>,
  pub hardware_hash: Option<String>,
  pub db_addr: Option<String>,
  pub tkn: Option<BearerGrantResponse>,
  pub secret: Option<String>,
  pub groups: Vec<surrealdb::types::RecordId>,
}

#[derive(Debug, Serialize, Deserialize, SurrealValue, Clone)]
pub struct Session {
  pub id: surrealdb::types::RecordId,
  pub client_id: Option<String>,
  pub client_name: String,
  pub hardware_hash: String,
  pub db_addr: Option<String>,
  pub tkn: Option<BearerGrantResponse>,
  pub secret: Option<String>,
  pub groups: Vec<surrealdb::types::RecordId>,
}

impl From<(String, SessionData)> for Session {
  fn from((id, data): (String, SessionData)) -> Self {
    Session {
      id: surrealdb::types::RecordId::new("session", id.as_str()),
      client_id: data.client_id,
      client_name: data.client_name.unwrap_or_default(),
      hardware_hash: data.hardware_hash.unwrap_or_default(),
      db_addr: data.db_addr,
      tkn: data.tkn,
      secret: data.secret,
      groups: data.groups,
    }
  }
}

impl Session {
  pub fn session_id(&self) -> String {
    match &self.id.key {
      surrealdb::types::RecordIdKey::String(s) => s.clone(),
      _ => panic!("expected string key"),
    }
  }

  pub async fn migrate(db: &Surreal<Db>) -> Result<(), DbError> {
    // this will create the table in the database if it does not already exist
    db.query(
      r"
        USE NS remex DB endpoint;
        DEFINE TABLE IF NOT EXISTS session SCHEMAFULL;
        DEFINE FIELD IF NOT EXISTS client_id ON TABLE session TYPE option<string>;
        DEFINE FIELD IF NOT EXISTS client_name ON TABLE session TYPE string;
        DEFINE FIELD IF NOT EXISTS hardware_hash ON TABLE session TYPE string;
        DEFINE FIELD IF NOT EXISTS db_addr ON TABLE session TYPE option<string>;
        DEFINE FIELD IF NOT EXISTS tkn ON TABLE session TYPE option<object> FLEXIBLE;
        DEFINE FIELD IF NOT EXISTS secret ON TABLE session TYPE option<string>;
        DEFINE FIELD IF NOT EXISTS groups ON TABLE session TYPE array<record<group>>;
      ",
    )
    .await?
    .check()?;
    Ok(())
  }
}

impl_surreal_db_operator!(pub SurrealSessionRepo, Session, SessionData, "session", "remex", "endpoint");
