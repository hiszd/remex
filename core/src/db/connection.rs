use surrealdb::{
  engine::{
    local::Db,
    remote::ws::{
      Client,
      Ws,
    },
  },
  opt::auth::Root,
  Surreal,
};

pub const CORE_DB_URL: &str = "192.168.10.87:8090";

pub async fn is_connected(db: &Surreal<Client>) -> bool {
  match db.health().await {
    Ok(_) => true,
    Err(e) => {
      tracing::warn!("Core DB health check failed: {}", e);
      false
    }
  }
}

pub struct DbClients {
  pub local: Surreal<Db>,
  pub remote: Option<Surreal<Client>>,
}

impl DbClients {
  pub fn new(local: Surreal<Db>) -> Self {
    DbClients {
      local,
      remote: None,
    }
  }

  pub fn with_remote(mut self, remote: Surreal<Client>) -> Self {
    self.remote = Some(remote);
    self
  }

  pub fn is_core_connected(&self) -> bool { self.remote.is_some() }
}
