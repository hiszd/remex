use jsonwebtoken::{
  decode,
  encode,
  Algorithm,
  DecodingKey,
  EncodingKey,
  Header,
  Validation,
};
use serde::{
  Deserialize,
  Serialize,
};

const JWT_SECRET: &[u8] = b"remex_jwt_secret_key_change_in_production";
const JWT_ALGORITHM: Algorithm = Algorithm::HS256;
const JWT_EXPIRY_SECONDS: i64 = 7 * 24 * 60 * 60;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EndpointClaims {
  pub sub: String,
  pub client_id: String,
  pub client_name: String,
  pub hardware_hash: String,
  pub scope: String,
  pub exp: i64,
  pub iat: i64,
}

impl EndpointClaims {
  pub fn new(client_id: String, client_name: String, hardware_hash: String) -> Self {
    let now = chrono::Utc::now().timestamp();
    Self {
      sub: client_id.clone(),
      client_id,
      client_name,
      hardware_hash,
      scope: "endpoint".to_string(),
      exp: now + JWT_EXPIRY_SECONDS,
      iat: now,
    }
  }
}

pub fn generate_token(claims: &EndpointClaims) -> Result<String, jsonwebtoken::errors::Error> {
  encode(&Header::new(JWT_ALGORITHM), claims, &EncodingKey::from_secret(JWT_SECRET))
}

pub fn validate_token(token: &str) -> Result<EndpointClaims, jsonwebtoken::errors::Error> {
  let validation = Validation::new(JWT_ALGORITHM);
  let token_data =
    decode::<EndpointClaims>(token, &DecodingKey::from_secret(JWT_SECRET), &validation)?;
  Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_generate_and_validate_token() {
    let claims = EndpointClaims::new(
      "test-client-id".to_string(),
      "test-client".to_string(),
      "hw-hash-123".to_string(),
    );

    let token = generate_token(&claims).expect("Failed to generate token");
    let validated = validate_token(&token).expect("Failed to validate token");

    assert_eq!(validated.client_id, claims.client_id);
    assert_eq!(validated.client_name, claims.client_name);
    assert_eq!(validated.hardware_hash, claims.hardware_hash);
  }
}
