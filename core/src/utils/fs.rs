use std::{
  fs::{
    self,
    File,
  },
  io::{
    self,
    Write,
  },
  path::Path,
  time::SystemTime,
};

use chrono::{
  self,
  DateTime,
  NaiveDateTime,
  TimeZone,
  Utc,
};

#[derive(Debug, Clone)]
struct FileInformation {
  #[allow(dead_code)]
  filename: String,
  size: u64,
  last_modified: chrono::NaiveDateTime,
}

/// A utility struct for reading from and writing to a specific file,
/// with built-in checks for external modifications.
pub struct FileInterface {
  file_path: String,
  content: String,
  file_info: FileInformation,
}

impl FileInterface {
  /// Creates a new FileInterface with the specified file path.
  /// Reads the file content and metadata upon creation.
  pub fn new(file_path: &str) -> Result<Self, io::Error> {
    let path = Path::new(file_path);
    let filename = path
      .file_name()
      .and_then(|name| name.to_str())
      .unwrap_or("unknown")
      .to_string();

    let (content, size, last_modified) = match fs::metadata(path) {
      Ok(metadata) => {
        let content = fs::read_to_string(path)?;
        let size = metadata.len();
        let last_modified_sys: SystemTime = metadata.modified()?;
        let last_modified: NaiveDateTime = DateTime::<Utc>::from(last_modified_sys).naive_utc();
        (content, size, last_modified)
      }
      Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
        // File doesn\'t exist, initialize with empty content and default metadata
        (
          String::new(),
          0,
          Utc
            .timestamp_opt(0, 0)
            .single()
            .unwrap_or_else(Utc::now)
            .naive_utc(),
        )
      }
      Err(e) => return Err(e),
    };

    Ok(FileInterface {
      file_path: file_path.to_string(),
      content,
      file_info: FileInformation {
        filename,
        size,
        last_modified,
      },
    })
  }

  /// Retrieves the internally stored file content.
  /// Before returning, it checks if the file on disk has been updated
  /// externally and re-reads it if necessary to synchronize.
  pub fn get(&mut self) -> Result<String, io::Error> {
    let path = Path::new(&self.file_path);
    let disk_metadata = fs::metadata(path)?;
    let disk_last_modified_sys: SystemTime = disk_metadata.modified()?;
    let disk_last_modified: NaiveDateTime =
      DateTime::<Utc>::from(disk_last_modified_sys).naive_utc();

    if disk_last_modified > self.file_info.last_modified {
      // File on disk is newer, re-read and update internal state
      self.content = fs::read_to_string(path)?;
      self.file_info.size = disk_metadata.len();
      self.file_info.last_modified = disk_last_modified;
    }

    Ok(self.content.clone())
  }

  /// Sets the internally stored content and writes it to the file.
  /// This operation will fail if the file on disk has been modified
  /// more recently than our last known state, preventing accidental overwrites.
  /// Returns `true` if the content was written, `false` if skipped due to external modification.
  pub fn set(&mut self, new_content: &str) -> Result<bool, io::Error> {
    let path = Path::new(&self.file_path);

    // Check for external modifications
    if path.exists() {
      let disk_metadata = fs::metadata(path)?;
      let disk_last_modified_sys: SystemTime = disk_metadata.modified()?;
      let disk_last_modified: NaiveDateTime =
        DateTime::<Utc>::from(disk_last_modified_sys).naive_utc();

      if disk_last_modified > self.file_info.last_modified {
        // External modification detected, do not overwrite
        return Ok(false);
      }
    }

    if self.content != new_content {
      let mut file = File::create(path)?;
      file.write_all(new_content.as_bytes())?;

      // Update internal state after successful write
      self.content = new_content.to_string();
      let new_metadata = fs::metadata(path)?;
      let new_last_modified_sys: SystemTime = new_metadata.modified()?;
      self.file_info.size = new_metadata.len();
      self.file_info.last_modified = DateTime::<Utc>::from(new_last_modified_sys).naive_utc();
      Ok(true)
    } else {
      // Content is the same, no write needed
      Ok(true)
    }
  }
}
