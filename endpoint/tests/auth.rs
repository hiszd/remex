//! Integration tests for endpoint signup/signin against a real SurrealDB instance.
//!
//! These tests exercise the real SIGNUP/SIGNIN blocks defined in `core/src/db/model/clients.rs`
//! via the WS SQL endpoint, which is the only endpoint that supports `ACCESS ... SIGNUP/Signin`.
//!
//! Prerequisites:
//!   docker run -d --name remex-test-surrealdb -p 8000:8000 surrealdb/surrealdb:latest start --log trace --user root --pass root
//!
//! Run:
//!   cargo test -p remex-endpoint --test auth -- --test-threads=1

use serde_json::Value;

/// Clean up all test data
async fn cleanup(db: &surrealdb::Surreal<surrealdb::engine::any::Any>) {
  db.query("REMOVE NAMESPACE remex;")
    .await
    .unwrap()
    .check()
    .unwrap();
}

async fn db_signin() -> surrealdb::Surreal<surrealdb::engine::any::Any> {
  let db: surrealdb::Surreal<surrealdb::engine::any::Any> = surrealdb::Surreal::init();
  db.connect("ws://localhost:8000")
    .await
    .expect("failed to connect to SurrealDB");
  db.signin(surrealdb::opt::auth::Root {
    username: "root".into(),
    password: "root".into(),
  })
  .await
  .unwrap();
  db
}

/// Setup: run migrations via the SDK, seed a user + enrollment token via WS SQL
async fn setup_test_db(test_id: &str) -> String {
  setup_test_db_with(test_id, true).await
}

/// Setup with configurable single_use flag
async fn setup_test_db_with(test_id: &str, single_use: bool) -> String {
  let token = format!("token-{test_id}");

  // Use the Rust SDK to run migrations (same as production)
  let db = db_signin().await;
  cleanup(&db).await;
  remex_core::db::migrate(&db)
    .await
    .expect("failed to run migrations");

  // Seed user
  db.query("USE NS remex DB remex; CREATE user SET username = $u, email = $e, password = 'pass';")
    .bind(("u", format!("user-{test_id}")))
    .bind(("e", format!("user-{test_id}@example.com")))
    .await
    .unwrap()
    .check()
    .unwrap();

  // Seed enrollment token (uses sha256 hash of the raw token)
  db.query("USE NS remex DB remex; CREATE enrollment_token SET token_hash = crypto::sha256($raw), valid = true, single_use = $su, issued_by = type::record((SELECT VALUE id FROM user WHERE username = $u)[0]);")
    .bind(("raw", Value::String(token.clone())))
    .bind(("su", Value::Bool(single_use)))
    .bind(("u", Value::String(format!("user-{test_id}"))))
    .await
    .unwrap().check().unwrap();

  token
}

/// Check whether a token is still valid by querying the database as root
async fn check_token_valid(raw_token: &str) -> Result<bool, surrealdb::Error> {
  let db = db_signin().await;
  let mut res = db
    .query("USE NS remex DB remex; SELECT VALUE valid FROM enrollment_token WHERE token_hash = crypto::sha256($raw);")
    .bind(("raw", Value::String(raw_token.to_owned())))
    .await?;
  let valid: Vec<bool> = res.take(1).unwrap_or_default();
  Ok(valid.first().copied().unwrap_or(false))
}

/// Execute signup via `ACCESS ... SIGNUP SET` over WS SQL
async fn do_signup(
  token: &str,
  client_name: &str,
  secret: &str,
  hardware_hash: &str,
) -> Result<surrealdb::opt::auth::Token, surrealdb::Error> {
  let db: surrealdb::Surreal<surrealdb::engine::any::Any> = surrealdb::Surreal::init();
  db.connect("ws://localhost:8000")
    .await
    .expect("failed to connect to SurrealDB");
  db.signup(surrealdb::opt::auth::Record {
    access: "endpoint_access".into(),
    namespace: "remex".into(),
    database: "remex".into(),
    params: serde_json::json!({
      "enrollment_token": token,
      "client_name": client_name,
      "hardware_hash": hardware_hash,
      "secret": secret,
    }),
  })
  .await
}

/// Execute signin via `ACCESS ... SIGNIN SET` over WS SQL
async fn do_signin(
  hardware_hash: &str,
  secret: &str,
) -> Result<surrealdb::opt::auth::Token, surrealdb::Error> {
  let db: surrealdb::Surreal<surrealdb::engine::any::Any> = surrealdb::Surreal::init();
  db.connect("ws://localhost:8000")
    .await
    .expect("failed to connect to SurrealDB");
  db.signin(surrealdb::opt::auth::Record {
    access: "endpoint_access".into(),
    namespace: "remex".into(),
    database: "remex".into(),
    params: serde_json::json!({
      "hardware_hash": hardware_hash,
      "secret": secret,
    }),
  })
  .await
}

