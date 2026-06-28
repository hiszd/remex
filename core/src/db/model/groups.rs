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
pub struct GroupData {
  pub group_name: String,
  pub created_at: surrealdb::types::Datetime,
  pub updated_at: surrealdb::types::Datetime,
  pub members: Vec<surrealdb::types::RecordId>,
}

#[derive(Serialize, Deserialize, Clone, SurrealValue)]
pub struct Group {
  pub id: surrealdb::types::RecordId,
  pub group_name: String,
  pub created_at: surrealdb::types::Datetime,
  pub updated_at: surrealdb::types::Datetime,
  pub members: Vec<surrealdb::types::RecordId>,
}

impl Group {
  pub async fn migrate(db: &Surreal<Any>) -> Result<(), DbError> {
    // this will create the table in the database if it does not already exist
    db.query(
      r"
        USE NS remex DB remex;
        DEFINE TABLE IF NOT EXISTS group SCHEMAFULL PERMISSIONS
          FOR select FULL,
          FOR create WHERE $auth.id INSIDE (SELECT VALUE id FROM user),
          FOR update WHERE $auth.id INSIDE (SELECT VALUE id FROM user),
          FOR delete WHERE $auth.id INSIDE (SELECT VALUE id FROM user);
        DEFINE FIELD IF NOT EXISTS group_name ON TABLE group TYPE string;

        DEFINE FIELD IF NOT EXISTS members ON TABLE group TYPE array<record<client>> DEFAULT [];

        DEFINE FIELD IF NOT EXISTS created_at ON TABLE group TYPE datetime DEFAULT time::now() READONLY;
        DEFINE FIELD IF NOT EXISTS updated_at ON TABLE group TYPE datetime VALUE time::now() READONLY;

        DEFINE EVENT IF NOT EXISTS audit_group ON TABLE group
        WHEN $event IN ['CREATE', 'UPDATE', 'DELETE']
        THEN {
          CREATE audit_log SET
            table_name = 'group',
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

impl From<(String, GroupData)> for Group {
  fn from((id, data): (String, GroupData)) -> Self {
    Group {
      id: surrealdb::types::RecordId::new("group", id.as_str()),
      group_name: data.group_name,
      created_at: data.created_at,
      updated_at: data.updated_at,
      members: data.members,
    }
  }
}

use crate::impl_surreal_db_operator;

impl_surreal_db_operator!(pub SurrealGroupRepo, Group, GroupData, "group", "remex", "remex");
