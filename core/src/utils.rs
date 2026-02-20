use rand::Rng;

pub mod fs;

pub fn generate_secret(long: bool) -> String {
  let secret: String = rand::rng()
    .sample_iter(&rand::distr::Alphanumeric)
    .take(if long { 64 } else { 32 })
    .map(char::from)
    .collect();
  secret
}
