use diesel::{
  QueryDsl,
  RunQueryDsl,
};
use serde::{
  Deserialize,
  Serialize,
};

use crate::db::{
  model,
  schema,
};

#[derive(Serialize, Deserialize, Clone)]
pub struct Group {
  pub id: String,
  pub group_name: String,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

impl Group {
  pub fn new(id: String, group_name: String) -> Self {
    Group {
      id,
      group_name,
      created_at: chrono::Utc::now().naive_utc(),
      updated_at: chrono::Utc::now().naive_utc(),
    }
  }
}

impl From<model::server::groups::GroupSRV> for Group {
  fn from(val: model::server::groups::GroupSRV) -> Self {
    Group {
      id: val.id,
      group_name: val.group_name,
      created_at: val.created_at,
      updated_at: val.updated_at,
    }
  }
}

impl super::SrvDbOperator for Group {
  fn create_srv(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error> {
    use model::server::groups::{
      GroupSRV,
      NewGroupSRV,
    };
    use schema::server::groups;
    match diesel::insert_into(groups::table)
      .values(NewGroupSRV::from(self.clone()))
      .get_result::<GroupSRV>(conn)
    {
      Ok(group) => Ok(group.into()),
      Err(e) => Err(e),
    }
  }
  fn update_srv(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error> {
    use model::server::groups::{
      GroupSRV,
      UpdateGroupSRV,
    };
    use schema::server::groups;
    match diesel::update(groups::table.find(self.id.clone()))
      .set(UpdateGroupSRV::from(self.clone()))
      .get_result::<GroupSRV>(conn)
    {
      Ok(group) => Ok(group.into()),
      Err(e) => Err(e),
    }
  }
  fn delete_srv(&self, conn: &mut diesel::PgConnection) -> Result<(), diesel::result::Error> {
    use schema::server::groups;
    match diesel::delete(groups::table.find(self.id.clone())).execute(conn) {
      Ok(_) => Ok(()),
      Err(e) => Err(e),
    }
  }
  fn read_srv(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error> {
    use model::server::groups::GroupSRV;
    use schema::server::groups;
    match groups::table
      .find(self.id.clone())
      .get_result::<GroupSRV>(conn)
    {
      Ok(group) => Ok(group.into()),
      Err(e) => Err(e),
    }
  }
  fn upsert_srv(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error> {
    use model::server::groups::{
      GroupSRV,
      NewGroupSRV,
      UpsertGroupSRV,
    };
    use schema::server::groups;
    diesel::insert_into(groups::table)
      .values(NewGroupSRV::from(self.clone()))
      .on_conflict(groups::id)
      .do_update()
      .set(UpsertGroupSRV::from(self.clone()))
      .execute(conn)?;
    groups::table
      .find(self.id.clone())
      .get_result::<GroupSRV>(conn)
      .map(|group| group.into())
  }
}
