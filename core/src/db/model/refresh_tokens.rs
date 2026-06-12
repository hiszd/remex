use surrealdb::{
  engine::any::Any,
  Surreal,
};

use crate::db::DbError;

pub struct RefreshToken;

impl RefreshToken {
  pub async fn migrate(db: &Surreal<Any>) -> Result<(), DbError> {
    db.query(
      r"
        USE NS remex DB remex;
        DEFINE TABLE IF NOT EXISTS refresh_token SCHEMAFULL
          PERMISSIONS FOR select WHERE user = $auth.id,
                    FOR create WHERE user = $auth.id,
                    FOR update WHERE user = $auth.id,
                    FOR delete WHERE user = $auth.id;
        DEFINE FIELD IF NOT EXISTS user ON TABLE refresh_token TYPE record<user>;
        DEFINE FIELD IF NOT EXISTS token ON TABLE refresh_token TYPE string;
        DEFINE FIELD IF NOT EXISTS expires ON TABLE refresh_token TYPE datetime;
        DEFINE FIELD IF NOT EXISTS active ON TABLE refresh_token TYPE bool DEFAULT true;
        DEFINE FIELD IF NOT EXISTS revoked_at ON TABLE refresh_token TYPE option<datetime>;
        DEFINE INDEX IF NOT EXISTS unique_token ON TABLE refresh_token FIELDS token UNIQUE;
      ",
    )
    .await?
    .check()?;
    Ok(())
  }
}
