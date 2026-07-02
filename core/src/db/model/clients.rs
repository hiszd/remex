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
  pub blocked: bool,
  pub last_seen: Option<surrealdb::types::Datetime>,
  pub connection_history: Vec<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone, SurrealValue)]
pub struct Client {
  pub id: surrealdb::types::RecordId,
  pub client_name: String,
  pub secret: String,
  pub hardware_hash: String,
  pub blocked: bool,
  pub last_seen: Option<surrealdb::types::Datetime>,
  pub connection_history: Vec<serde_json::Value>,
  pub created_at: surrealdb::types::Datetime,
  pub updated_at: surrealdb::types::Datetime,
}

impl Client {
  pub async fn migrate(db: &Surreal<Any>) -> Result<(), DbError> {
    // Initial table creation (IF NOT EXISTS — only runs once)
    db.query(
      r"
        USE NS remex DB remex;
        DEFINE TABLE IF NOT EXISTS client SCHEMAFULL
          PERMISSIONS FOR select WHERE id = $auth.id OR $auth.id INSIDE (SELECT VALUE id FROM user),
                    FOR update WHERE id = $auth.id,
                    FOR create FULL,
                    FOR delete NONE;
        DEFINE FIELD IF NOT EXISTS client_name ON TABLE client TYPE string;
        DEFINE FIELD IF NOT EXISTS secret ON TABLE client TYPE string VALUE crypto::argon2::generate($value);
        DEFINE FIELD IF NOT EXISTS hardware_hash ON TABLE client TYPE string;
        DEFINE FIELD IF NOT EXISTS blocked ON TABLE client TYPE bool DEFAULT false;
        DEFINE FIELD IF NOT EXISTS created_at ON TABLE client TYPE datetime DEFAULT time::now() READONLY;
        DEFINE FIELD IF NOT EXISTS updated_at ON TABLE client TYPE datetime VALUE time::now() READONLY;

        DEFINE FIELD IF NOT EXISTS last_seen ON TABLE client TYPE option<datetime>;
        DEFINE FIELD IF NOT EXISTS connection_history ON TABLE client TYPE array<object> DEFAULT [];
        DEFINE EVENT IF NOT EXISTS trim_client_connection_history ON TABLE client
        WHEN $event = 'UPDATE'
        THEN {
          IF array::len($after.connection_history) > 100 THEN
            UPDATE $this.id SET connection_history = $after.connection_history[
              math::max(0, array::len($after.connection_history) - 100)..
            ];
          END;
        };

        DEFINE INDEX IF NOT EXISTS idx_hardware_hash ON TABLE client COLUMNS hardware_hash UNIQUE;

          DEFINE ACCESS IF NOT EXISTS endpoint_access ON DATABASE TYPE RECORD
            SIGNUP {
              LET $tok = (SELECT * FROM enrollment_token WHERE token_hash = crypto::sha256($enrollment_token) AND valid = true AND (expires_at = NONE OR expires_at > time::now()) LIMIT 1)[0];
              IF $tok = NONE {
                THROW 'Invalid or expired enrollment token'
              } ELSE {
                LET $cl = (CREATE client CONTENT {
                  client_name: $client_name,
                  secret: $secret,
                  hardware_hash: $hardware_hash,
                  blocked: false
                })[0];
                IF $tok.single_use = true {
                  UPDATE $tok.id SET valid = false, usage_history += { client_id: $cl.id, used_at: time::now() };
                  RETURN $cl
                } ELSE {
                  UPDATE $tok.id SET usage_history += { client_id: $cl.id, used_at: time::now() };
                  RETURN $cl
                }
              }
            }
            SIGNIN (SELECT * FROM client WHERE hardware_hash = $hardware_hash AND crypto::argon2::compare(secret, $secret) AND blocked != true)
            DURATION FOR TOKEN 1d;
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
      blocked: data.blocked,
      last_seen: data.last_seen,
      connection_history: data.connection_history,
      created_at: surrealdb::types::Datetime::default(),
      updated_at: surrealdb::types::Datetime::default(),
    }
  }
}

use crate::impl_surreal_db_operator;

impl_surreal_db_operator!(pub SurrealClientRepo, Client, ClientData, "client", "remex", "remex");
