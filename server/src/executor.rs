use remex_core::db::{
  endpoint::model::executor::ExecutorModel as EndpntExecutorModel,
  server::model::executor::ExecutorModel,
};
use serde::{
  Deserialize,
  Serialize,
};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum ExecutorStatus {
  Created,
  Assigned,
  Running,
  Success,
  Failed,
}

impl Into<String> for ExecutorStatus {
  fn into(self) -> String {
    match self {
      ExecutorStatus::Created => String::from("created"),
      ExecutorStatus::Assigned => String::from("assigned"),
      ExecutorStatus::Running => String::from("running"),
      ExecutorStatus::Success => String::from("success"),
      ExecutorStatus::Failed => String::from("failed"),
    }
  }
}

impl From<String> for ExecutorStatus {
  fn from(s: String) -> Self {
    match s.as_str() {
      "created" => ExecutorStatus::Created,
      "assigned" => ExecutorStatus::Assigned,
      "running" => ExecutorStatus::Running,
      "success" => ExecutorStatus::Success,
      "failed" => ExecutorStatus::Failed,
      _ => panic!("Unknown executor status: {}", s),
    }
  }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[allow(non_snake_case)]
pub struct Executor {
  pub id: String,
  pub name: String,
  pub command: String,
  pub status: ExecutorStatus,
  pub active: bool,
  #[serde(rename = "createdAt")]
  pub created_at: chrono::NaiveDateTime,
  #[serde(rename = "updatedAt")]
  pub updated_at: chrono::NaiveDateTime,
}

impl From<ExecutorModel> for Executor {
  fn from(e: ExecutorModel) -> Self {
    Executor {
      id: e.id,
      name: e.name,
      command: e.command,
      status: e.status.into(),
      active: e.active,
      created_at: e.created_at,
      updated_at: e.updated_at,
    }
  }
}
impl From<&ExecutorModel> for Executor {
  fn from(e: &ExecutorModel) -> Self {
    Executor {
      id: e.id.clone(),
      name: e.name.clone(),
      command: e.command.clone(),
      status: e.status.clone().into(),
      active: e.active,
      created_at: e.created_at,
      updated_at: e.updated_at,
    }
  }
}

impl Into<ExecutorModel> for Executor {
  fn into(self) -> ExecutorModel {
    ExecutorModel {
      id: self.id,
      name: self.name,
      command: self.command,
      status: self.status.into(),
      active: self.active,
      created_at: self.created_at,
      updated_at: self.updated_at,
    }
  }
}

impl From<EndpntExecutorModel> for Executor {
  fn from(e: EndpntExecutorModel) -> Self {
    Executor {
      id: e.id,
      name: e.name,
      command: e.command,
      status: e.status.into(),
      active: e.active,
      created_at: e.created_at,
      updated_at: e.updated_at,
    }
  }
}
impl From<&EndpntExecutorModel> for Executor {
  fn from(e: &EndpntExecutorModel) -> Self {
    Executor {
      id: e.id.clone(),
      name: e.name.clone(),
      command: e.command.clone(),
      status: e.status.clone().into(),
      active: e.active,
      created_at: e.created_at,
      updated_at: e.updated_at,
    }
  }
}

impl Into<EndpntExecutorModel> for Executor {
  fn into(self) -> EndpntExecutorModel {
    EndpntExecutorModel {
      id: self.id,
      name: self.name,
      command: self.command,
      status: self.status.into(),
      active: self.active,
      created_at: self.created_at,
      updated_at: self.updated_at,
    }
  }
}

// this is a collection of executors, but this also needs to have the function of updating the
// database when a change is made.
// the lifecycle should look like this:
// [X] 1. create this struct, and query the database for the existing executors
// [X] 2. whenever the endpoint gets pushed a new list of executors, compare to the existing and update
//    the database if necessary
// [ ] 3. the endpoint will want to get the executors that have not yet been executed
// [ ] 4. the endpoint will also want to update the executors as they are executed, and include log
//    messages from the execution.
pub struct Vexecutors {
  executors: Vec<Executor>,
  pool: sqlx::SqlitePool,
}

impl Vexecutors {
  /// Creates a new Vexecutors struct and queries the database for the existing executors
  pub async fn new(pool: sqlx::SqlitePool) -> Vexecutors {
    let executors: Vec<ExecutorModel> = sqlx::query_as("SELECT * FROM executors")
      .fetch_all(&pool)
      .await
      .unwrap();
    Vexecutors {
      executors: executors.iter().map(|e| e.clone().into()).collect(),
      pool,
    }
  }

  /// Updates the executors in the database by adding new executors and updating existing ones if
  /// they updated_at value is newer than the one in the database
  pub async fn update(&mut self, executors: Vec<Executor>) -> Vec<Executor> {
    if self.executors != executors {
      let mut execs: Vec<Executor> = executors.clone();
      let temp_dbexecs: Vec<ExecutorModel> = execs.iter().map(|e| e.clone().into()).collect();
      let old_dbexecs: Vec<ExecutorModel> = sqlx::query_as("SELECT * FROM executors")
        .fetch_all(&self.pool)
        .await
        .unwrap();
      for t in temp_dbexecs {
        let o = match old_dbexecs.iter().find(|e| e.id == t.id) {
          Some(e) => Some(e.clone()),
          None => None,
        };
        if !o.is_none() {
          sqlx::query(format!("INSERT INTO executors (id, name, command, status, active, created_at, updated_at) VALUES {}", t.dbvalues()).as_str())
            .execute(&self.pool)
            .await
            .unwrap();
        } else {
          let o = o.unwrap();
          if t.updated_at > o.updated_at {
            sqlx::query(format!("UPDATE executors SET name = {}, command = {}, active = {}, created_at = {}, updated_at = {} WHERE id = {}", &t.name, &t.command, &t.active, &t.created_at, &t.updated_at, &t.id).as_str())
                .execute(&self.pool)
                .await
                .unwrap();
            match execs.iter_mut().find(|e| e.id == t.id) {
              Some(e) => e.updated_at = t.updated_at,
              None => (),
            }
          }
        }
      }
      self.executors = execs.clone();
      return execs
        .iter()
        .filter(|e| e.status != ExecutorStatus::Success)
        .cloned()
        .collect();
    } else {
      return executors
        .iter()
        .filter(|e| e.status != ExecutorStatus::Success)
        .cloned()
        .collect();
    }
  }

  /// Returns a vector of executors that have not yet been executed
  pub fn get_relevant_active(&self) -> Vec<Executor> {
    self
      .executors
      .iter()
      .filter(|e| e.status != ExecutorStatus::Success || e.active == true)
      .cloned()
      .collect()
  }

  pub fn len(&self) -> usize { self.executors.len() }
  pub fn is_empty(&self) -> bool { self.executors.is_empty() }
  fn clear(&mut self) { self.executors.clear(); }
  pub fn get(&self, executor_id: &str) -> Option<Executor> {
    self.executors.iter().find(|e| e.id == executor_id).cloned()
  }
}
