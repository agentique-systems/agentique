//! The launcher (ROADMAP §4.15, C-51; part `Launcher` in
//! `model/Agentique.sysml`): Agentique's installed builds, which one starts,
//! and the last known good one. It depends on no other part of Agentique, so
//! it works when a new build does not, and it never builds, downloads or
//! deletes anything.
//!
//! - [`Registry`]: `builds.json` in the builds folder: the builds, the
//!   current one and the last known good one.
//! - [`Manifest`]: `build.json` in each build's folder: what it was built
//!   from, after which checks, and the digests of its executables.
//! - [`start`]: starts a build and waits for it to report ready (a file it
//!   writes once its window is up); if it exits or does not report ready,
//!   the build is marked failed with the reason, and the last known good
//!   build starts instead, told what happened.
#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const FORMAT: u32 = 1;

/// The Studio's executable in a build's folder.
pub const STUDIO: &str = if cfg!(windows) {
    "agq-studio-native.exe"
} else {
    "agq-studio-native"
};

/// The launcher's executable.
pub const LAUNCHER: &str = if cfg!(windows) {
    "agentique-launcher.exe"
} else {
    "agentique-launcher"
};

/// How long a build may take to report ready.
pub const READY_WITHIN: Duration = Duration::from_secs(120);

/// `%LOCALAPPDATA%\Agentique\builds`, or the folder `AGENTIQUE_BUILDS`
/// names (a test instance gets its own, so it can never adopt into the
/// Operator's).
pub fn default_root() -> PathBuf {
    if let Some(folder) = std::env::var_os("AGENTIQUE_BUILDS").filter(|f| !f.is_empty()) {
        return PathBuf::from(folder);
    }
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_DATA_HOME").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir)
        .join("Agentique")
        .join("builds")
}

/// What became of a build.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum State {
    /// Built, not used yet.
    Built,
    /// Started and reported ready at least once.
    Started,
    /// Did not start or report ready; why.
    Failed { reason: String },
}

/// One build the registry knows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    /// When it was registered (RFC 3339, UTC).
    pub created: String,
    /// The commit it was built from, or empty for one built outside
    /// Agentique (the first, bootstrapped one).
    pub commit: String,
    #[serde(flatten)]
    pub state: State,
}

/// `builds.json`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Registry {
    pub format: u32,
    /// The build the launcher starts.
    pub current: Option<String>,
    /// The newest build that started and reported ready: where a failed
    /// start falls back to.
    pub last_known_good: Option<String>,
    pub builds: Vec<Entry>,
}

impl Registry {
    pub fn path(root: &Path) -> PathBuf {
        root.join("builds.json")
    }

    /// The registry in `root`; empty when there is none. A later format is
    /// refused, never guessed.
    pub fn load(root: &Path) -> Result<Registry, String> {
        let path = Self::path(root);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Registry {
                    format: FORMAT,
                    ..Registry::default()
                });
            }
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        let registry: Registry =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        if registry.format != FORMAT {
            return Err(format!(
                "{} is in format {}; this launcher reads format {FORMAT}",
                path.display(),
                registry.format
            ));
        }
        Ok(registry)
    }

    /// Saves atomically: a temporary file, synced, then renamed.
    pub fn save(&self, root: &Path) -> Result<(), String> {
        write_atomically(
            &Self::path(root),
            serde_json::to_string_pretty(self)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
    }

    pub fn entry(&self, id: &str) -> Option<&Entry> {
        self.builds.iter().find(|e| e.id == id)
    }

    pub fn entry_mut(&mut self, id: &str) -> Option<&mut Entry> {
        self.builds.iter_mut().find(|e| e.id == id)
    }

    /// Adds a build, or replaces the entry with its id.
    pub fn add(&mut self, entry: Entry) {
        self.builds.retain(|e| e.id != entry.id);
        self.builds.push(entry);
    }
}

/// `build.json`: what a build was made from and after which checks.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub format: u32,
    pub id: String,
    pub created: String,
    /// The repository it was built from, and the commit and tree.
    pub repository: String,
    pub commit: String,
    pub tree: String,
    /// The task whose commit it is, if any.
    #[serde(default)]
    pub task: Option<String>,
    /// `cargo --version` of the toolchain that built it.
    pub toolchain: String,
    /// The Claude Agent companion built into it, and its packages.
    pub companion: String,
    pub packages: String,
    /// The required checks of the verification it was built after: (name,
    /// verdict).
    #[serde(default)]
    pub checks: Vec<(String, String)>,
    /// The data formats it reads and writes: rolling back to a build with
    /// other formats needs its data restored.
    pub data_formats: std::collections::BTreeMap<String, u64>,
    /// Each executable and its SHA-256.
    pub executables: Vec<(String, String)>,
}

impl Manifest {
    pub fn path(folder: &Path) -> PathBuf {
        folder.join("build.json")
    }

