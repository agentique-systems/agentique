//! Execution (ROADMAP §4.15; part `Execution` in
//! `models/agentique/Agentique.sysml`): every side effect outside the System
//! State, as typed operations with a scope.
//!
//! - [`Scope`]: the repository root, where writes may go (a task's
//!   worktree, and inside it only the allowed paths), and what is protected.
//!   Paths are canonicalised; anything escaping the scope is refused.
//! - [`Executor`]: reads, lists, writes and deletes files inside the scope,
//!   and runs allow-listed commands with a scrubbed environment (no
//!   provider keys, tokens or other secrets), Cargo offline unless the
//!   network is allowed, a timeout, and cancellation that ends the whole
//!   process tree. [`Isolation`] says what that does and does not protect.
//! - [`git`]: worktrees for tasks, the patch a task made, and integration
//!   that checks the base revision again and never touches uncommitted work.
//! - [`jobs`]: jobs with stable ids, states and a journal that records each
//!   side effect before and after it happens, so recovery never repeats a
//!   completed one.
//!
//! A worktree isolates edits; it is not a sandbox. On this host there is no
//! process, filesystem or network sandbox, so the Studio runs anything here
//! only in the explicit trusted-local mode the Operator turns on.
#![forbid(unsafe_code)]

pub mod git;
pub mod jobs;
pub mod process;

pub use process::{Finished, Interactive, Program};

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

/// Why an operation was not carried out. Nothing happened.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The path leaves the scope, or is not a plain relative path.
    OutsideScope(String),
    /// The path is inside the scope but not where this task may write.
    NotWritable(String),
    /// The path is protected (a protected test, a requirement's check).
    Protected(String),
    /// The command is not one this executor runs.
    NotAllowed(String),
    /// The operation needs trusted-local execution, which is off.
    NotTrusted,
    /// Reading or writing failed.
    Io(String),
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::OutsideScope(path) => {
                write!(f, "`{path}` is outside the repository this task may use")
            }
            Refusal::NotWritable(path) => {
                write!(f, "`{path}` is outside the paths this task may change")
            }
            Refusal::Protected(path) => write!(
                f,
                "`{path}` is protected: the task may not change it (a contract test, a scenario or an evaluation)"
            ),
            Refusal::NotAllowed(what) => write!(f, "`{what}` is not a command this task may run"),
            Refusal::NotTrusted => f.write_str(
                "running code needs trusted-local execution, which is off for this project",
            ),
            Refusal::Io(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for Refusal {}

/// Where an executor may read and write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scope {
    /// The folder everything is relative to (canonical).
    root: PathBuf,
    /// Relative paths (files or folders, `/`-separated) that may be written;
    /// empty means nothing may be written.
    writable: Vec<String>,
    /// Relative paths that may never be written, even inside `writable`.
    protected: Vec<String>,
}

impl Scope {
    /// A read-only scope rooted at an existing folder.
    pub fn read_only(root: &Path) -> Result<Scope, Refusal> {
        let root = dunce_canonical(root)?;
        Ok(Scope {
            root,
            writable: Vec::new(),
            protected: Vec::new(),
        })
    }

