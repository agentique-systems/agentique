//! Reserve publication destinations before expensive preparation. Empty reservations
//! are removed on failure; populated artifacts remain available for diagnosis.
use serde_json::Value;
use std::{
    fs::{File, OpenOptions},
    io::{Seek, Write},
    path::{Path, PathBuf},
};

pub struct ReservedFile {
    path: PathBuf,
    file: Option<File>,
}
impl ReservedFile {
    pub fn new(path: impl Into<PathBuf>) -> std::io::Result<Self> {
        let path = path.into();
        std::fs::create_dir_all(
            path.parent()
                .ok_or_else(|| std::io::Error::other("output parent"))?,
        )?;
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        let output = Self {
            path,
            file: Some(file),
        };
        // Prove that the destination accepts actual data, then retain its exclusive
        // reservation. Free space for the corpus remains the external watchdog's gate.
        let mut file = output.file();
        if let Err(error) = (|| {
            file.write_all(b"\n")?;
            file.set_len(0)?;
            file.rewind()?;
            file.sync_all()
        })() {
            let failed_path = output.path.clone();
            drop(output);
            let _ = std::fs::remove_file(failed_path);
            return Err(error);
        }
        Ok(output)
    }
    pub fn replacement(destination: &Path) -> std::io::Result<Self> {
        // Opening an existing destination without truncation detects read-only
        // files before closure; acceptance content remains untouched.
        match OpenOptions::new().write(true).open(destination) {
            Ok(file) => file.sync_all()?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        Self::new(destination.with_extension("json.pending"))
    }
    pub fn file(&self) -> &File {
        self.file.as_ref().expect("live reservation")
    }
    pub fn write_json(&self, value: &Value) -> Result<(), Box<dyn std::error::Error>> {
        let mut file = self.file();
        file.rewind()?;
        file.set_len(0)?;
        serde_json::to_writer_pretty(&mut file, value)?;
        writeln!(file)?;
        file.sync_all()?;
        Ok(())
    }
    pub fn replace(mut self, destination: &Path) -> std::io::Result<()> {
        // Close before rename for a portable Windows/POSIX replacement boundary.
        drop(self.file.take());
        std::fs::rename(&self.path, destination)
    }
}
impl Drop for ReservedFile {
    fn drop(&mut self) {
        let empty = self
            .file
            .as_ref()
            .and_then(|file| file.metadata().ok())
            .is_some_and(|m| m.len() == 0);
        drop(self.file.take());
        if empty {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn directory() -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("agq-publication-output-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        path
    }
    #[test]
    fn reservations_reject_collisions_and_clean_only_their_empty_files() {
        let directory = directory();
        let existing = directory.join("existing.json");
        std::fs::write(&existing, b"retained").unwrap();
        let early = directory.join("early.json");
        let reserved = ReservedFile::new(&early).unwrap();
        assert!(ReservedFile::new(&existing).is_err());
        assert!(ReservedFile::new(&early).is_err());
        drop(reserved);
        assert!(!early.exists());
        assert_eq!(std::fs::read(&existing).unwrap(), b"retained");
        std::fs::remove_file(existing).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
    #[test]
    fn staged_replacement_is_durable_and_leaves_no_pending_reservation() {
        let directory = directory();
        let destination = directory.join("accepted.json");
        let pending = directory.join("accepted.json.pending");
        std::fs::write(&destination, b"old receipt").unwrap();
        let staged = ReservedFile::replacement(&destination).unwrap();
        staged
            .write_json(&json!({"complete":true,"long_diagnostic":"first report"}))
            .unwrap();
        staged.write_json(&json!({"complete":true})).unwrap();
        staged.replace(&destination).unwrap();
        assert!(!pending.exists());
        assert_eq!(
            serde_json::from_slice::<Value>(&std::fs::read(&destination).unwrap()).unwrap(),
            json!({"complete":true})
        );
        std::fs::remove_file(destination).unwrap();
        std::fs::remove_dir(directory).unwrap();
    }
}
