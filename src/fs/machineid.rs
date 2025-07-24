use std::io::Write;

use machineid_rs::{Encryption, HWIDComponent, IdBuilder};
use tracing::info;

pub fn get_machineid() -> anyhow::Result<String> {
  match get_machineid_from_file() {
    Ok(Some(id)) => {
      info!("Using existing machineid {} from file", &id);
      Ok(id)
    }
    Ok(None) => {
      let id = generate_machineid();
      info!("Generated new machineid {} and saving to file", &id);
      match save_machineid(id.clone()) {
        Ok(_) => Ok(id),
        Err(e) => Err(e),
      }
    }
    Err(e) => Err(e.into()),
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
    Ok(id) => id,
    Err(e) => panic!("Failed to build machine ID: {e}"),
  }
}

fn get_machineid_from_file() -> anyhow::Result<Option<String>, std::io::Error> {
  let cdir = super::getcdir();
  tracing::info!("Reading ID from: {}id", &cdir);
  match std::fs::read_to_string(cdir + "machineid") {
    Ok(s) => {
      tracing::info!("Read machineid: {}", &s);
      Ok(Some(s))
    }
    Err(e) => {
      if e.kind() == std::io::ErrorKind::NotFound {
        tracing::error!("id file not found");
        return Ok(None);
      }
      tracing::error!("could not get machineid: {}", e);
      Err(e)
    }
  }
}

fn save_machineid(id: String) -> anyhow::Result<()> {
  let cdir = super::getcdir();
  let flnm = cdir.clone() + "machineid";
  match std::fs::exists(flnm.clone()) {
    Ok(true) => {
      tracing::info!("Updating existing machineid file");
      if let Err(e) = std::fs::remove_file(flnm.clone()) {
        anyhow::bail!("could not remove machineid file: {}", e);
      };
      match std::fs::write(flnm.clone(), id.clone()) {
        Err(e) => {
          anyhow::bail!("could not write machineid file: {}", e);
        }
        _ => Ok(()),
      }
    }
    Ok(false) => {
      tracing::info!("Creating new machineid file");
      if std::fs::exists(cdir.clone()).unwrap() {
        let mut fle = match std::fs::File::create(flnm.clone()) {
          Err(e) => {
            anyhow::bail!("Could not create machineid file: {}", e);
          }
          Ok(f) => f,
        };
        match fle.write(id.clone().as_bytes()) {
          Err(e) => {
            anyhow::bail!("could not write machineid file: {}", e);
          }
          _ => Ok(()),
        }
      } else {
        if let Err(e) = std::fs::create_dir_all(cdir.clone()) {
          anyhow::bail!("could not create machineid dir: {}", e);
        }
        let mut fle = match std::fs::File::create(flnm.clone()) {
          Err(e) => {
            anyhow::bail!("Could not create machineid file: {}", e);
          }
          Ok(f) => f,
        };
        match fle.write(id.clone().as_bytes()) {
          Err(e) => {
            anyhow::bail!("could not write machineid file: {}", e);
          }
          _ => Ok(()),
        }
      }
    }
    Err(e) => {
      Err(anyhow::anyhow!("{}", e))
    }
  }
}
