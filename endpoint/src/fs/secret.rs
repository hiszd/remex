use std::env;
use std::io::Write;

pub fn save_secret(secret: String) -> anyhow::Result<()> {
  let usr = env::var("USER").expect("No $USER env var found");
  let cdir = "/home/".to_owned() + &usr + "/.config/remex/";
  let flnm = cdir.clone() + "secret";
  match std::fs::exists(flnm.clone()) {
    Ok(true) => {
      if let Err(e) = std::fs::remove_file(flnm.clone()) {
        anyhow::bail!("could not remove secret file: {}", e);
      }
      if let Err(e) = std::fs::write(flnm.clone(), secret.clone()) {
        anyhow::bail!("could not write secret file: {}", e);
      }
      Ok(())
    }
    Ok(false) => {
      if std::fs::exists(cdir.clone()).unwrap() {
        let mut fle = match std::fs::File::create(flnm.clone()) {
          Err(e) => {
            anyhow::bail!("Could not create secret file: {}", e);
          }
          Ok(f) => f,
        };
        match fle.write(secret.clone().as_bytes()) {
          Err(e) => {
            anyhow::bail!("could not write secret file: {}", e);
          }
          _ => Ok(()),
        }
      } else {
        if let Err(e) = std::fs::create_dir_all(cdir.clone()) {
          anyhow::bail!("could not create secret dir: {}", e);
        }
        let mut fle = match std::fs::File::create(flnm.clone()) {
          Err(e) => {
            anyhow::bail!("Could not create secret file: {}", e);
          }
          Ok(f) => f,
        };
        if let Err(e) = fle.write(secret.clone().as_bytes()) {
          anyhow::bail!("could not write secret file: {}", e);
        }
        Ok(())
      }
    }
    Err(e) => Err(anyhow::anyhow!("{}", e)),
  }
}

pub fn get_secret() -> anyhow::Result<Option<String>, std::io::Error> {
  let usr = env::var("USER").expect("No $USER env var found");
  let cdir = "/home/".to_owned() + &usr + "/.config/remex/";
  tracing::info!("Reading secret from: {}secret", &cdir);
  match std::fs::read_to_string(cdir + "secret") {
    Ok(s) => {
      tracing::info!("Read secret: {}", &s);
      Ok(Some(s))
    }
    Err(e) => {
      if e.kind() == std::io::ErrorKind::NotFound {
        tracing::error!("secret file not found");
        return Ok(None);
      }
      tracing::error!("could not get secret: {}", e);
      Err(e)
    }
  }
}

pub fn remove_secret() -> anyhow::Result<(), std::io::Error> {
  let usr = env::var("USER").expect("No $USER env var found");
  let cdir = "/home/".to_owned() + &usr + "/.config/remex/";
  tracing::info!("Reading secret from: {}secret", &cdir);
  match std::fs::remove_file(cdir + "secret") {
    Ok(_) => {
      tracing::info!("removed secret file");
      Ok(())
    }
    Err(e) => {
      if e.kind() == std::io::ErrorKind::NotFound {
        tracing::error!("secret file not found");
        return Ok(());
      }
      tracing::error!("could not get secret: {}", e);
      Err(e)
    }
  }
}