    /// A scope that may write under `writable` (an empty entry: anywhere
    /// in the root) and never under `protected`.
    pub fn writable(root: &Path, writable: &[&str], protected: &[&str]) -> Result<Scope, Refusal> {
        let mut scope = Scope::read_only(root)?;
        scope.writable = writable.iter().map(|p| normalise(p)).collect();
        scope.protected = protected.iter().map(|p| normalise(p)).collect();
        Ok(scope)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn protected(&self) -> &[String] {
        &self.protected
    }

    /// The absolute path for a relative one inside the scope. Refuses
    /// absolute paths, drive or UNC prefixes, `..`, alternate data streams,
    /// reserved device names, and anything a link would lead out of.
    pub fn resolve(&self, relative: &str) -> Result<PathBuf, Refusal> {
        let refuse = || Refusal::OutsideScope(relative.to_string());
        if relative.trim().is_empty() || relative.contains('\0') {
            return Err(refuse());
        }
        let path = Path::new(relative);
        let mut joined = self.root.clone();
        for component in path.components() {
            match component {
                Component::Normal(part) => {
                    let text = part.to_str().ok_or_else(refuse)?;
                    if !plain_name(text) {
                        return Err(refuse());
                    }
                    joined.push(part);
                }
                Component::CurDir => {}
                _ => return Err(refuse()),
            }
        }
        // The nearest existing ancestor must stay inside the root once links
        // are followed.
        let mut existing = joined.as_path();
        while !existing.exists() {
            existing = existing.parent().ok_or_else(refuse)?;
        }
        let real = dunce_canonical(existing)?;
        if !real.starts_with(&self.root) {
            return Err(refuse());
        }
        Ok(joined)
    }

    /// `relative` in canonical `/` form, if it may be written.
    pub fn check_write(&self, relative: &str) -> Result<PathBuf, Refusal> {
        let absolute = self.resolve(relative)?;
        let normal = normalise(relative);
        if self
            .protected
            .iter()
            .any(|p| normal == *p || normal.starts_with(&format!("{p}/")))
        {
            return Err(Refusal::Protected(normal));
        }
        if !self
            .writable
            .iter()
            .any(|w| w.is_empty() || normal == *w || normal.starts_with(&format!("{w}/")))
        {
            return Err(Refusal::NotWritable(normal));
        }
        Ok(absolute)
    }
}

/// `a\b/./c` → `a/b/c`.
fn normalise(path: &str) -> String {
    path.replace('\\', "/")
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect::<Vec<_>>()
        .join("/")
}

/// A file or folder name that means only itself on Windows and elsewhere.
fn plain_name(name: &str) -> bool {
    const RESERVED: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    !name.is_empty()
        && !name.contains([':', '*', '?', '"', '<', '>', '|'])
        && !name.ends_with(['.', ' '])
        && !RESERVED.contains(&stem.as_str())
}

/// A canonical path without Windows' `\\?\` prefix, so prefixes compare.
fn dunce_canonical(path: &Path) -> Result<PathBuf, Refusal> {
    let canonical = path
        .canonicalize()
        .map_err(|e| Refusal::Io(format!("{}: {e}", path.display())))?;
    let text = canonical.to_string_lossy();
    Ok(match text.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC\\") => PathBuf::from(rest),
        _ => canonical,
    })
}

/// What running code here does and does not protect, in plain words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Isolation {
    /// No sandbox: processes run with the Operator's rights.
    TrustedLocal,
}

impl Isolation {
    /// The isolation this host offers.
    pub fn available() -> Isolation {
        Isolation::TrustedLocal
    }

    pub fn describe(self) -> &'static str {
        match self {
            Isolation::TrustedLocal => {
                "Trusted-local: builds, tests and the harness run on this machine with your rights. \
                 Their environment is scrubbed of provider keys, tokens and other secrets, and Cargo \
                 runs offline unless you allow the network. Agentique's own writes stay inside the \
                 task's worktree and its allowed paths; build scripts and tests are not sandboxed, \
                 and a worktree isolates edits, not processes."
            }
        }
    }
}

/// Carries out operations inside a scope.
#[derive(Clone, Debug)]
pub struct Executor {
    scope: Scope,
    /// Trusted-local execution is on for this project.
    trusted: bool,
    /// Commands may use the network (Cargo online).
    network: bool,
    /// Where Cargo builds (shared by a project's tasks, outside the repository).
    target_dir: Option<PathBuf>,
    cancel: Arc<AtomicBool>,
}

/// Environment variables passed to commands; everything else is dropped.
const KEPT: [&str; 22] = [
    "PATH",
    "PATHEXT",
    "SystemRoot",
    "SYSTEMROOT",
    "SystemDrive",
    "windir",
    "ComSpec",
    "TEMP",
    "TMP",
    "HOME",
    "USERPROFILE",
    "HOMEDRIVE",
    "HOMEPATH",
    "APPDATA",
    "LOCALAPPDATA",
    "ProgramData",
    "CARGO_HOME",
    "RUSTUP_HOME",
    "RUSTUP_TOOLCHAIN",
    "NUMBER_OF_PROCESSORS",
    "PROCESSOR_ARCHITECTURE",
    "LANG",
];

/// Names that look like secrets are never passed, even if kept above.
fn secret(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    ["KEY", "TOKEN", "SECRET", "PASSWORD", "CREDENTIAL", "AUTH"]
        .iter()
        .any(|word| upper.contains(word))
}