    pub fn load(folder: &Path) -> Result<Manifest, String> {
        let path = Self::path(folder);
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
    }

    pub fn save(&self, folder: &Path) -> Result<(), String> {
        write_atomically(
            &Self::path(folder),
            serde_json::to_string_pretty(self)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
    }

    /// Whether the executables in `folder` are the ones this manifest
    /// records: the same files with the same digests.
    pub fn matches(&self, folder: &Path) -> Result<(), String> {
        for (file, digest) in &self.executables {
            let actual = file_digest(&folder.join(file))?;
            if &actual != digest {
                return Err(format!(
                    "{file} is not the one that was built (its digest changed)"
                ));
            }
        }
        Ok(())
    }
}

/// The SHA-256 of a file, as hex.
pub fn file_digest(path: &Path) -> Result<String, String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 16];
    loop {
        let n = file
            .read(&mut buffer)
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// Writes a file through a temporary file in the same folder, synced, then
/// renamed into place.
pub fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let folder = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
    let temporary = path.with_extension(format!("tmp.{}", std::process::id()));
    let result = (|| {
        let mut file = std::fs::File::create(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, path)
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_file(&temporary);
        return Err(format!("{}: {e}", path.display()));
    }
    Ok(())
}

/// Now, as RFC 3339 in UTC, without a date library.
pub fn now() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (days, rest) = (seconds / 86_400, seconds % 86_400);
    // Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

/// How a start ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Started {
    /// The build reported ready.
    Ready { id: String },
    /// The build did not start; the last known good one did instead.
    FellBack {
        failed: String,
        reason: String,
        running: String,
    },
    /// Neither started: what to tell the Operator, and where the log is.
    Failed { reason: String, log: PathBuf },
}

/// Starts build `id` from `root` with `args`, waits for it to report ready
/// (`--ready-file`), and records the outcome: ready makes it current and the
/// last known good one; a failure is recorded with its reason and the last
/// known good build starts instead, with `--recovered-from`.
pub fn start(root: &Path, id: &str, args: &[String], ready_within: Duration) -> Started {
    let mut registry = match Registry::load(root) {
        Ok(registry) => registry,
        Err(reason) => return failed(root, reason),
    };
    let log = root.join("launcher.log");
    match try_start(root, id, args, ready_within) {
        Ok(()) => {
            if let Some(entry) = registry.entry_mut(id) {
                entry.state = State::Started;
            }
            registry.current = Some(id.to_string());
            registry.last_known_good = Some(id.to_string());
            let _ = registry.save(root);
            append(
                &log,
                &format!("{} started {id} and it reported ready", now()),
            );
            Started::Ready { id: id.to_string() }
        }
        Err(reason) => {
            append(&log, &format!("{} {id} did not start: {reason}", now()));
            if let Some(entry) = registry.entry_mut(id) {
                entry.state = State::Failed {
                    reason: reason.clone(),
                };
            }
            let fallback = registry.last_known_good.clone().filter(|good| good != id);
            if registry.current.as_deref() == Some(id) {
                registry.current = fallback.clone();
            }
            let _ = registry.save(root);
            let Some(good) = fallback else {
                return failed(
                    root,
                    format!(
                        "{id} did not start ({reason}), and there is no last known good build to return to"
                    ),
                );
            };
            let mut recovery = args.to_vec();
            recovery.extend(["--recovered-from".to_string(), id.to_string()]);
            match try_start(root, &good, &recovery, ready_within) {
                Ok(()) => {
                    append(
                        &log,
                        &format!("{} started the last known good build {good}", now()),
                    );
                    Started::FellBack {
                        failed: id.to_string(),
                        reason,
                        running: good,
                    }
                }
                Err(second) => failed(
                    root,
                    format!(
                        "{id} did not start ({reason}), nor did the last known good build {good} ({second})"
                    ),
                ),
            }
        }
    }
}

fn failed(root: &Path, reason: String) -> Started {
    let log = root.join("launcher.log");
    append(&log, &format!("{} {reason}", now()));
    Started::Failed { reason, log }
}

