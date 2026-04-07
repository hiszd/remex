use remex_core::db::{
  BearerGrantResponse,
  DbError,
};
use serde::{
  Deserialize,
  Serialize,
};
use surrealdb::{
  engine::local::Db,
  types::{
    SurrealValue,
    ToSql,
  },
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
}

impl Session {
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
        DEFINE FIELD IF NOT EXISTS tkn ON TABLE session TYPE any;
        DEFINE FIELD IF NOT EXISTS secret ON TABLE session TYPE option<string>;
      ",
    )
    .await?
    .check()?;
    Ok(())
  }
}

impl remex_core::db::DbOperator<Session, SessionData> for Session {
  async fn create(obj: SessionData, db: &Surreal<Db>) -> Result<Option<Session>, DbError> {
    let s: Option<Session> = db
      .query(
        r"
        USE NS remex DB endpoint;
        CREATE session CONTENT $data;
      ",
      )
      .bind(("data", obj))
      .await?
      .check()?
      .take(1)?;
    if let Some(session) = s {
      Ok(Some(session.clone()))
    } else {
      Err(DbError::OperationFailed("Failed to create session".to_string()))
    }
  }
  async fn read(id: String, db: &Surreal<Db>) -> Result<Option<Session>, DbError> {
    Ok(
      db.query("USE NS remex DB endpoint; SELECT * FROM session WHERE id = $id;")
        .bind(("id", id))
        .await?
        .check()?
        .take(1)?,
    )
  }
  async fn push(&mut self, db: &Surreal<Db>) -> Result<(), DbError> {
    tracing::info!("Pushing session: {}", serde_json::to_string_pretty(self).unwrap());
    let s: Option<Session> = db
      .query(format!(
        "USE NS remex DB endpoint; UPSERT session:{} CONTENT $data;",
        self.id.key.to_sql()
      ))
      .bind(("data", self.clone()))
      .await?
      .check()?
      .take(1)?;
    if let Some(session) = s {
      *self = session.clone();
      Ok(())
    } else {
      Err(DbError::OperationFailed("Failed to upsert session".to_string()))
    }
  }

  async fn pull(&self, db: &Surreal<Db>) -> Result<Option<Session>, DbError> {
    Ok(
      db.query("USE NS remex DB endpoint; SELECT * FROM session WHERE id = $id;")
        .bind(("id", self.id.key.clone()))
        .await?
        .check()?
        .take(1)?,
    )
  }

  async fn delete(&self, db: &Surreal<Db>) -> Result<(), DbError> {
    db.query("USE NS remex DB endpoint; DELETE session WHERE id = $id;")
      .bind(("id", self.id.key.clone()))
      .await?
      .check()?;
    Ok(())
  }
}

impl From<crate::Context> for SessionData {
  fn from(ctx: crate::Context) -> Self {
    SessionData {
      client_id: match ctx.id {
        Some(id) => Some(id.to_sql()),
        None => None,
      },
      client_name: Some(ctx.name),
      hardware_hash: Some(ctx.hardware_hash),
      db_addr: None,
      tkn: None,
      secret: None,
    }
  }
}

impl From<Session> for crate::Context {
  fn from(ses: Session) -> Self {
    crate::Context {
      id: match ses.client_id {
        Some(id) => Some(surrealdb::types::RecordId::parse_simple(&id).unwrap()),
        None => None,
      },
      name: ses.client_name.clone(),
      hardware_hash: ses.hardware_hash.clone(),
      authenticated: false,
      token: ses.tkn,
      secret: ses.secret,
    }
  }
}