impl Executor {
    pub fn new(scope: Scope) -> Executor {
        Executor {
            scope,
            trusted: false,
            network: false,
            target_dir: None,
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Turns trusted-local execution on (the Operator's choice, per project).
    pub fn trusted(mut self, trusted: bool) -> Executor {
        self.trusted = trusted;
        self
    }

    pub fn network(mut self, allowed: bool) -> Executor {
        self.network = allowed;
        self
    }

    pub fn target_dir(mut self, dir: PathBuf) -> Executor {
        self.target_dir = Some(dir);
        self
    }

    /// Cancelling ends the command running now and refuses new ones.
    pub fn cancel_flag(mut self, cancel: Arc<AtomicBool>) -> Executor {
        self.cancel = cancel;
        self
    }

    pub fn scope(&self) -> &Scope {
        &self.scope
    }

    pub fn read(&self, relative: &str) -> Result<String, Refusal> {
        let path = self.scope.resolve(relative)?;
        std::fs::read_to_string(&path).map_err(|e| Refusal::Io(format!("{relative}: {e}")))
    }

    /// The files under `relative` (a folder; "" for the root), as relative
    /// paths, skipping `target` and `.git`, at most `limit`.
    pub fn list(&self, relative: &str, limit: usize) -> Result<Vec<String>, Refusal> {
        let start = if relative.is_empty() || relative == "." {
            self.scope.root.clone()
        } else {
            self.scope.resolve(relative)?
        };
        let mut out = Vec::new();
        let mut pending = vec![start];
        while let Some(dir) = pending.pop() {
            let mut entries: Vec<PathBuf> = std::fs::read_dir(&dir)
                .map_err(|e| Refusal::Io(format!("{}: {e}", dir.display())))?
                .filter_map(Result::ok)
                .map(|e| e.path())
                .collect();
            entries.sort();
            for entry in entries.into_iter().rev() {
                let name = entry.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if matches!(name, "target" | ".git" | ".agentique") {
                    continue;
                }
                if entry.is_dir() {
                    pending.push(entry);
                } else if let Ok(rel) = entry.strip_prefix(&self.scope.root) {
                    out.push(rel.to_string_lossy().replace('\\', "/"));
                    if out.len() >= limit {
                        out.sort();
                        return Ok(out);
                    }
                }
            }
        }
        out.sort();
        Ok(out)
    }

    /// Writes a file, creating folders; only where the scope allows.
    pub fn write(&self, relative: &str, text: &str) -> Result<(), Refusal> {
        let path = self.scope.check_write(relative)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| Refusal::Io(format!("{relative}: {e}")))?;
        }
        std::fs::write(&path, text).map_err(|e| Refusal::Io(format!("{relative}: {e}")))
    }

    pub fn delete(&self, relative: &str) -> Result<(), Refusal> {
        let path = self.scope.check_write(relative)?;
        std::fs::remove_file(&path).map_err(|e| Refusal::Io(format!("{relative}: {e}")))
    }

    /// The environment a command gets: the kept variables that are set,
    /// minus anything that looks like a secret, plus Cargo's settings.
    pub fn environment(&self) -> Vec<(String, String)> {
        let mut env: Vec<(String, String)> = std::env::vars()
            .filter(|(name, _)| KEPT.iter().any(|k| k.eq_ignore_ascii_case(name)) && !secret(name))
            .collect();
        if !self.network {
            env.push(("CARGO_NET_OFFLINE".into(), "true".into()));
        }
        if let Some(target) = &self.target_dir {
            env.push((
                "CARGO_TARGET_DIR".into(),
                target.to_string_lossy().into_owned(),
            ));
        }
        env.push(("CARGO_TERM_COLOR".into(), "never".into()));
        env
    }

    /// Runs an allow-listed command in the scope's root (or a folder
    /// inside it), to completion or the timeout.
    pub fn run(
        &self,
        program: &Program,
        folder: &str,
        timeout: Duration,
    ) -> Result<Finished, Refusal> {
        let cwd = self.command_folder(program, folder)?;
        process::run(program, &cwd, &self.environment(), timeout, &self.cancel)
    }

    /// Starts an allow-listed command that talks over its standard input and
    /// output (the harness).
    pub fn spawn(&self, program: &Program, folder: &str) -> Result<Interactive, Refusal> {
        let cwd = self.command_folder(program, folder)?;
        process::spawn(program, &cwd, &self.environment(), self.cancel.clone())
    }

    fn command_folder(&self, program: &Program, folder: &str) -> Result<PathBuf, Refusal> {
        if !self.trusted {
            return Err(Refusal::NotTrusted);
        }
        program.check()?;
        if folder.is_empty() || folder == "." {
            Ok(self.scope.root.clone())
        } else {
            self.scope.resolve(folder)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> (tempfile::TempDir, Scope) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::create_dir_all(dir.path().join("tests")).unwrap();
        std::fs::write(dir.path().join("tests/contract.rs"), "// protected").unwrap();
        let scope = Scope::writable(
            dir.path(),
            &["src", "tests", "Cargo.toml"],
            &["tests/contract.rs"],
        )
        .unwrap();
        (dir, scope)
    }

    #[test]
    fn paths_that_escape_the_scope_are_refused() {
        let (_dir, scope) = scope();
        for bad in [
            "..",
            "../outside.txt",
            "src/../../outside.txt",
            "/etc/passwd",
            r"C:\Windows\win.ini",
            r"\\server\share\x",
            "C:relative.txt",
            "src/file.rs:stream",
            "src/CON",
            "src/nul.txt",
            "src/name. ",
            "",
        ] {
            assert!(
                matches!(scope.resolve(bad), Err(Refusal::OutsideScope(_))),
                "{bad:?} was not refused"
            );
        }
        assert!(scope.resolve("src/lib.rs").is_ok());
        assert!(scope.resolve("./src/./lib.rs").is_ok());
    }

    #[test]
    fn writes_go_only_where_the_task_may_write() {
        let (dir, scope) = scope();
        let executor = Executor::new(scope);
        executor.write("src/lib.rs", "pub fn f() {}").unwrap();
        assert_eq!(executor.read("src/lib.rs").unwrap(), "pub fn f() {}");
        executor.write("src/deep/mod.rs", "").unwrap();
        assert_eq!(
            executor.write("tests/contract.rs", "// weakened"),
            Err(Refusal::Protected("tests/contract.rs".into()))
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("tests/contract.rs")).unwrap(),
            "// protected"
        );
        assert!(matches!(
            executor.write("README.md", "x"),
            Err(Refusal::NotWritable(_))
        ));
        assert!(matches!(
            executor.write("../escape.rs", "x"),
            Err(Refusal::OutsideScope(_))
        ));
        let files = executor.list("", 100).unwrap();
        assert_eq!(
            files,
            ["src/deep/mod.rs", "src/lib.rs", "tests/contract.rs"]
        );
        // A whole worktree: anywhere in it but the protected paths.
        let whole =
            Executor::new(Scope::writable(dir.path(), &[""], &["tests/contract.rs"]).unwrap());
        whole.write("README.md", "x").unwrap();
        assert!(whole.write("tests/contract.rs", "x").is_err());
        assert!(whole.write("../escape.rs", "x").is_err());
    }

    #[test]
    fn commands_need_trusted_local_execution_and_an_allowed_program() {
        let (_dir, scope) = scope();
        let executor = Executor::new(scope.clone());
        let cargo = Program::cargo(&["--version"]);
        assert_eq!(
            executor
                .run(&cargo, "", Duration::from_secs(5))
                .unwrap_err(),
            Refusal::NotTrusted
        );
        let executor = Executor::new(scope).trusted(true);
        let shell = Program::new("cmd", &["/c", "del", "*"]);
        assert!(matches!(
            executor.run(&shell, "", Duration::from_secs(5)),
            Err(Refusal::NotAllowed(_))
        ));
        let publish = Program::cargo(&["publish"]);
        assert!(matches!(
            executor.run(&publish, "", Duration::from_secs(5)),
            Err(Refusal::NotAllowed(_))
        ));
    }

    #[test]
    fn an_allowed_command_runs_with_the_scrubbed_environment() {
        let (_dir, scope) = scope();
        let executor = Executor::new(scope).trusted(true);
        let finished = executor
            .run(&Program::cargo(&["--version"]), "", Duration::from_secs(60))
            .unwrap();
        assert!(finished.success, "{finished:?}");
        assert!(finished.stdout.starts_with("cargo "), "{}", finished.stdout);
        assert!(finished.summary().starts_with("succeeded"));
        let cancel = Arc::new(AtomicBool::new(true));
        let (_dir, scope) = super::tests::scope();
        let cancelled = Executor::new(scope)
            .trusted(true)
            .cancel_flag(cancel)
            .run(&Program::cargo(&["--version"]), "", Duration::from_secs(60))
            .unwrap();
        assert!(cancelled.cancelled);
    }

    #[test]
    fn the_environment_carries_no_secrets() {
        let (_dir, scope) = scope();
        let executor = Executor::new(scope);
        let env = executor.environment();
        assert!(env.iter().all(|(name, _)| !secret(name)));
        assert!(
            env.iter()
                .any(|(name, value)| name == "CARGO_NET_OFFLINE" && value == "true")
        );
        assert!(secret("DEEPSEEK_API_KEY"));
        assert!(secret("GITHUB_TOKEN"));
        assert!(!secret("PATH"));
    }
}
