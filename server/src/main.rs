use actix::Actor;
use remex_core::actors::server::RemexServer;
use remex_core::utils::generate_secret;

mod pnpm;
mod secret;

use remex_server::web;

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
      let secret_val = generate_secret(true);
      secret::save_secret("server", secret_val.clone()).expect("Failed to save secret");
      secret_val
    }
    _ => {
      // Generate a new secret
      println!("No secret found, generating new secret");
      let secret_val = generate_secret(true);
      secret::save_secret("server", secret_val.clone()).expect("Failed to save secret");
      secret_val
    }
  }
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
  let web_server = web::start_web_server();
  let web_handle = web_server.handle();
  tokio::spawn(web_server);

  tokio::spawn(pnpm::start_server());

  let tcp_fut = remex_core::actors::session::tcp_server(ADDRESS, &secret_string, server);

  tokio::select! {
    _ = tokio::signal::ctrl_c() => {
      println!("Ctrl-C received, shutting down gracefully...");
    }
    _ = tcp_fut => {
      println!("TCP server exited unexpectedly.");
    }
  }

  // Stop the web server properly
  web_handle.stop(true).await;
}
