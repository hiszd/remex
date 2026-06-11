use serde::{
  Deserialize,
  Serialize,
};
use surrealdb::{
  engine::any::Any,
  types::SurrealValue,
  Surreal,
};

use crate::db::DbError;

#[derive(Serialize, Deserialize, Clone, SurrealValue)]
pub struct UserData {
  pub username: String,
  pub email: String,
  pub password: String, // argon2 hashed
}

#[derive(Debug, Serialize, Deserialize, Clone, SurrealValue)]
pub struct User {
  pub id: surrealdb::types::RecordId,
  pub username: String,
  pub email: String,
  pub password: String,
  pub created_at: chrono::DateTime<chrono::Utc>,
  pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl User {
  pub async fn migrate(db: &Surreal<Any>) -> Result<(), DbError> {
    let query = r#"
      USE NS remex DB remex;
      DEFINE TABLE IF NOT EXISTS user SCHEMAFULL;
      DEFINE FIELD IF NOT EXISTS username ON TABLE user TYPE string;
      DEFINE FIELD IF NOT EXISTS email ON TABLE user TYPE string;
      DEFINE FIELD IF NOT EXISTS password ON TABLE user TYPE string VALUE crypto::argon2::generate($value);
      DEFINE FIELD IF NOT EXISTS created_at ON TABLE user TYPE datetime DEFAULT time::now() READONLY;
      DEFINE FIELD IF NOT EXISTS updated_at ON TABLE user TYPE datetime VALUE time::now() READONLY;
      DEFINE INDEX IF NOT EXISTS idx_email ON TABLE user COLUMNS email UNIQUE;

      DEFINE ACCESS IF NOT EXISTS configurator_access ON DATABASE TYPE RECORD
        SIGNUP (CREATE user SET username = $username, email = $email, password = $password)
        SIGNIN (
          IF $email != NONE AND $pass != NONE {
            SELECT * FROM user WHERE email = $email AND crypto::argon2::compare(password, $pass)
          } ELSE IF $refresh_token != NONE {
            SELECT * FROM user WHERE id = (
              SELECT VALUE user FROM refresh_token
              WHERE token = $refresh_token
                AND expires > time::now()
                AND (
                  active = true
                  OR (active = false AND revoked_at > time::now() - 1m)
                )
            )[0]
          } ELSE {
            THROW "Authentication failed: Invalid credentials or expired session token."
          }
        )
        DURATION FOR TOKEN 15m;
    "#;
    tracing::info!("Running user migration");
    let result = db.query(query).await?;
    tracing::info!("User migration result: {:?}", result);
    result.check()?;
    Ok(())
  }
}
