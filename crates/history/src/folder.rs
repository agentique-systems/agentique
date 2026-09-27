//! The working folder: loading, and saving several files as one atomic change.
//!
//! A save writes each changed file to `<file>.tmp` and syncs it, then writes
//! the save journal (`agentique.pending`: which files to move into place and
//! which to delete) by syncing a temporary copy and renaming it. That rename
//! is the moment the save happens. The files are then moved into place and
//! the journal is removed. Recovery, run before every load, save and commit,
//! repeats the moves of a journal it finds (they are idempotent) and deletes
//! temporary files that no journal names.
use crate::{Error, FORMAT, History, IDENTITY_FILE, Identities, ModelFiles, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, ErrorKind, Write};
use std::path::Path;

const JOURNAL: &str = "agentique.pending";
const TMP: &str = ".tmp";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Journal {
    format: u64,
    write: Vec<String>,
    delete: Vec<String>,
}

impl History {
    /// Reads the working folder again, first finishing or discarding a save
    /// that was interrupted.
    pub fn load(&mut self) -> Result<ModelFiles> {
        self.recover()?;
        let files = read_model_files(&self.dir)?;
        let model = model_files(&files)?;
        self.seen = files;
        Ok(model)
    }

    /// Saves the model files, atomically. Only changed files are written.
    /// Refuses if any model file changed on disk since the last load or save.
    ///
    /// Once the save journal is durable the save has happened: if moving the
    /// files into place fails after that, the next load, save or commit
    /// finishes it.
    pub fn save(&mut self, files: &ModelFiles) -> Result<()> {
        self.save_until(files, None).map(|_| ())
    }

    /// For crash tests: performs at most `steps` file operations of a save
    /// and then stops, as if the process had died there. Returns whether the
    /// save completed.
    #[cfg(any(test, feature = "crash-test"))]
    pub fn save_interrupted(&mut self, files: &ModelFiles, steps: usize) -> Result<bool> {
        self.save_until(files, Some(steps))
    }

    /// Saves, stopping after `stop_after` file operations if given.
    fn save_until(&mut self, files: &ModelFiles, mut stop_after: Option<usize>) -> Result<bool> {
        self.check_unchanged()?;
        let mut next = BTreeMap::new();
        for (path, text) in &files.documents {
            check_path(path)?;
            next.insert(path.clone(), normalize(text));
        }
        next.insert(IDENTITY_FILE.to_owned(), files.identities.to_text());
        let journal = Journal {
            format: FORMAT,
            write: next
                .iter()
                .filter(|(path, text)| self.seen.get(*path) != Some(*text))
                .map(|(path, _)| path.clone())
                .collect(),
            delete: self
                .seen
                .keys()
                .filter(|path| !next.contains_key(*path))
                .cloned()
                .collect(),
        };
        if journal.write.is_empty() && journal.delete.is_empty() {
            return Ok(true);
        }
        for path in &journal.write {
            if !step(&mut stop_after) {
                return Ok(false);
            }
            write_synced(
                &self.dir.join(format!("{path}{TMP}")),
                next[path].as_bytes(),
            )?;
        }
        if !step(&mut stop_after) {
            return Ok(false);
        }
        let pending = self.dir.join(format!("{JOURNAL}{TMP}"));
        write_synced(
            &pending,
            serde_json::to_string(&journal).expect("journal").as_bytes(),
        )?;
        rename(&pending, &self.dir.join(JOURNAL))?;
        sync_dir(&self.dir)?;
        // The save has happened. If moving the files into place fails or
        // stops from here on, recovery finishes it.
        self.seen = next;
        let moved = matches!(apply(&self.dir, &journal, &mut stop_after), Ok(true));
        Ok(moved && step(&mut stop_after) && remove(&self.dir.join(JOURNAL)).is_ok())
    }

    /// Finishes an interrupted save, then refuses if the model files differ
    /// from what was last loaded or saved: they were edited outside the app.
    pub(crate) fn check_unchanged(&self) -> Result<()> {
        self.recover()?;
        let disk = read_model_files(&self.dir)?;
        let changed: BTreeSet<&String> = self
            .seen
            .keys()
            .chain(disk.keys())
            .filter(|path| self.seen.get(*path) != disk.get(*path))
            .collect();
        if changed.is_empty() {
            Ok(())
        } else {
            Err(Error::ChangedOnDisk(changed.into_iter().cloned().collect()))
        }
    }

