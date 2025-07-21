use serde::{Deserialize, Serialize};

/// Identifying data for an endpoint
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Endpoint {
  /// Endpoint id as provided by the server upon first connection
  pub id: Option<String>,
  /// Endpoint name which is the hostname of the machine
  pub name: String,
  /// Endpoint machineid, which is a hardware hash that should never change
  pub machineid: String,
}

impl Endpoint {
  pub fn merge(&mut self, other: Endpoint) -> Endpoint {
    assert!(self.name == other.name, "Endpoint merge failed: Name mismatch");
    assert!(self.machineid == other.machineid, "Endpoint merge failed: Machineid mismatch");
    Endpoint {
      id: other.id,
      name: self.name.clone(),
      machineid: self.machineid.clone(),
    }
  }
}