fn append(log: &Path, line: &str) {
    use std::io::Write;
    if let Some(folder) = log.parent() {
        let _ = std::fs::create_dir_all(folder);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
    {
        let _ = writeln!(file, "{line}");
    }
}

/// Starts one build and waits until it reports ready, exits or runs out of
/// time. What it wrote to its error output goes to `start-failure.log` in
/// its folder.
fn try_start(root: &Path, id: &str, args: &[String], ready_within: Duration) -> Result<(), String> {
    let folder = root.join(id);
    let exe = folder.join(STUDIO);
    if !exe.is_file() {
        return Err(format!("{} is missing", exe.display()));
    }
    Manifest::load(&folder)?.matches(&folder)?;
    let ready = folder.join("ready");
    let _ = std::fs::remove_file(&ready);
    let errors = folder.join("start-failure.log");
    let stderr = std::fs::File::create(&errors)
        .map(Stdio::from)
        .unwrap_or_else(|_| Stdio::null());
    let mut child = Command::new(&exe)
        .args(args)
        .arg("--ready-file")
        .arg(&ready)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(stderr)
        .spawn()
        .map_err(|e| format!("it could not start: {e}"))?;
    let started = Instant::now();
    loop {
        if ready.is_file() {
            // It runs on; the launcher's work is done.
            return Ok(());
        }
        if let Ok(Some(status)) = child.try_wait() {
            let tail = std::fs::read_to_string(&errors).unwrap_or_default();
            let tail: Vec<&str> = tail.lines().rev().take(8).collect();
            return Err(format!(
                "it exited ({status}) before it was ready{}",
                if tail.is_empty() {
                    String::new()
                } else {
                    format!(
                        ": {}",
                        tail.into_iter().rev().collect::<Vec<_>>().join(" | ")
                    )
                }
            ));
        }
        if started.elapsed() > ready_within {
            let _ = child.kill();
            return Err(format!(
                "it did not report ready within {} s",
                ready_within.as_secs()
            ));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Waits until the process holding `lock` (the Studio handing over) has
/// ended: the operating system releases its lock with it.
pub fn wait_for_release(lock: &Path, within: Duration) -> Result<(), String> {
    let started = Instant::now();
    loop {
        match std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(lock)
        {
            Ok(file) => {
                if file.try_lock().is_ok() {
                    return Ok(());
                }
            }
            // Gone: nothing holds it.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(_) => {}
        }
        if started.elapsed() > within {
            return Err(format!(
                "the running Agentique did not end within {} s",
                within.as_secs()
            ));
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str) -> Entry {
        Entry {
            id: id.into(),
            created: now(),
            commit: "abc".into(),
            state: State::Built,
        }
    }

    #[test]
    fn the_registry_is_saved_atomically_and_a_later_format_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let mut registry = Registry::load(dir.path()).unwrap();
        assert!(registry.builds.is_empty());
        registry.add(entry("b1"));
        registry.current = Some("b1".into());
        registry.save(dir.path()).unwrap();
        let loaded = Registry::load(dir.path()).unwrap();
        assert_eq!(loaded, registry);
        std::fs::write(Registry::path(dir.path()), r#"{"format":2,"builds":[]}"#).unwrap();
        assert!(Registry::load(dir.path()).unwrap_err().contains("format 2"));
    }

    #[test]
    fn a_manifest_says_whether_its_executables_are_the_ones_built() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(STUDIO), b"the studio").unwrap();
        let manifest = Manifest {
            format: FORMAT,
            id: "b1".into(),
            executables: vec![(
                STUDIO.into(),
                file_digest(&dir.path().join(STUDIO)).unwrap(),
            )],
            ..Manifest::default()
        };
        manifest.save(dir.path()).unwrap();
        assert!(
            Manifest::load(dir.path())
                .unwrap()
                .matches(dir.path())
                .is_ok()
        );
        std::fs::write(dir.path().join(STUDIO), b"another studio").unwrap();
        assert!(
            manifest
                .matches(dir.path())
                .unwrap_err()
                .contains("digest changed")
        );
    }

    /// A build without a manifest is never started: nothing vouches for its
    /// executable.
    #[test]
    fn a_build_without_a_manifest_is_not_started() {
        let dir = tempfile::tempdir().unwrap();
        let folder = dir.path().join("b1");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join(STUDIO), b"not checked").unwrap();
        let refused = try_start(dir.path(), "b1", &[], Duration::from_secs(1)).unwrap_err();
        assert!(refused.contains("build.json"), "{refused}");
    }

    #[test]
    fn dates_are_utc_rfc_3339() {
        let now = now();
        assert_eq!(now.len(), 20, "{now}");
        assert!(now.ends_with('Z') && now.as_bytes()[10] == b'T', "{now}");
        assert!(now.starts_with("20"), "{now}");
    }

    #[test]
    fn a_released_lock_lets_the_handover_go_on() {
        let dir = tempfile::tempdir().unwrap();
        let lock = dir.path().join("handover.lock");
        let held = std::fs::File::create(&lock).unwrap();
        held.lock().unwrap();
        let path = lock.clone();
        let waiter = std::thread::spawn(move || wait_for_release(&path, Duration::from_secs(10)));
        std::thread::sleep(Duration::from_millis(300));
        drop(held);
        assert!(waiter.join().unwrap().is_ok());
        // Held past the time: refused.
        let held = std::fs::File::create(&lock).unwrap();
        held.lock().unwrap();
        assert!(wait_for_release(&lock, Duration::from_millis(300)).is_err());
    }
}
