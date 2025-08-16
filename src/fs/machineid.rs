use std::io::Write;

use machineid_rs::{Encryption, HWIDComponent, IdBuilder};
use thiserror::Error;
use tracing::{info, warn};

// FIXME: use polyerror
#[derive(Error, Debug)]
pub enum Error {
  #[error("IO error: {0}")]
  Io(std::io::Error),
  #[error("MachineId Error: {0}")]
  Other(String),
}

pub fn get_machineid() -> Result<String, Error> {
  match get_machineid_from_file() {
    Ok(Some(id)) => {
      info!("Using existing machineid {} from file", &id);
      Ok(id)
    }
    Ok(None) => {
      let id = generate_machineid();
      warn!("Generated machineid {}", &id);
      match save_machineid(id.clone()) {
        Ok(_) => Ok(id),
        Err(e) => Err(e),
      }
    }
    Err(e) => Err(e),
  }
}

fn generate_machineid() -> String {
  let mut builder = IdBuilder::new(Encryption::SHA256);

  // Add hardware components to the identifier
  builder
    .add_component(HWIDComponent::SystemID)
    .add_component(HWIDComponent::CPUCores)
    .add_component(HWIDComponent::MacAddress)
    .add_component(HWIDComponent::CPUID);

  // Build the unique ID
  match builder.build("") {
    Ok(id) => id.split_at(60).0.to_string(),
    Err(e) => panic!("Failed to build machine ID: {e}"),
  }
}

fn get_machineid_from_file() -> Result<Option<String>, Error> {
  let cdir = super::getcdir();
  tracing::info!("Reading ID from: {}id", &cdir);
  match std::fs::read_to_string(cdir + "machineid") {
    Ok(s) => {
      tracing::info!("Read machineid: {}", &s);
      Ok(Some(s))
    }
    Err(e) => {
      if e.kind() == std::io::ErrorKind::NotFound {
        tracing::warn!("machineid file not found");
        return Ok(None);
      }
      tracing::error!("could not get machineid: {}", e);
      Err(Error::Io(e))
    }
  }
}

// TODO: Use polyerror and thiserror
fn save_machineid(id: String) -> Result<(), Error> {
  let cdir = super::getcdir();
  let flnm = cdir.clone() + "machineid";
  match std::fs::exists(flnm.clone()) {
    Ok(true) => {
      tracing::info!("Updating existing machineid file");
      if let Err(e) = std::fs::remove_file(flnm.clone()) {
        return Err(Error::Other(format!("could not remove machineid file: {}", e)));
      };
      match std::fs::write(flnm.clone(), id.clone()) {
        Err(e) => {
          return Err(Error::Other(format!("could not write machineid file: {}", e)));
        }
        _ => Ok(()),
      }
    }
    Ok(false) => {
      tracing::warn!("Creating new machineid file");
      if std::fs::exists(cdir.clone()).unwrap() {
        let mut fle = match std::fs::File::create(flnm.clone()) {
          Err(e) => {
            return Err(Error::Other(format!("Could not create machineid file: {}", e)));
          }
          Ok(f) => f,
        };
        match fle.write(id.clone().as_bytes()) {
          Err(e) => {
            return Err(Error::Other(format!("could not write machineid file: {}", e)));
          }
          _ => Ok(()),
        }
      } else {
        if let Err(e) = std::fs::create_dir_all(cdir.clone()) {
          return Err(Error::Other(format!("could not create machineid dir: {}", e)));
        }
        let mut fle = match std::fs::File::create(flnm.clone()) {
          Err(e) => {
            return Err(Error::Other(format!("Could not create machineid file: {}", e)));
          }
          Ok(f) => f,
        };
        match fle.write(id.clone().as_bytes()) {
          Err(e) => {
            return Err(Error::Other(format!("could not write machineid file: {}", e)));
          }
          _ => Ok(()),
        }
      }
    }
    Err(e) => Err(Error::Io(e)),
  }
}