// ── Signup tests ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn signup_with_valid_params_succeeds() {
  let token = setup_test_db("flat-ok").await;
  match do_signup(&token, "test-host", "test-secret-123", "abc123def456").await {
    Ok(_) => println!("signup succeeded as expected"),
    Err(e) => panic!("signup should have succeeded: {e}"),
  }
}

#[tokio::test]
async fn signup_fails_with_invalid_token() {
  let _token = setup_test_db("invalid-tok").await;
  match do_signup("this-token-does-not-exist", "test-host", "test-secret-123", "abc123").await {
    Ok(t) => panic!("signup with invalid token should have failed, got: {t:?}"),
    Err(e) => println!("signup failed as expected: {e}"),
  }
}

#[tokio::test]
async fn signup_invalidates_single_use_token() {
  let token = setup_test_db("single-use").await;
  // First use — should succeed
  match do_signup(&token, "host-1", "secret-1", "hw-1").await {
    Ok(_) => println!("first signup succeeded as expected"),
    Err(e) => panic!("first signup should have succeeded: {e}"),
  }
  // Second use with same token — should fail (single_use)
  match do_signup(&token, "host-2", "secret-2", "hw-2").await {
    Ok(t) => panic!("second signup with single-use token should have failed, got: {t:?}"),
    Err(e) => println!("second signup failed as expected: {e}"),
  }
}

#[tokio::test]
async fn signup_token_lookup_uses_sha256() {
  let token = setup_test_db("sha256-check").await;
  match do_signup(&token, "host", "secret", "hw").await {
    Ok(_) => println!("signup succeeded as expected"),
    Err(e) => panic!("signup should have succeeded: {e}"),
  }
}

#[tokio::test]
async fn signup_multi_use_token_works_twice() {
  let token = setup_test_db_with("multi-use", false).await;
  // First use — should succeed
  match do_signup(&token, "host-1", "secret-1", "hw-multi-1").await {
    Ok(_) => println!("first use of multi-use token succeeded as expected"),
    Err(e) => panic!("first use of multi-use token should have succeeded: {e}"),
  }
  // Second use with different credentials — should also succeed (multi-use)
  match do_signup(&token, "host-2", "secret-2", "hw-multi-2").await {
    Ok(_) => println!("second use of multi-use token succeeded as expected"),
    Err(e) => panic!("second use of multi-use token should have succeeded: {e}"),
  }
  // Verify the token is still valid in the database
  match check_token_valid(&token).await {
    Ok(true) => println!("multi-use token is still valid after two uses"),
    Ok(false) => panic!("multi-use token was invalidated despite single_use = false"),
    Err(e) => panic!("failed to check token validity: {e}"),
  }
}

// ── Signin tests ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn signin_round_trip() {
  let token = setup_test_db("signin-rt").await;
  let hw_hash = "hw-rt-001";
  let secret = "my-secret-value";
  match do_signup(&token, "test-host", secret, hw_hash).await {
    Ok(_) => println!("signup succeeded as expected"),
    Err(e) => panic!("signup should have succeeded: {e}"),
  }
  match do_signin(hw_hash, secret).await {
    Ok(jwt) => println!("signin succeeded, token length: {}", jwt.access.as_insecure_token().len()),
    Err(e) => panic!("signin should have succeeded: {e}"),
  }
}

#[tokio::test]
async fn signin_with_wrong_secret_fails() {
  let token = setup_test_db("signin-wrong").await;
  match do_signup(&token, "test-host", "correct-secret", "hw-wrong").await {
    Ok(_) => println!("signup succeeded as expected"),
    Err(e) => panic!("signup should have succeeded: {e}"),
  }
  match do_signin("hw-wrong", "wrong-secret").await {
    Ok(t) => panic!("signin with wrong secret should have failed, got: {t:?}"),
    Err(e) => println!("signin failed as expected: {e}"),
  }
}

#[tokio::test]
async fn signin_with_wrong_hardware_hash_fails() {
  let token = setup_test_db("signin-hw").await;
  match do_signup(&token, "test-host", "secret", "hw-real").await {
    Ok(_) => println!("signup succeeded as expected"),
    Err(e) => panic!("signup should have succeeded: {e}"),
  }
  match do_signin("hw-wrong", "secret").await {
    Ok(t) => panic!("signin with wrong hardware hash should have failed, got: {t:?}"),
    Err(e) => println!("signin failed as expected: {e}"),
  }
}
