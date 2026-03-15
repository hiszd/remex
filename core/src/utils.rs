use rand::{
  distr::Alphanumeric,
  RngExt,
};

pub mod fs;

/// Generate a secret as 32 or 64 characters long
pub fn generate_secret(long: bool) -> String {
  let len = if long { 64 } else { 32 };

  rand::rng()
    .sample_iter(&Alphanumeric)
    .take(len)
    .map(char::from)
    .collect()
}
