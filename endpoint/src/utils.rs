use std::process::Command;

/// Runs a command on the system and returns combined stdout and stderr as a string.
pub fn run_command(program: &str, args: &[&str]) -> Result<String, std::io::Error> {
  let output = Command::new(program).args(args).output()?;

  let stdout = String::from_utf8_lossy(&output.stdout);
  let stderr = String::from_utf8_lossy(&output.stderr);

  let mut result = String::new();
  if !stdout.is_empty() {
    result.push_str(&stdout);
  }
  if !stderr.is_empty() {
    if !result.is_empty() {
      result.push('\n');
    }
    result.push_str(&stderr);
  }

  Ok(result)
}

/// Derives the authentication type
/// 1: ClientSecret
/// 2: Secret
pub fn derive_auth(secret: Option<&String>, args_secret: Option<&String>) -> anyhow::Result<u8> {
  tracing::info!(
    "Deriving authentication type from client_secret: {:?}, server_secret: {:?}",
    &secret,
    &args_secret
  );
  // TODO: Create a way for the client not to upend things if it cannot connect to the server, but
  // it can connect to the database with a token that is still valid.
  // In other words, revisit the logic involved in determining what would cause the client to fail
  match (secret, args_secret) {
    (Some(_), _) => {
      tracing::info!("Using client secret");
      Ok(1)
    }
    (None, Some(_)) => {
      tracing::info!("Using server secret");
      Ok(2)
    }
    _ => Err(anyhow::anyhow!("No viable authentication provided to connect to the server. Restart client with a valid --secret.")),
  }
}
