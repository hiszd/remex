use actix::Actor;
use remex_core::actors::server::RemexServer;
pub mod secret;

//SERVER

const ADDRESS: &str = "127.0.0.1:4269";

fn get_or_generate_secret() -> String {
  // Try to get existing secret
  match secret::get_secret("server") {
    Ok(Some(secret_val)) => {
      println!("Using existing secret from file");
      secret_val
    }
    Err(e) => {
      tracing::error!("Failed to get secret: {}", e);
      let secret_val = generate_secret();
      secret::save_secret("server", secret_val.clone()).expect("Failed to save secret");
      secret_val
    }
    _ => {
      // Generate a new secret
      println!("No secret found, generating new secret");
      let secret_val = generate_secret();
      secret::save_secret("server", secret_val.clone()).expect("Failed to save secret");
      secret_val
    }
  }
}

fn generate_secret() -> String {
  use rand::Rng;
  const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789!@#$%^&*()_+-=[]{}|;:,.<>?";
  const SECRET_LENGTH: usize = 64;

  let mut rng = rand::rng();
  let secret_val: String = (0..SECRET_LENGTH)
    .map(|_| {
      let idx = rng.random_range(0..CHARSET.len());
      CHARSET[idx] as char
    })
    .collect();

  secret_val
}

#[actix_web::main]
async fn main() {
  tracing_subscriber::fmt::init();

  // Get or generate the secret before starting the server
  let secret_string = get_or_generate_secret();
  println!("Full secret (for copying to endpoint): {}", secret_string);

  let server = RemexServer {
    sessions: remex_core::sessionmap::SessionMap::default(),
    migrated: false,
    secret: Some(secret_string.clone()),
  }
  .start();
  remex_core::actors::session::tcp_server(ADDRESS, &secret_string, server).await;
}
