use std::io::Write;

use gethostname::gethostname;
use serde::{Deserialize, Serialize};

use super::machineid::get_machineid;
use crate::endpoint::Endpoint;

/// Identifying data for an endpoint
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StoredEndpoint {
  /// Endpoint id as provided by the server upon first connection
  pub id: Option<String>,
  /// Endpoint name which is the hostname of the machine
  pub name: String,
  /// Endpoint machineid, which is a hardware hash that should never change
  pub machineid: String,
  pub secret: Option<String>,
}

impl From<StoredEndpoint> for Endpoint {
  fn from(value: StoredEndpoint) -> Self {
    Endpoint {
      id: value.id,
      name: value.name,
      machineid: value.machineid,
    }
  }
}

impl From<Endpoint> for StoredEndpoint {
  fn from(value: Endpoint) -> Self {
    StoredEndpoint {
      id: value.id,
      name: value.name,
      machineid: value.machineid,
      secret: None,
    }
  }
}

pub fn save_identity(id: StoredEndpoint) -> Result<(), std::io::Error> {
  let cdir = super::getcdir();
  let flnm = cdir.clone() + "identity.json";
  let mut f = std::fs::File::create(flnm).unwrap();
  f.write_all(serde_json::to_string(&id).unwrap().as_bytes())
    .unwrap();
  Ok(())
}

pub fn get_identity() -> StoredEndpoint {
  if !exists_identity() {
    return StoredEndpoint {
      id: None,
      name: gethostname().into_string().unwrap(),
      machineid: get_machineid().unwrap(),
      secret: None,
    };
  }
  let cdir = super::getcdir();
  let flnm = cdir.clone() + "identity.json";
  let f = std::fs::File::open(flnm).unwrap();
  serde_json::from_reader(f).unwrap()
}

pub fn exists_identity() -> bool {
  let cdir = super::getcdir();
  let flnm = cdir.clone() + "identity.json";
  std::fs::exists(flnm).unwrap()
}

pub fn reset_identity() -> Result<(), std::io::Error> {
  let cdir = super::getcdir();
  let flnm = cdir.clone() + "identity.json";
  let f = std::fs::File::open(flnm)?;
  let mut id: StoredEndpoint = serde_json::from_reader(f)?;
  id.secret = None;
  save_identity(id.clone())?;
  Ok(())
}
