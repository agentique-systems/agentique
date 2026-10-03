//! Builds of Agentique (ROADMAP §4.15; C-51, C-53): a release build of one
//! commit into its own folder with a manifest, for trying and adopting; and
//! a debug build of a cycle's checkout, for the test instance its evaluator
//! operates. Each builds a fresh checkout of exactly its commit, so its
//! output never comes from another working copy's files.

use agq_execution::git;
use agq_execution::process::Program;
use agq_execution::{Executor, Scope};
use agq_launcher::{Entry, Manifest, Registry, State};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

/// A commit's short form, for names.
pub fn short(commit: &str) -> &str {
    commit.get(..10).unwrap_or(commit)
}

/// Builds `commit` of `repository` in release mode into its own folder of
/// `root` (the builds folder), with its manifest, and registers it. `task`
/// and `checks` say what it was built after. The checkout is removed after.
pub fn build(
    repository: &Path,
    commit: &str,
    task: Option<String>,
    checks: Vec<(String, String)>,
    root: &Path,
    cancel: Arc<AtomicBool>,
) -> Result<Manifest, String> {
    let created = agq_launcher::now();
    let id = format!(
        "{}-{}",
        created
            .replace(['-', ':', 'T', 'Z'], "")
            .get(..12)
            .unwrap_or("build"),
        short(commit)
    );
    let source = root.join("src").join(&id);
    git::checkout_worktree(repository, &format!("build-{id}"), &source, commit)
        .map_err(|e| e.to_string())?;
    let result = (|| -> Result<Manifest, String> {
        let executor = Executor::new(Scope::read_only(&source).map_err(|e| e.to_string())?)
            .trusted(true)
            .target_dir(root.join("target"))
            .cancel_flag(cancel);
        let toolchain = executor
            .run(&Program::cargo(&["--version"]), "", Duration::from_secs(60))
            .map(|f| f.stdout.trim().to_string())
            .unwrap_or_default();
        let built = executor
            .run(
                &Program::cargo(&[
                    "build",
                    "--release",
                    "--offline",
                    "-p",
                    "agq-studio-native",
                    "-p",
                    "agq-launcher",
                ]),
                "",
                Duration::from_secs(3600),
            )
            .map_err(|e| e.to_string())?;
        if !built.success {
            return Err(format!(
                "cargo build {}: {}",
                built.summary(),
                agq_execution::process::last_lines(&built.stderr, 12)
            ));
        }
        let folder = root.join(&id);
        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        let mut executables = Vec::new();
        for file in [agq_launcher::STUDIO, agq_launcher::LAUNCHER] {
            let from = root.join("target").join("release").join(file);
            std::fs::copy(&from, folder.join(file))
                .map_err(|e| format!("{}: {e}", from.display()))?;
            executables.push((
                file.to_string(),
                agq_launcher::file_digest(&folder.join(file))?,
            ));
        }
        // The build says what it is (its data formats, its companion).
        let described = std::process::Command::new(folder.join(agq_launcher::STUDIO))
            .arg("--describe")
            .output()
            .map_err(|e| format!("the build could not describe itself: {e}"))?;
        let described: serde_json::Value = serde_json::from_slice(&described.stdout)
            .map_err(|e| format!("the build's description is not JSON: {e}"))?;
        let data_formats = described["dataFormats"]
            .as_object()
            .map(|formats| {
                formats
                    .iter()
                    .map(|(k, v)| (k.clone(), v.as_u64().unwrap_or(0)))
                    .collect()
            })
            .unwrap_or_default();
        let manifest = Manifest {
            format: agq_launcher::FORMAT,
            id: id.clone(),
            created: created.clone(),
            repository: repository.display().to_string(),
            commit: commit.to_string(),
            tree: git::tree_of(repository, commit).map_err(|e| e.to_string())?,
            task,
            toolchain,
            companion: described["companion"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            packages: described["packages"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            checks,
            data_formats,
            executables,
        };
        manifest.save(&folder)?;
        let mut registry = Registry::load(root)?;
        registry.add(Entry {
            id: id.clone(),
            created,
            commit: commit.to_string(),
            state: State::Built,
        });
        registry.save(root)?;
        Ok(manifest)
    })();
    let _ = git::remove_worktree(repository, &format!("build-{id}"));
    result
}

/// A debug build of the Studio in `checkout` (a cycle's clean checkout),
/// into `target`: the executable a cycle's test instance runs.
pub fn debug_studio(
    checkout: &Path,
    target: &Path,
    cancel: Arc<AtomicBool>,
) -> Result<PathBuf, String> {
    let executor = Executor::new(Scope::read_only(checkout).map_err(|e| e.to_string())?)
        .trusted(true)
        .target_dir(target.to_path_buf())
        .cancel_flag(cancel);
    let built = executor
        .run(
            &Program::cargo(&["build", "--offline", "-p", "agq-studio-native"]),
            "",
            Duration::from_secs(3600),
        )
        .map_err(|e| e.to_string())?;
    if !built.success {
        return Err(format!(
            "the cycle's Studio did not build: {}",
            agq_execution::process::last_lines(&built.stderr, 12)
        ));
    }
    let exe = target.join("debug").join(agq_launcher::STUDIO);
    exe.is_file()
        .then_some(exe)
        .ok_or_else(|| "the cycle's Studio was built, but its executable is missing".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The build pipeline on a stand-in workspace with the same two package
    /// names (Agentique's own release build takes many minutes and GB): a
    /// detached worktree at exactly the commit, `cargo build --release`, the
    /// executables copied and digested, the build describing itself, the
    /// manifest and the registry; the worktree is removed and no branch is
    /// left; a build whose executable is changed afterwards is refused.
    #[test]
    fn a_build_is_made_from_exactly_one_commit_with_its_manifest() {
        let dir = std::env::temp_dir().join(format!("agq-build-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let repository = dir.join("repository");
        let studio_main = r#"fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--describe") {
        println!("{{\"version\":\"0.0.0\",\"companion\":\"c1\",\"packages\":\"p1\",\"dataFormats\":{{\"settings\":1}}}}");
    }
}
"#;
        let files = [
            (
                "Cargo.toml",
                "[workspace]\nmembers = [\"studio\", \"launcher\"]\nresolver = \"3\"\n",
            ),
            (
                "studio/Cargo.toml",
                "[package]\nname = \"agq-studio-native\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
            ),
            ("studio/src/main.rs", studio_main),
            (
                "launcher/Cargo.toml",
                "[package]\nname = \"agq-launcher\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[[bin]]\nname = \"agentique-launcher\"\npath = \"src/main.rs\"\n",
            ),
            ("launcher/src/main.rs", "fn main() {}\n"),
            (".gitignore", "/target\n"),
        ];
        for (path, text) in files {
            let file = repository.join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, text).unwrap();
        }
        let commit = git::init_and_commit(&repository, "Stand-in").unwrap();
        // A later commit: the build is of the one asked for.
        std::fs::write(
            repository.join("launcher/src/main.rs"),
            "fn main() { later() }\n",
        )
        .unwrap();
        git::init_and_commit(&repository, "Later, and broken").unwrap();
        let root = dir.join("builds");
        let manifest = build(
            &repository,
            &commit,
            Some("t1".into()),
            vec![("Build".into(), "passed".into())],
            &root,
            Arc::new(AtomicBool::new(false)),
        )
        .expect("built");
        let folder = root.join(&manifest.id);
        assert_eq!(manifest.commit, commit);
        assert_eq!(manifest.task.as_deref(), Some("t1"));
        assert_eq!(manifest.companion, "c1");
        assert_eq!(manifest.data_formats.get("settings"), Some(&1));
        assert!(
            manifest.toolchain.starts_with("cargo "),
            "{}",
            manifest.toolchain
        );
        manifest
            .matches(&folder)
            .expect("the executables are the ones built");
        let registry = Registry::load(&root).unwrap();
        assert!(matches!(
            registry.entry(&manifest.id).map(|e| &e.state),
            Some(State::Built)
        ));
        assert!(
            !root.join("src").join(&manifest.id).exists(),
            "worktree removed"
        );
        let branches = std::process::Command::new("git")
            .args(["branch", "--list"])
            .current_dir(&repository)
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default();
        assert!(!branches.contains("agentique/"), "{branches}");
        // Tampered with afterwards: refused.
        std::fs::write(folder.join(agq_launcher::LAUNCHER), b"changed").unwrap();
        assert!(Manifest::load(&folder).unwrap().matches(&folder).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