    /// Finishes a save whose journal is durable; discards one whose journal
    /// is not.
    pub(crate) fn recover(&self) -> Result<()> {
        let journal_path = self.dir.join(JOURNAL);
        match fs::read_to_string(&journal_path) {
            Ok(text) => {
                let journal = parse_journal(&text)?;
                apply(&self.dir, &journal, &mut None)?;
                remove(&journal_path)?;
            }
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        for path in walk(&self.dir)? {
            if let Some(target) = path.strip_suffix(TMP)
                && (target == JOURNAL || is_model_file(target))
            {
                remove(&self.dir.join(&path))?;
            }
        }
        Ok(())
    }
}

/// Counts one file operation against `stop_after`; false when none is left.
fn step(stop_after: &mut Option<usize>) -> bool {
    match stop_after {
        None => true,
        Some(0) => false,
        Some(left) => {
            *left -= 1;
            true
        }
    }
}

/// Moves the journal's files into place and deletes the ones it names.
fn apply(dir: &Path, journal: &Journal, stop_after: &mut Option<usize>) -> Result<bool> {
    for path in &journal.write {
        let tmp = dir.join(format!("{path}{TMP}"));
        // A missing temporary file was already moved into place.
        if tmp.exists() {
            if !step(stop_after) {
                return Ok(false);
            }
            rename(&tmp, &dir.join(path))?;
        }
    }
    for path in &journal.delete {
        if !step(stop_after) {
            return Ok(false);
        }
        match remove(&dir.join(path)) {
            Err(e) if e.kind() != ErrorKind::NotFound => return Err(e.into()),
            _ => {}
        }
    }
    // Every folder that had a file renamed or deleted in it.
    let folders: BTreeSet<&str> = journal
        .write
        .iter()
        .chain(&journal.delete)
        .map(|path| path.rsplit_once('/').map_or("", |(folder, _)| folder))
        .collect();
    for folder in folders {
        sync_dir(&dir.join(folder))?;
    }
    Ok(true)
}

fn parse_journal(text: &str) -> Result<Journal> {
    let invalid = |message: String| Error::Invalid {
        path: JOURNAL.into(),
        message,
    };
    let journal: Journal = serde_json::from_str(text).map_err(|e| invalid(e.to_string()))?;
    if journal.format != FORMAT {
        return Err(Error::UnknownFormat {
            path: JOURNAL.into(),
            found: journal.format,
        });
    }
    for path in journal.write.iter().chain(&journal.delete) {
        if path != IDENTITY_FILE {
            check_path(path)?;
        }
    }
    Ok(journal)
}

pub(crate) fn is_model_file(path: &str) -> bool {
    path == IDENTITY_FILE || path.ends_with(".sysml")
}

/// Splits model files into documents and identities. Refuses files that a
/// git merge left with conflict markers.
pub(crate) fn model_files(files: &BTreeMap<String, String>) -> Result<ModelFiles> {
    for (path, text) in files {
        if has_conflict_markers(text) {
            return Err(Error::ConflictMarkers { path: path.clone() });
        }
    }
    let identities = match files.get(IDENTITY_FILE) {
        Some(text) => Identities::parse(text)?,
        None => Identities::default(),
    };
    let documents = files
        .iter()
        .filter(|(path, _)| path.as_str() != IDENTITY_FILE)
        .map(|(path, text)| (path.clone(), text.clone()))
        .collect();
    Ok(ModelFiles {
        documents,
        identities,
    })
}

fn has_conflict_markers(text: &str) -> bool {
    let mut lines = text.lines();
    lines.any(|line| line.starts_with("<<<<<<<")) && lines.any(|line| line.starts_with(">>>>>>>"))
}

/// Model files in the working folder: relative path to text.
pub(crate) fn read_model_files(dir: &Path) -> Result<BTreeMap<String, String>> {
    let mut files = BTreeMap::new();
    for path in walk(dir)? {
        if is_model_file(&path) {
            let bytes = fs::read(dir.join(&path))?;
            files.insert(path.clone(), text_of(&path, bytes)?);
        }
    }
    Ok(files)
}

/// Model text is UTF-8 with LF line ends; git may check it out with CRLF.
pub(crate) fn text_of(path: &str, bytes: Vec<u8>) -> Result<String> {
    let text = String::from_utf8(bytes).map_err(|_| Error::Invalid {
        path: path.into(),
        message: "not UTF-8".into(),
    })?;
    Ok(normalize(&text))
}

fn normalize(text: &str) -> String {
    if text.contains('\r') {
        text.replace("\r\n", "\n")
    } else {
        text.to_owned()
    }
}

fn check_path(path: &str) -> Result<()> {
    let ok = path.ends_with(".sysml")
        && path
            .split('/')
            .all(|s| !s.is_empty() && !s.starts_with('.') && !s.contains(['\\', ':']));
    if ok {
        Ok(())
    } else {
        Err(Error::Invalid {
            path: path.into(),
            message: "a document path must be relative, use '/', and end in .sysml".into(),
        })
    }
}

/// Files below `dir` as relative paths with `/`, skipping hidden entries
/// such as `.git`.
fn walk(dir: &Path) -> Result<Vec<String>> {
    let mut out = Vec::new();
    let mut stack = vec![(dir.to_path_buf(), String::new())];
    while let Some((path, prefix)) = stack.pop() {
        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let relative = format!("{prefix}{name}");
            if entry.file_type()?.is_dir() {
                stack.push((entry.path(), format!("{relative}/")));
            } else {
                out.push(relative);
            }
        }
    }
    out.sort();
    Ok(out)
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

/// Renames `from` to `to`, replacing `to`.
fn rename(from: &Path, to: &Path) -> io::Result<()> {
    retry(|| fs::rename(from, to))
}

fn remove(path: &Path) -> io::Result<()> {
    retry(|| fs::remove_file(path))
}

/// On Windows another program (a virus scanner, the search indexer, an
/// editor) may hold a file open for a moment, and renaming or deleting it
/// fails. Such failures are retried for up to about a second.
fn retry(mut operation: impl FnMut() -> io::Result<()>) -> io::Result<()> {
    let mut wait = 1;
    loop {
        match operation() {
            Err(e) if cfg!(windows) && wait <= 512 && is_transient(&e) => {
                std::thread::sleep(std::time::Duration::from_millis(wait));
                wait *= 2;
            }
            result => return result,
        }
    }
}

/// ERROR_ACCESS_DENIED, ERROR_SHARING_VIOLATION and ERROR_LOCK_VIOLATION.
fn is_transient(error: &io::Error) -> bool {
    matches!(error.raw_os_error(), Some(5 | 32 | 33))
}

/// Makes renames and deletes in the folder `dir` durable.
fn sync_dir(dir: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // FILE_FLAG_BACKUP_SEMANTICS: needed to open a folder.
        let folder = fs::OpenOptions::new()
            .write(true)
            .custom_flags(0x0200_0000)
            .open(dir)?;
        folder.sync_all()?;
    }
    #[cfg(not(windows))]
    fs::File::open(dir)?.sync_all()?;
    Ok(())
}
