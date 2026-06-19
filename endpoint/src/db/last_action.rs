use remex_core::db::DbError;
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
pub struct LastActionData {
  pub task_name: String,
  pub last_run: surrealdb::types::Datetime,
}

#[derive(Debug, Serialize, Deserialize, SurrealValue, Clone)]
pub struct LastAction {
  pub id: surrealdb::types::RecordId,
  pub task_name: String,
  pub last_run: surrealdb::types::Datetime,
}

impl LastAction {
  pub async fn migrate(db: &Surreal<Db>) -> Result<(), DbError> {
    db.query(
      r"
        USE NS remex DB endpoint;
        DEFINE TABLE IF NOT EXISTS last_action SCHEMAFULL;
        DEFINE FIELD IF NOT EXISTS task_name ON TABLE last_action TYPE string;
        DEFINE FIELD IF NOT EXISTS last_run ON TABLE last_action TYPE datetime;
      ",
    )
    .await?
    .check()?;
    Ok(())
  }

  /// Check if a task has run within the given interval.
  /// Returns true if the task should be skipped (ran recently).
  pub async fn should_skip(db: &Surreal<Db>, task_name: &str, interval_secs: u64) -> Result<bool, DbError> {
    let result: Option<LastAction> = db
      .query(
        "USE NS remex DB endpoint; SELECT * FROM last_action WHERE task_name = $name LIMIT 1;",
      )
      .bind(("name", task_name.to_string()))
      .await?
      .check()?
      .take(1)?;

    match result {
      Some(action) => {
        let now: chrono::DateTime<chrono::Utc> = surrealdb::types::Datetime::now().into();
        let then: chrono::DateTime<chrono::Utc> = action.last_run.into();
        let elapsed = (now - then).num_seconds() as u64;
        Ok(elapsed < interval_secs)
      }
      None => Ok(false),
    }
  }

  /// Record that a task has just run.
  pub async fn record(db: &Surreal<Db>, task_name: &str) -> Result<(), DbError> {
    let now = surrealdb::types::Datetime::now();
    let task = task_name.to_string();
    db.query(
      r"
        USE NS remex DB endpoint;
        LET $existing = (SELECT * FROM last_action WHERE task_name = $name LIMIT 1)[0];
        IF $existing != NONE {
          UPDATE $existing.id SET task_name = $name, last_run = $now;
        } ELSE {
          CREATE last_action CONTENT { task_name: $name, last_run: $now };
        };
      ",
    )
    .bind(("name", task))
    .bind(("now", now))
    .await?
    .check()?;
    Ok(())
  }

  /// Delete records older than 72 hours.
  pub async fn cleanup_old(db: &Surreal<Db>) -> Result<(), DbError> {
    db.query(
      r"
        USE NS remex DB endpoint;
        DELETE last_action WHERE last_run < time::now() - 72h;
      ",
    )
    .await?
    .check()?;
    Ok(())
  }
}
