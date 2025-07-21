use std::env;
use std::io::Write;

pub fn save_id(id: String) -> anyhow::Result<()> {
  let cdir = super::getcdir();
  let flnm = cdir.clone() + "id";
  match std::fs::exists(flnm.clone()) {
    Ok(true) => {
      tracing::info!("Updating existing id file");
      match std::fs::remove_file(flnm.clone()) {
        Err(e) => {
          anyhow::bail!("could not remove id file: {}", e);
        }
        _ => {}
      };
      match std::fs::write(flnm.clone(), id.clone()) {
        Err(e) => {
          anyhow::bail!("could not write id file: {}", e);
        }
        _ => Ok(()),
      }
    }
    Ok(false) => {
      tracing::info!("Creating new id file");
      if std::fs::exists(cdir.clone()).unwrap() {
        let mut fle = match std::fs::File::create(flnm.clone()) {
          Err(e) => {
            anyhow::bail!("Could not create id file: {}", e);
          }
          Ok(f) => f,
        };
        match fle.write(id.clone().as_bytes()) {
          Err(e) => {
            anyhow::bail!("could not write id file: {}", e);
          }
          _ => Ok(()),
        }
      } else {
        match std::fs::create_dir_all(cdir.clone()) {
          Err(e) => {
            anyhow::bail!("could not create id dir: {}", e);
          }
          _ => {}
        }
        let mut fle = match std::fs::File::create(flnm.clone()) {
          Err(e) => {
            anyhow::bail!("Could not create id file: {}", e);
          }
          Ok(f) => f,
        };
        match fle.write(id.clone().as_bytes()) {
          Err(e) => {
            anyhow::bail!("could not write id file: {}", e);
          }
          _ => Ok(()),
        }
      }
    }
    Err(e) => {
      return Err(anyhow::anyhow!("{}", e));
    }
  }
}

pub fn get_id_from_file() -> anyhow::Result<Option<String>, std::io::Error> {
  let cdir = super::getcdir();
  tracing::info!("Reading ID from: {}id", &cdir);
  match std::fs::read_to_string(cdir + "id") {
    Ok(s) => {
      tracing::info!("Read id: {}", &s);
      Ok(Some(s))
    }
    Err(e) => {
      if e.kind() == std::io::ErrorKind::NotFound {
        tracing::error!("id file not found");
        return Ok(None);
      }
      tracing::error!("could not get id: {}", e);
      Err(e)
    }
  }
}

pub fn remove_id() -> anyhow::Result<(), std::io::Error> {
  let cdir = super::getcdir();
  match std::fs::remove_file(cdir + "id") {
    Ok(s) => {
      tracing::info!("removed id file");
      Ok(())
    }
    Err(e) => {
      if e.kind() == std::io::ErrorKind::NotFound {
        tracing::error!("id file not found");
        return Ok(());
      }
      tracing::error!("could not get id: {}", e);
      Err(e)
    }
  }
}
