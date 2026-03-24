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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Log {
  pub id: String,
  pub client_id: String,
  pub execution_id: String,
  pub output: String,
  pub command: String,
  pub exit_code: String,
  pub start_time: chrono::NaiveDateTime,
  pub end_time: chrono::NaiveDateTime,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

impl From<model::endpoint::logs::LogCLT> for Log {
  fn from(log: model::endpoint::logs::LogCLT) -> Self {
    Log {
      id: log.id,
      client_id: log.client_id,
      execution_id: log.execution_id,
      output: log.output,
      command: log.command,
      exit_code: log.exit_code,
      start_time: log.start_time,
      end_time: log.end_time,
      created_at: log.created_at,
      updated_at: log.updated_at,
    }
  }
}

impl From<model::server::logs::LogSRV> for Log {
  fn from(log: model::server::logs::LogSRV) -> Self {
    Log {
      id: log.id,
      client_id: log.client_id,
      execution_id: log.execution_id,
      output: log.output,
      command: log.command,
      exit_code: log.exit_code,
      start_time: log.start_time,
      end_time: log.end_time,
      created_at: log.created_at,
      updated_at: log.updated_at,
    }
  }
}

impl super::SrvDbOperator for Log {
  fn create_srv(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error> {
    use model::server::logs::{
      LogSRV,
      NewLogSRV,
    };
    use schema::server::logs;
    match diesel::insert_into(logs::table)
      .values(NewLogSRV::from(self.clone()))
      .get_result::<LogSRV>(conn)
    {
      Ok(log) => Ok(log.into()),
      Err(e) => Err(e),
    }
  }
  fn update_srv(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error> {
    use model::server::logs::{
      LogSRV,
      UpdateLogSRV,
    };
    use schema::server::logs;
    match diesel::update(logs::table.find(self.id.clone()))
      .set(UpdateLogSRV::from(self.clone()))
      .get_result::<LogSRV>(conn)
    {
      Ok(log) => Ok(log.into()),
      Err(e) => Err(e),
    }
  }
  fn delete_srv(&self, conn: &mut diesel::PgConnection) -> Result<(), diesel::result::Error> {
    use schema::server::logs;
    match diesel::delete(logs::table.find(self.id.clone())).execute(conn) {
      Ok(_) => Ok(()),
      Err(e) => Err(e),
    }
  }
  fn read_srv(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error> {
    use model::server::logs::LogSRV;
    use schema::server::logs;
    match logs::table.find(self.id.clone()).get_result::<LogSRV>(conn) {
      Ok(log) => Ok(log.into()),
      Err(e) => Err(e),
    }
  }
  fn upsert_srv(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error> {
    use model::server::logs::{
      LogSRV,
      NewLogSRV,
      UpsertLogSRV,
    };
    use schema::server::logs;
    diesel::insert_into(logs::table)
      .values(NewLogSRV::from(self.clone()))
      .on_conflict(logs::id)
      .do_update()
      .set(UpsertLogSRV::from(self.clone()))
      .execute(conn)?;
    logs::table
      .find(self.id.clone())
      .get_result::<LogSRV>(conn)
      .map(|log| log.into())
  }
}

impl super::CltDbOperator for Log {
  fn create_clt(&self, conn: &mut diesel::SqliteConnection) -> Result<Self, diesel::result::Error> {
    use model::endpoint::logs::{
      LogCLT,
      NewLogCLT,
    };
    use schema::endpoint::logs;
    match diesel::insert_into(logs::table)
      .values(NewLogCLT::from(self.clone()))
      .get_result::<LogCLT>(conn)
    {
      Ok(log) => Ok(log.into()),
      Err(e) => Err(e),
    }
  }
  fn update_clt(&self, conn: &mut diesel::SqliteConnection) -> Result<Self, diesel::result::Error> {
    use model::endpoint::logs::{
      LogCLT,
      UpdateLogCLT,
    };
    use schema::endpoint::logs;
    match diesel::update(logs::table.find(self.id.clone()))
      .set(UpdateLogCLT::from(self.clone()))
      .get_result::<LogCLT>(conn)
    {
      Ok(log) => Ok(log.into()),
      Err(e) => Err(e),
    }
  }
  fn delete_clt(&self, conn: &mut diesel::SqliteConnection) -> Result<(), diesel::result::Error> {
    use schema::endpoint::logs;
    match diesel::delete(logs::table.find(self.id.clone())).execute(conn) {
      Ok(_) => Ok(()),
      Err(e) => Err(e),
    }
  }
  fn read_clt(&self, conn: &mut diesel::SqliteConnection) -> Result<Self, diesel::result::Error> {
    use model::endpoint::logs::LogCLT;
    use schema::endpoint::logs;
    match logs::table.find(self.id.clone()).get_result::<LogCLT>(conn) {
      Ok(log) => Ok(log.into()),
      Err(e) => Err(e),
    }
  }
  fn upsert_clt(&self, conn: &mut diesel::SqliteConnection) -> Result<Self, diesel::result::Error> {
    use model::endpoint::logs::{
      LogCLT,
      NewLogCLT,
      UpsertLogCLT,
    };
    use schema::endpoint::logs;
    diesel::insert_into(logs::table)
      .values(NewLogCLT::from(self.clone()))
      .on_conflict(logs::id)
      .do_update()
      .set(UpsertLogCLT::from(self.clone()))
      .execute(conn)?;
    logs::table
      .find(self.id.clone())
      .get_result::<LogCLT>(conn)
      .map(|log| log.into())
  }
}
