use std::process::Stdio;

use tokio::{
  io::{
    AsyncBufReadExt,
    BufReader,
  },
  process::Command,
};

pub async fn start_server() {
  // Initialize tracing subscriber first (e.g., tracing_subscriber::fmt::init())

  let mut child = Command::new("pnpm")
    .current_dir("frontend")
    .arg("run") // Or "run", "dev", etc.
    .arg("dev")
    .arg("--host")
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .kill_on_drop(true)
    .spawn()
    .expect("Failed to spawn pnpm server");

  // Take the stdout handle to read from it
  let stdout = child
    .stdout
    .take()
    .expect("child did not have a handle to stdout");

  // Spawn a dedicated task to process the logs
  tokio::spawn(async move {
    let mut reader = BufReader::new(stdout).lines();

    while let Ok(Some(line)) = reader.next_line().await {
      tracing::info!(target: "pnpm_server", "{}", line);
    }
  });

  // Wait for the process to finish in the background
  let status = child.wait().await.expect("failed to wait on child");
  tracing::info!("PNPM server exited with status: {}", status);
}
