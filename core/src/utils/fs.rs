use std::fs::{self, File};
use std::io::{self, Write};

/// A utility struct for reading from and writing to a specific file
pub struct FileInterface {
  file_path: String,
}

impl FileInterface {
  /// Creates a new FileInterface with the specified file path
  pub fn new(file_path: &str) -> Self {
    FileInterface {
      file_path: file_path.to_string(),
    }
  }

  /// Reads the contents of the file
  /// Returns the file contents as a String, or an error if reading fails
  pub fn read(&self) -> Result<String, io::Error> { fs::read_to_string(&self.file_path) }

  /// Writes content to the file, replacing the entire contents
  /// Returns true if successful, false otherwise
  pub fn write(&self, content: &str) -> Result<bool, io::Error> {
    let mut file = File::create(&self.file_path)?;
    file.write_all(content.as_bytes())?;
    Ok(true)
  }
}
