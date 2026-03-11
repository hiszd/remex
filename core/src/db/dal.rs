pub mod clients;
pub mod executions;
pub mod groups;
pub mod jobs;
pub mod logs;

pub trait SrvDbOperator
where
  Self: Sized,
{
  fn create_srv(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error>;
  fn update_srv(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error>;
  fn delete_srv(&self, conn: &mut diesel::PgConnection) -> Result<(), diesel::result::Error>;
  fn read_srv(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error>;
}

pub trait CltDbOperator
where
  Self: Sized,
{
  fn create_clt(&self, conn: &mut diesel::SqliteConnection) -> Result<Self, diesel::result::Error>;
  fn update_clt(&self, conn: &mut diesel::SqliteConnection) -> Result<Self, diesel::result::Error>;
  fn delete_clt(&self, conn: &mut diesel::SqliteConnection) -> Result<(), diesel::result::Error>;
  fn read_clt(&self, conn: &mut diesel::SqliteConnection) -> Result<Self, diesel::result::Error>;
  fn upsert_clt(&self, conn: &mut diesel::SqliteConnection) -> Result<Self, diesel::result::Error>;
}
