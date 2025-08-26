use std::env;

pub mod id;
pub mod identity;
pub mod machineid;

pub fn getcdir() -> String {
  let usr = env::var("USER").expect("No $USER env var found");
  "/home/".to_owned() + &usr + "/.config/remex/"
}
