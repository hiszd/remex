use std::env;
use std::io::Write;

pub fn save_id(id: u64) -> anyhow::Result<()> {
  let usr = env::var("USER").expect("No $USER env var found");
  let cdir = "/home/".to_owned() + &usr + "/.config/remex/";
  let flnm = cdir.clone() + "id";
  tracing::info!("dir: {}, flnm: {}", &cdir, &flnm);
  match std::fs::exists(flnm.clone()) {
    Ok(true) => {
      match std::fs::remove_file(flnm.clone()) {
        Err(e) => {
          anyhow::bail!("could not remove id file: {}", e);
        }
        _ => {}
      };
      match std::fs::write(flnm.clone(), id.to_string()) {
        Err(e) => {
          anyhow::bail!("could not write id file: {}", e);
        }
        _ => Ok(()),
      }
    }
    Ok(false) => {
      if std::fs::exists(cdir.clone()).unwrap() {
        let mut fle = match std::fs::File::create(flnm.clone()) {
          Err(e) => {
            anyhow::bail!("Could not create id file: {}", e);
          }
          Ok(f) => f,
        };
        match fle.write(id.to_string().as_bytes()) {
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
        match fle.write(id.to_string().as_bytes()) {
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

pub fn get_id() -> anyhow::Result<Option<String>, std::io::Error> {
  let usr = env::var("USER").expect("No $USER env var found");
  let cdir = "/home/".to_owned() + &usr + ".config/remex/";
  match std::fs::read_to_string(cdir + "id") {
    Ok(s) => Ok(Some(s)),
    Err(e) => {
      if e.kind() == std::io::ErrorKind::NotFound {
        return Ok(None);
      }
      tracing::error!("could not get id: {}", e);
      Err(e)
    }
  }
}
