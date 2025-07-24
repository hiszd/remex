use std::collections::HashMap;

use anyhow::anyhow;
use tracing::{error, info};

use crate::endpoint::Endpoint;
pub mod maxarray;

pub struct SessionItem {
  pub identity: Endpoint,
  pub addr: actix::Addr<crate::session::RemexSession>,
  pub previous_ids: maxarray::MaxArray<String, 10>,
}

pub struct SessionMap {
  pub sessions: HashMap<String, SessionItem>,
}

impl Default for SessionMap {
  fn default() -> Self {
    SessionMap {
      sessions: HashMap::new(),
    }
  }
}

impl SessionMap {
  pub fn insert(
    &mut self,
    identity: Endpoint,
    addr: actix::Addr<crate::session::RemexSession>,
  ) -> anyhow::Result<()> {
    let _ = self.sessions.insert(identity.machineid.clone(), SessionItem {
      identity,
      addr,
      previous_ids: maxarray::MaxArray::new(),
    });
    Ok(())
  }

  pub fn remove(&mut self, machineid: String) -> Option<SessionItem> {
    self.sessions.remove(&machineid)
  }

  pub fn exists(&self, machineid: String) -> bool { self.sessions.contains_key(&machineid) }

  pub fn update_identity(
    &mut self,
    machineid: String,
    identity: Endpoint,
  ) -> Result<(), anyhow::Error> {
    match self.sessions.get_mut(&machineid) {
      Some(s) => {
        s.identity = identity;
        Ok(())
      }
      None => {
        error!("Could not find session for machineid {}", &machineid);
        Err(anyhow!("Could not find session for machineid {}", &machineid))
      }
    }
  }

  pub fn get_addr(&self, id: String) -> anyhow::Result<actix::Addr<crate::session::RemexSession>> {
    match self.sessions.get(&id) {
      Some(s) => Ok(s.addr.clone()),
      None => Err(anyhow::anyhow!("Could not find addr for id {}", &id)),
    }
  }
}
