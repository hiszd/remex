use std::env;

pub fn save_secret(name: &str, secret: String) -> anyhow::Result<()> {
  let usr = env::var("USER").expect("No $USER env var found");
  let cdir = "/home/".to_owned() + &usr + "/.config/remex/";
  let flnm = cdir.clone() + format!("{}secret", name).as_str();
  match std::fs::write(flnm.clone(), secret.clone()) {
    Ok(_) => Ok(()),
    Err(e) => match e.kind() {
      std::io::ErrorKind::NotFound => {
        std::fs::create_dir_all(cdir.clone()).unwrap();
        std::fs::write(flnm.clone(), secret.clone()).unwrap();
        Err(e.into())
      }
      _ => {
        tracing::error!("could not write secret file: {}", e);
        Err(e.into())
      }
    },
  }
}

pub fn get_secret(name: &str) -> anyhow::Result<Option<String>, std::io::Error> {
  let usr = env::var("USER").expect("No $USER env var found");
  let cdir = "/home/".to_owned() + &usr + "/.config/remex/";
  tracing::debug!("Reading secret from: {}secret", &cdir);
  match std::fs::read_to_string(cdir + format!("{}secret", name).as_str()) {
    Ok(s) => {
      tracing::debug!("Read secret: {}", &s);
      Ok(Some(s))
    }
    Err(e) => {
      if e.kind() == std::io::ErrorKind::NotFound {
        tracing::debug!("secret file not found");
        return Ok(None);
      }
      tracing::error!("could not get secret: {}", e);
      Err(e)
    }
  }
}
