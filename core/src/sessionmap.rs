use std::collections::HashMap;

use anyhow::anyhow;

pub struct SessionMap<T> {
  pub sessions: HashMap<T, actix::Addr<crate::actors::session::RemexSession>>,
}

impl<T> Default for SessionMap<T> {
  fn default() -> Self {
    SessionMap {
      sessions: HashMap::new(),
    }
  }
}

impl<T> SessionMap<T>
where
  T: std::cmp::Eq + std::hash::Hash + Clone + std::fmt::Display,
{
  pub fn insert(
    &mut self,
    id: T,
    addr: actix::Addr<crate::actors::session::RemexSession>,
  ) -> anyhow::Result<()> {
    if self.sessions.contains_key(&id) {
      return Err(anyhow!("Client with id {} is already connected", &id));
    }
    self.sessions.insert(id, addr);
    Ok(())
  }

  pub fn remove(&mut self, id: &T) -> Option<actix::Addr<crate::actors::session::RemexSession>> {
    self.sessions.remove(id)
  }

  pub fn exists(&self, id: &T) -> bool { self.sessions.contains_key(id) }

  pub fn change_id(&mut self, old_id: &T, new_id: &T) -> anyhow::Result<()> {
    if !self.exists(old_id) {
      return Err(anyhow!("Session does not exist"));
    }
    if self.exists(new_id) {
      return Err(anyhow!("Duplicate session id, cannot assign"));
    }
    let old = match self.sessions.remove(old_id) {
      None => return Err(anyhow!("Session does not exist")),
      Some(s) => s,
    };
    match self.sessions.insert(new_id.to_owned(), old) {
      Some(_) => Ok(()),
      None => Ok(()),
    }
  }

  pub fn get_addr(
    &self,
    id: T,
  ) -> anyhow::Result<actix::Addr<crate::actors::session::RemexSession>> {
    match self.sessions.get(&id) {
      Some(s) => Ok(s.clone()),
      None => Err(anyhow::anyhow!("Could not find addr for id {}", &id)),
    }
  }
}
