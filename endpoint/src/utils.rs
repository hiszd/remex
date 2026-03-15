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
