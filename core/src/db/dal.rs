pub mod clients;
pub mod executions;
pub mod groups;
pub mod jobs;
pub mod logs;

pub trait SrvDbOperator
where
  Self: Sized,
{
  fn create(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error>;
  fn update(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error>;
  fn delete(&self, conn: &mut diesel::PgConnection) -> Result<(), diesel::result::Error>;
  fn read(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error>;
  fn upsert(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error>;
}
