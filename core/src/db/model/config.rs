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
pub struct ConfigData {
  pub group_name: String,
  pub created_at: surrealdb::types::Datetime,
  pub updated_at: surrealdb::types::Datetime,
}

#[derive(Serialize, Deserialize, Clone, SurrealValue)]
pub struct Config {
  pub id: surrealdb::types::RecordId,
  pub group_name: String,
  pub created_at: surrealdb::types::Datetime,
  pub updated_at: surrealdb::types::Datetime,
}

impl Config {
  pub async fn migrate(db: &Surreal<Any>) -> Result<(), DbError> {
    // this will create the table in the database if it does not already exist
    db.query(
      r"
        USE NS remex DB config;

        DEFINE TABLE IF NOT EXISTS config SCHEMALESS
          PERMISSIONS FOR select FULL FOR create FULL FOR update FULL FOR delete FULL;

        DEFINE TABLE IF NOT EXISTS global_config SCHEMAFULL
          PERMISSIONS FOR select FULL FOR create FULL FOR update FULL FOR delete FULL;
        DEFINE FIELD IF NOT EXISTS setting_key ON TABLE global_config TYPE string;
        DEFINE FIELD IF NOT EXISTS setting_value ON TABLE global_config TYPE object FLEXIBLE;
        DEFINE FIELD IF NOT EXISTS created_at ON TABLE global_config TYPE datetime DEFAULT time::now() READONLY;
        DEFINE FIELD IF NOT EXISTS updated_at ON TABLE global_config TYPE datetime VALUE time::now() READONLY;

        DEFINE TABLE IF NOT EXISTS user_config SCHEMAFULL
          PERMISSIONS FOR select FULL FOR create FULL FOR update FULL FOR delete FULL;
        DEFINE FIELD IF NOT EXISTS user_id ON TABLE user_config TYPE record<user>;
        DEFINE FIELD IF NOT EXISTS setting_key ON TABLE user_config TYPE string;
        DEFINE FIELD IF NOT EXISTS setting_value ON TABLE user_config TYPE object FLEXIBLE;
        DEFINE FIELD IF NOT EXISTS created_at ON TABLE user_config TYPE datetime DEFAULT time::now() READONLY;
        DEFINE FIELD IF NOT EXISTS updated_at ON TABLE user_config TYPE datetime VALUE time::now() READONLY;
        DEFINE INDEX IF NOT EXISTS idx_user_id ON TABLE user_config COLUMNS user_id;

        DEFINE FIELD IF NOT EXISTS created_at ON TABLE config TYPE datetime DEFAULT time::now() READONLY;
        DEFINE FIELD IF NOT EXISTS updated_at ON TABLE config TYPE datetime VALUE time::now() READONLY;
      ",
    )
    .await?
    .check()?;
    Ok(())
  }
}

impl From<(String, ConfigData)> for Config {
  fn from((id, data): (String, ConfigData)) -> Self {
    Config {
      id: surrealdb::types::RecordId::new("config", id.as_str()),
      group_name: data.group_name,
      created_at: data.created_at,
      updated_at: data.updated_at,
    }
  }
}

use crate::impl_surreal_db_operator;

impl_surreal_db_operator!(pub SurrealConfigRepo, Config, ConfigData, "config", "remex", "config");
