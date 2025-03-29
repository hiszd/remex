use std::collections::HashMap;

use anyhow::anyhow;

pub struct SessionMap {
  pub sessions: HashMap<u64, actix::Addr<crate::session::RemexSession>>,
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
    id: u64,
    addr: actix::Addr<crate::session::RemexSession>,
  ) -> anyhow::Result<()> {
    let _ = self.sessions.insert(id, addr);
    Ok(())
  }

  pub fn remove(&mut self, id: u64) -> Option<actix::Addr<crate::session::RemexSession>> {
    self.sessions.remove(&id)
  }

  pub fn exists(&self, id: u64) -> bool { self.sessions.contains_key(&id) }

  pub fn change_id(&mut self, old_id: u64, new_id: u64) -> anyhow::Result<()> {
    if !self.exists(old_id) {
      return Err(anyhow!("Session does not exist"));
    }
    if self.exists(new_id) {
      return Err(anyhow!("Duplicate session id, cannot assign"));
    }
    let old = match self.sessions.remove(&old_id) {
      None => return Err(anyhow!("Session does not exist")),
      Some(s) => s,
    };
    match self.sessions.insert(new_id, old) {
      Some(_) => Ok(()),
      None => Ok(()),
    }
  }

  pub fn get_addr(&self, id: u64) -> anyhow::Result<actix::Addr<crate::session::RemexSession>> {
    match self.sessions.get(&id) {
      Some(s) => Ok(s.clone()),
      None => Err(anyhow::anyhow!("Could not find addr for id {}", &id)),
    }
  }
}
