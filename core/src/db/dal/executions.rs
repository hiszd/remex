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
pub struct Execution {
  pub id: String,
  pub job_id: Option<String>,
  pub client_id: String,
  pub executed_at: Option<chrono::NaiveDateTime>,
  pub execution_result: Option<String>,
  pub created_at: chrono::NaiveDateTime,
  pub updated_at: chrono::NaiveDateTime,
}

impl From<model::server::executions::ExecutionSRV> for Execution {
  fn from(execution: model::server::executions::ExecutionSRV) -> Self {
    Execution {
      id: execution.id,
      job_id: execution.job_id,
      client_id: execution.client_id,
      executed_at: execution.executed_at,
      execution_result: execution.execution_result,
      created_at: execution.created_at,
      updated_at: execution.updated_at,
    }
  }
}

impl From<model::endpoint::executions::ExecutionCLT> for Execution {
  fn from(execution: model::endpoint::executions::ExecutionCLT) -> Self {
    Execution {
      id: execution.id,
      job_id: execution.job_id,
      client_id: execution.client_id,
      executed_at: execution.executed_at,
      execution_result: execution.execution_result,
      created_at: execution.created_at,
      updated_at: execution.updated_at,
    }
  }
}

impl super::CltDbOperator for Execution {
  fn create_clt(&self, conn: &mut diesel::SqliteConnection) -> Result<Self, diesel::result::Error> {
    use model::endpoint::executions::{
      ExecutionCLT,
      NewExecutionCLT,
    };
    use schema::endpoint::executions;
    match diesel::insert_into(executions::table)
      .values(NewExecutionCLT::from(self.clone()))
      .get_result::<ExecutionCLT>(conn)
    {
      Ok(execution) => Ok(execution.into()),
      Err(e) => Err(e),
    }
  }
  fn update_clt(&self, conn: &mut diesel::SqliteConnection) -> Result<Self, diesel::result::Error> {
    use model::endpoint::executions::{
      ExecutionCLT,
      UpdateExecutionCLT,
    };
    use schema::endpoint::executions;
    match diesel::update(executions::table.find(self.id.clone()))
      .set(UpdateExecutionCLT::from(self.clone()))
      .get_result::<ExecutionCLT>(conn)
    {
      Ok(execution) => Ok(execution.into()),
      Err(e) => Err(e),
    }
  }
  fn delete_clt(&self, conn: &mut diesel::SqliteConnection) -> Result<(), diesel::result::Error> {
    use schema::endpoint::executions;
    match diesel::delete(executions::table.find(self.id.clone())).execute(conn) {
      Ok(_) => Ok(()),
      Err(e) => Err(e),
    }
  }
  fn read_clt(&self, conn: &mut diesel::SqliteConnection) -> Result<Self, diesel::result::Error> {
    use model::endpoint::executions::ExecutionCLT;
    use schema::endpoint::executions;
    match executions::table
      .find(self.id.clone())
      .get_result::<ExecutionCLT>(conn)
    {
      Ok(execution) => Ok(execution.into()),
      Err(e) => Err(e),
    }
  }
  fn upsert_clt(&self, conn: &mut diesel::SqliteConnection) -> Result<Self, diesel::result::Error> {
    use model::endpoint::executions::{
      ExecutionCLT,
      NewExecutionCLT,
      UpsertExecutionCLT,
    };
    use schema::endpoint::executions;
    diesel::insert_into(executions::table)
      .values(NewExecutionCLT::from(self.clone()))
      .on_conflict(executions::id)
      .do_update()
      .set(UpsertExecutionCLT::from(self.clone()))
      .execute(conn)?;
    executions::table
      .find(self.id.clone())
      .get_result::<ExecutionCLT>(conn)
      .map(|execution| execution.into())
  }
}

impl super::SrvDbOperator for Execution {
  fn create_srv(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error> {
    use model::server::executions::{
      ExecutionSRV,
      NewExecutionSRV,
    };
    use schema::server::executions;
    match diesel::insert_into(executions::table)
      .values(NewExecutionSRV::from(self.clone()))
      .get_result::<ExecutionSRV>(conn)
    {
      Ok(execution) => Ok(execution.into()),
      Err(e) => Err(e),
    }
  }
  fn update_srv(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error> {
    use model::server::executions::{
      ExecutionSRV,
      UpdateExecutionSRV,
    };
    use schema::server::executions;
    match diesel::update(executions::table.find(self.id.clone()))
      .set(UpdateExecutionSRV::from(self.clone()))
      .get_result::<ExecutionSRV>(conn)
    {
      Ok(execution) => Ok(execution.into()),
      Err(e) => Err(e),
    }
  }
  fn delete_srv(&self, conn: &mut diesel::PgConnection) -> Result<(), diesel::result::Error> {
    use schema::server::executions;
    match diesel::delete(executions::table.find(self.id.clone())).execute(conn) {
      Ok(_) => Ok(()),
      Err(e) => Err(e),
    }
  }
  fn read_srv(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error> {
    use model::server::executions::ExecutionSRV;
    use schema::server::executions;
    match executions::table
      .find(self.id.clone())
      .get_result::<ExecutionSRV>(conn)
    {
      Ok(execution) => Ok(execution.into()),
      Err(e) => Err(e),
    }
  }
  fn upsert_srv(&self, conn: &mut diesel::PgConnection) -> Result<Self, diesel::result::Error> {
    use model::server::executions::{
      ExecutionSRV,
      NewExecutionSRV,
      UpsertExecutionSRV,
    };
    use schema::server::executions;
    diesel::insert_into(executions::table)
      .values(NewExecutionSRV::from(self.clone()))
      .on_conflict(executions::id)
      .do_update()
      .set(UpsertExecutionSRV::from(self.clone()))
      .execute(conn)?;
    executions::table
      .find(self.id.clone())
      .get_result::<ExecutionSRV>(conn)
      .map(|execution| execution.into())
  }
}
