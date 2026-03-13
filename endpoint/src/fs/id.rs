use std::{
  env,
  io::Write,
};

pub fn save_id(id: String) -> anyhow::Result<()> {
  let usr = env::var("USER").expect("No $USER env var found");
  let cdir = "/home/".to_owned() + &usr + "/.config/remex/";
  let flnm = cdir.clone() + "id";
  match std::fs::exists(flnm.clone()) {
    Ok(true) => {
      if let Err(e) = std::fs::remove_file(flnm.clone()) {
        anyhow::bail!("could not remove id file: {}", e);
      }
      if let Err(e) = std::fs::write(flnm.clone(), id.clone()) {
        anyhow::bail!("could not write id file: {}", e);
      }
      Ok(())
    }
    Ok(false) => {
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
        if let Err(e) = std::fs::create_dir_all(cdir.clone()) {
          anyhow::bail!("could not create id dir: {}", e);
        }
        let mut fle = match std::fs::File::create(flnm.clone()) {
          Err(e) => {
            anyhow::bail!("Could not create id file: {}", e);
          }
          Ok(f) => f,
        };
        if let Err(e) = fle.write(id.clone().as_bytes()) {
          anyhow::bail!("could not write id file: {}", e);
        }
        Ok(())
      }
    }
    Err(e) => Err(anyhow::anyhow!("{}", e)),
  }
}

pub fn get_id() -> anyhow::Result<Option<String>, std::io::Error> {
  let usr = env::var("USER").expect("No $USER env var found");
  let cdir = "/home/".to_owned() + &usr + "/.config/remex/";
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
  let usr = env::var("USER").expect("No $USER env var found");
  let cdir = "/home/".to_owned() + &usr + "/.config/remex/";
  tracing::info!("Reading ID from: {}id", &cdir);
  match std::fs::remove_file(cdir + "id") {
    Ok(_) => {
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
