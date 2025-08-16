use std::collections::HashMap;

use thiserror::Error;
use tracing::error;

use crate::endpoint::Endpoint;
pub mod maxarray;

// FIXME: use polyerror
#[derive(Error, Debug)]
pub enum Error {
  #[error("IO error: {0}")]
  Io(std::io::Error),
  #[error("Session Error: {0}")]
  Other(String),
}

pub struct SessionItem {
  pub identity: Endpoint,
  pub addr: actix::Addr<crate::session::RemexSession>,
  pub previous_ids: maxarray::MaxArray<String, 10>,
}

#[derive(Default)]
pub struct SessionMap {
  pub sessions: HashMap<String, SessionItem>,
}

impl SessionMap {
  pub fn insert(&mut self, identity: Endpoint, addr: actix::Addr<crate::session::RemexSession>) {
    let _ = self
      .sessions
      .insert(identity.machineid.clone(), SessionItem {
        identity,
        addr,
        previous_ids: maxarray::MaxArray::new(),
      });
  }

  pub fn remove(&mut self, machineid: String) -> Option<SessionItem> {
    self.sessions.remove(&machineid)
  }

  pub fn exists(&self, machineid: String) -> bool { self.sessions.contains_key(&machineid) }

  pub fn update_identity(&mut self, machineid: String, identity: Endpoint) -> Result<(), Error> {
    match self.sessions.get_mut(&machineid) {
      Some(s) => {
        s.identity = identity;
        Ok(())
      }
      None => {
        error!("Could not find session for machineid {}", &machineid);
        Err(Error::Other(format!("Could not find session for machineid {}", &machineid)))
      }
    }
  }

  pub fn get_addr(&self, id: String) -> Result<actix::Addr<crate::session::RemexSession>, Error> {
    match self.sessions.get(&id) {
      Some(s) => Ok(s.addr.clone()),
      None => Err(Error::Other(format!("Could not find addr for id {}", &id))),
    }
  }
}
