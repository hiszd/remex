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
  pub password: String, // argon2 hashed
  pub created_at: surrealdb::types::Datetime,
  pub updated_at: surrealdb::types::Datetime,
}

impl User {
  pub async fn migrate(db: &Surreal<Any>) -> Result<(), DbError> {
    db.query(
      r"
        USE NS remex DB remex;
        DEFINE TABLE IF NOT EXISTS user SCHEMAFULL PERMISSIONS FOR select FULL;
        DEFINE FIELD IF NOT EXISTS username ON TABLE user TYPE string;
        DEFINE FIELD IF NOT EXISTS email ON TABLE user TYPE string;
        DEFINE FIELD IF NOT EXISTS password ON TABLE user TYPE string VALUE crypto::argon2::generate($value);
        DEFINE FIELD IF NOT EXISTS created_at ON TABLE user TYPE datetime DEFAULT time::now() READONLY;
        DEFINE FIELD IF NOT EXISTS updated_at ON TABLE user TYPE datetime VALUE time::now() READONLY;
        DEFINE INDEX IF NOT EXISTS idx_email ON TABLE user COLUMNS email UNIQUE;

        DEFINE ACCESS IF NOT EXISTS configurator_access ON DATABASE TYPE RECORD
          SIGNUP (CREATE user SET username = $username, email = $email, password = $password)
          SIGNIN (SELECT * FROM user WHERE email = $email AND crypto::argon2::compare(password, $password))
          DURATION FOR TOKEN 1h;
      ",
    )
    .await?
    .check()?;
    Ok(())
  }
}
