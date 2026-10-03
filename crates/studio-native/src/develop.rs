//! Developing Agentique in Agentique (ROADMAP §2.8, §4.15, §6.6, C-51):
//!
//! - "Develop Agentique" opens Agentique's own repository as an ordinary
//!   project, whose model is Agentique's self-model (`model/`), and gives it
//!   Agentique's own required checks (`AGENTS.md`) when it has none.
//! - Builds: a release build of one commit (a worktree holding exactly that
//!   commit) into its own folder under the launcher's builds folder, with a
//!   manifest of what it was built from and after which checks.
//! - "Try this build": the build starts as a test instance, with its own app
//!   data, Claude Agent sessions and builds folder, on its own copy of the
//!   repository at the build's commit.
//! - "Use this build": after checking that the executable is the one built
//!   and that its commit is the repository's (the reviewed, integrated one),
//!   that it reads and writes the same data formats, the app data is backed
//!   up and the launcher takes over: it starts the build and falls back to
//!   the last known good one if it does not start.

use crate::studio::{Dirty, Studio};
use agq_execution::{Executor, Program, Scope, git};
use agq_implementation::task::ProjectChecks;
use agq_launcher::{Entry, Manifest, Registry, State};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Duration;

/// Whether `folder` is Agentique's repository: its self-model and workspace.
pub fn is_agentique(folder: &Path) -> bool {
    folder.join("model/Agentique.sysml").is_file()
        && folder.join("Cargo.toml").is_file()
        && folder.join("claude-agent/package.json").is_file()
}

/// The data formats this build reads and writes. A build with other
/// formats cannot be adopted until rolling its data back is supported.
pub fn data_formats() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("settings".to_string(), crate::settings::FORMAT),
        (
            "conversation".to_string(),
            agq_assistant::conversation::FORMAT,
        ),
        (
            "links".to_string(),
            u64::from(agq_implementation::links::FORMAT),
        ),
        ("checks".to_string(), u64::from(ProjectChecks::FORMAT)),
        ("jobs".to_string(), u64::from(agq_execution::jobs::FORMAT)),
        (
            "runs".to_string(),
            u64::from(agq_simulation::result::FORMAT),
        ),
        ("builds".to_string(), u64::from(agq_launcher::FORMAT)),
    ])
}

/// What `--describe` prints: what this build is.
pub fn describe() -> serde_json::Value {
    serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "companion": agq_assistant::claude_agent::companion_digest(),
        "packages": agq_assistant::claude_agent::packages_digest(),
        "dataFormats": data_formats(),
    })
}

/// Work on builds running on its own thread.
pub enum BuildWork {
    Building(Receiver<Result<Manifest, String>>, Arc<AtomicBool>),
}

#[derive(Default)]
pub struct BuildsState {
    pub work: Option<BuildWork>,
    pub message: Option<String>,
    /// The build that did not start, when the launcher fell back to this
    /// one (`--recovered-from`), and why.
    pub recovered: Option<(String, String)>,
    /// Held while Agentique hands over to the launcher; the launcher goes on
    /// once the operating system releases it with this process.
    pub handover: Option<std::fs::File>,
    /// The Studio should end (after handing over).
    pub quit: bool,
    /// The exit code when it ends: the supervisor's handover code.
    pub exit: Option<i32>,
    /// The Operator chose "Use this build" for this build; the card asks
    /// once more.
    pub confirm_use: Option<String>,
    ready_written: bool,
}

impl Studio {
    /// Agentique's repository, if this Agentique knows where it is: the
    /// folder named by `AGENTIQUE_REPOSITORY`, else the repository it was
    /// built from (when that folder is still there).
    pub fn agentique_repository(&self) -> Option<PathBuf> {
        let candidates = [
            std::env::var_os("AGENTIQUE_REPOSITORY").map(PathBuf::from),
            Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")),
        ];
        candidates
            .into_iter()
            .flatten()
            .filter(|folder| is_agentique(folder))
            .find_map(|folder| folder.canonicalize().ok())
            .map(plain_path)
    }

    /// Opens Agentique's own repository as the project, and gives it
    /// Agentique's required checks if it has none yet.
    pub fn develop_agentique(&mut self) {
        let Some(folder) = self.agentique_repository() else {
            self.status = "This Agentique does not know where its repository is: open it with Open project…, or set AGENTIQUE_REPOSITORY.".into();
            self.mark(Dirty::STATUS);
            return;
        };
        self.open_project(&folder);
        if self
            .project
            .as_ref()
            .is_none_or(|p| !is_agentique(p.folder()))
        {
            return;
        }
        if self.project_checks().commands.is_empty() {
            match self.save_project_checks(&ProjectChecks::agentique()) {
                Ok(()) => {
                    self.status = format!(
                        "Agentique's own repository is open. Its tasks must pass Agentique's required checks: {}.",
                        ProjectChecks::agentique()
                            .commands
                            .iter()
                            .map(|c| c.id.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                Err(error) => {
                    self.status = format!("The required checks could not be saved: {error}");
                }
            }
        }
        self.mark(Dirty::ALL);
    }

    /// The launcher's builds folder.
    pub fn builds_root(&self) -> PathBuf {
        agq_launcher::default_root()
    }

    /// The build this process runs, if it runs from the builds folder.
    pub fn running_build(&self) -> Option<String> {
        let exe = std::env::current_exe().ok()?.canonicalize().ok()?;
        let root = self.builds_root().canonicalize().ok()?;
        let relative = exe.parent()?.strip_prefix(&root).ok()?;
        relative.to_str().map(str::to_string)
    }

    /// The builds, newest first.
    pub fn builds(&self) -> Result<Registry, String> {
        let mut registry = Registry::load(&self.builds_root())?;
        registry.builds.reverse();
        Ok(registry)
    }

    /// Builds Agentique from the repository's current commit on its own
    /// thread: a worktree holding exactly that commit, `cargo build
    /// --release` into the builds folder's own target folder, then the
    /// executables, their digests and the manifest.
    pub fn build_agentique(&mut self) {
        if self.develop.work.is_some() || self.refused_to_agents("building Agentique") {
            return;
        }
        let Some(repository) = self.agentique_repository() else {
            self.develop.message =
                Some("This Agentique does not know where its repository is.".into());
            self.mark(Dirty::LAYOUT);
            return;
        };
        let commit = match git::head(&repository) {
            Ok(head) => head.commit,
            Err(refusal) => {
                self.develop.message =
                    Some(format!("The repository has no commit to build: {refusal}"));
                self.mark(Dirty::LAYOUT);
                return;
            }
        };
        if !self.execution_choice().trusted {
            self.develop.message = Some(
                "Building runs Cargo with your rights: allow trusted-local execution for Agentique's project first (the Run panel or the palette).".into(),
            );
            self.mark(Dirty::LAYOUT);
            return;
        }
        let reviewed = self.reviewed_task(&commit);
        let root = self.builds_root();
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("agentique-build".into())
            .spawn(move || {
                let _ = sender.send(build(&repository, &commit, reviewed, &root, flag));
            })
            .ok();
        self.develop.work = Some(BuildWork::Building(receiver, cancel));
        self.develop.message = Some(
            "Building Agentique (a release build: several minutes, and a few GB in the builds folder the first time)…".into(),
        );
        self.mark(Dirty::LAYOUT | Dirty::STATUS);
    }

    pub fn cancel_build(&mut self) {
        if let Some(BuildWork::Building(_, cancel)) = &self.develop.work {
            cancel.store(true, std::sync::atomic::Ordering::SeqCst);
            self.develop.message = Some("Stopping the build…".into());
        }
    }

    /// Takes a finished build. Returns whether anything changed.
    pub fn poll_build(&mut self) -> bool {
        let Some(BuildWork::Building(receiver, _)) = &self.develop.work else {
            return false;
        };
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => Err("The build stopped without a result.".into()),
        };
        self.develop.work = None;
        self.develop.message = Some(match result {
            Ok(manifest) => format!(
                "Built {} from {} ({}). Try it, then use it.",
                manifest.id,
                short(&manifest.commit),
                manifest.toolchain
            ),
            Err(why) => format!("Not built: {why}"),
        });
        self.mark(Dirty::LAYOUT | Dirty::STATUS);
        true
    }

    /// Starts build `id` as a test instance: its own app data, Claude Agent
    /// sessions and builds folder, on its own copy of the repository at the
    /// build's commit. The running Agentique and its data are untouched.
    pub fn try_build(&mut self, id: &str) {
        if self.refused_to_agents("trying a build") {
            return;
        }
        let result = (|| -> Result<PathBuf, String> {
            let folder = self.builds_root().join(id);
            let manifest = Manifest::load(&folder)?;
            manifest.matches(&folder)?;
            let repository = PathBuf::from(&manifest.repository);
            let trial = folder.join("try");
            let copy = trial.join("agentique");
            if !copy.join("model/Agentique.sysml").is_file() {
                git::checkout_worktree(&repository, &format!("try-{id}"), &copy, &manifest.commit)
                    .map_err(|e| e.to_string())?;
            }
            std::process::Command::new(folder.join(agq_launcher::STUDIO))
                .arg("--session")
                .arg(trial.join("session").join("studio-session.json"))
                .arg("--project")
                .arg(&copy)
                .env("AGENTIQUE_BUILDS", trial.join("builds"))
                .env("AGENTIQUE_REPOSITORY", &copy)
                .spawn()
                .map_err(|e| format!("it could not start: {e}"))?;
            Ok(copy)
        })();
        self.develop.message = Some(match result {
            Ok(copy) => format!(
                "Started {id} as a test instance on its own copy ({}) with its own data. Close it when you are done; nothing here changed.",
                copy.display()
            ),
            Err(why) => format!("Could not try {id}: {why}"),
        });
        self.mark(Dirty::LAYOUT | Dirty::STATUS);
    }

    /// Why build `id` cannot be adopted now, or `None` when it can: its
    /// executables must be the ones built, its commit the repository's
    /// (reviewed and integrated), and its data formats this build's.
    pub fn adoption_blocker(&self, id: &str) -> Option<String> {
        let folder = self.builds_root().join(id);
        let manifest = match Manifest::load(&folder) {
            Ok(manifest) => manifest,
            Err(why) => return Some(why),
        };
        if let Err(why) = manifest.matches(&folder) {
            return Some(why);
        }
        if let Ok(registry) = Registry::load(&self.builds_root())
            && let Some(Entry {
                state: State::Failed { reason },
                ..
            }) = registry.entry(id)
        {
            return Some(format!("it did not start before: {reason}"));
        }
        if !manifest.repository.is_empty() {
            match git::head(Path::new(&manifest.repository)) {
                Ok(head) if head.commit != manifest.commit => {
                    return Some(format!(
                        "it was built from {}, but the repository is at {}: build the integrated commit",
                        short(&manifest.commit),
                        short(&head.commit)
                    ));
                }
                Err(refusal) => return Some(format!("the repository cannot be read: {refusal}")),
                Ok(_) => {}
            }
        }
        let formats = data_formats();
        if manifest.data_formats != formats {
            let changed: Vec<String> = manifest
                .data_formats
                .iter()
                .filter(|(name, format)| formats.get(*name) != Some(format))
                .map(|(name, format)| format!("{name} (format {format})"))
                .collect();
            return Some(format!(
                "it reads and writes other data formats ({}); returning to this version afterwards would need its data restored, which Agentique does not do yet",
                changed.join(", ")
            ));
        }
        None
    }

    /// Hands over to build `id`: checks it ([`Studio::adoption_blocker`]),
    /// keeps this version as the one to return to, backs up the app data,
    /// installs the launcher if it is missing, saves the session, and starts
    /// the launcher, which waits for this process to end, starts the build
    /// on the same project, and falls back to the last known good build if
    /// it does not start. This process then ends.
    pub fn use_build(&mut self, id: &str) -> Result<(), String> {
        if self.refused_to_agents("restarting in another build") {
            return Err("restarting in another build is the Operator's own".into());
        }
        if let Some(why) = self.adoption_blocker(id) {
            return Err(format!("{id} cannot be used: {why}."));
        }
        let root = self.builds_root();
        self.keep_this_version()?;
        self.save_session();
        let backup = root.join(id).join(format!(
            "data-backup-{}",
            agq_launcher::now().replace([':', '-'], "")
        ));
        let data = self
            .session_path
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf();
        back_up(&data, &backup)?;
        let launcher = root.join(agq_launcher::LAUNCHER);
        if !launcher.is_file() {
            std::fs::copy(root.join(id).join(agq_launcher::LAUNCHER), &launcher)
                .map_err(|e| format!("the launcher could not be installed: {e}"))?;
        }
        if self.args.supervised {
            // The supervising launcher is this Studio's parent: it starts the
            // build named here, on this session and the project open now,
            // once this process has ended (C-53).
            let mut args = vec![
                "--adopted".to_string(),
                id.to_string(),
                "--session".to_string(),
                self.session_path.display().to_string(),
            ];
            if let Some(project) = &self.project {
                args.extend([
                    "--project".to_string(),
                    project.folder().display().to_string(),
                ]);
            }
            agq_launcher::Handover::new(id, args).save(&root)?;
            self.develop.exit = Some(agq_launcher::HANDOVER_EXIT);
            self.develop.quit = true;
            self.status = format!("Handing over to {id}…");
            self.mark(Dirty::STATUS);
            return Ok(());
        }
        let lock = root.join("handover.lock");
        let file = std::fs::File::create(&lock).map_err(|e| e.to_string())?;
        file.lock().map_err(|e| e.to_string())?;
        let mut command = std::process::Command::new(&launcher);
        command
            .arg("--adopt")
            .arg(id)
            .arg("--wait")
            .arg(&lock)
            .arg("--session")
            .arg(&self.session_path);
        if let Some(project) = &self.project {
            command.arg("--project").arg(project.folder());
        }
        command
            .spawn()
            .map_err(|e| format!("the launcher could not start: {e}"))?;
        self.develop.handover = Some(file);
        self.develop.quit = true;
        self.status = format!("Handing over to {id}…");
        self.mark(Dirty::STATUS);
        Ok(())
    }

    /// Registers this running version as a build to return to, if it is
    /// not one already (the first, bootstrapped Agentique).
    pub fn keep_this_version(&mut self) -> Result<String, String> {
        if let Some(id) = self.running_build() {
            return Ok(id);
        }
        let root = self.builds_root();
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let digest = agq_launcher::file_digest(&exe)?;
        let id = format!("bootstrap-{}", &digest[..10]);
        let folder = root.join(&id);
        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        std::fs::copy(&exe, folder.join(agq_launcher::STUDIO)).map_err(|e| e.to_string())?;
        let mut executables = vec![(agq_launcher::STUDIO.to_string(), digest)];
        let launcher = exe.with_file_name(agq_launcher::LAUNCHER);
        if launcher.is_file() {
            std::fs::copy(&launcher, folder.join(agq_launcher::LAUNCHER))
                .map_err(|e| e.to_string())?;
            executables.push((
                agq_launcher::LAUNCHER.to_string(),
                agq_launcher::file_digest(&launcher)?,
            ));
        }
        Manifest {
            format: agq_launcher::FORMAT,
            id: id.clone(),
            created: agq_launcher::now(),
            toolchain: "built outside Agentique".into(),
            companion: agq_assistant::claude_agent::companion_digest(),
            packages: agq_assistant::claude_agent::packages_digest(),
            data_formats: data_formats(),
            executables,
            ..Manifest::default()
        }
        .save(&folder)?;
        let mut registry = Registry::load(&root)?;
        if registry.entry(&id).is_none() {
            registry.add(Entry {
                id: id.clone(),
                created: agq_launcher::now(),
                commit: String::new(),
                state: State::Started,
            });
        }
        registry.current.get_or_insert_with(|| id.clone());
        registry.last_known_good.get_or_insert_with(|| id.clone());
        registry.save(&root)?;
        Ok(id)
    }

    /// The launcher's `--ready-file`: written once the window is up.
    pub fn mark_ready(&mut self) {
        if self.develop.ready_written {
            return;
        }
        self.develop.ready_written = true;
        // Started for an adoption: ready only once the check after adoption
        // passed; otherwise end, so the launcher returns to the last known
        // good build (C-53, ROADMAP §4.16).
        if let Some(problem) = self.adoption_check() {
            eprintln!("The check after adoption failed: {problem}");
            self.control.close_endpoint();
            std::process::exit(4);
        }
        if let Some(path) = &self.args.ready_file {
            let _ = std::fs::write(path, "ready");
        }
    }

    /// The check after adoption, when started for one: this process is the
    /// adopted build (its manifest and executables), and the project the
    /// session names opened. `None` when it passed or was not asked for.
    pub fn adoption_check(&self) -> Option<String> {
        let id = self.args.adopted.as_deref()?;
        if self.running_build().as_deref() != Some(id) {
            return Some(format!(
                "this process is not the build {id} (it runs {})",
                self.running_build()
                    .as_deref()
                    .unwrap_or("outside the builds folder")
            ));
        }
        let folder = self.builds_root().join(id);
        if let Err(problem) = Manifest::load(&folder).and_then(|m| m.matches(&folder)) {
            return Some(problem);
        }
        // The project it was told to open, else the one its session names.
        let expected = self.args.project.clone().or(self.session.project.clone());
        if let Some(expected) = expected
            && expected.join("model").is_dir()
            && self.project.as_ref().map(|p| p.folder().to_path_buf()) != Some(expected.clone())
        {
            return Some(format!(
                "the project {} did not open: {}",
                expected.display(),
                self.status
            ));
        }
        None
    }

    /// Stops everything that runs (C-53, before a handover): the
    /// Assistant's turn, a run, the implementation task and its checks, a
    /// build; and waits up to `within` for them to end, so the processes
    /// they started end with them. Returns what had not ended by then.
    pub fn stop_work(&mut self, within: Duration) -> Vec<&'static str> {
        self.end_turn();
        self.stop_run();
        self.stop_task();
        self.stop_checks();
        self.cancel_build();
        let started = std::time::Instant::now();
        loop {
            self.poll_runs();
            self.poll_task();
            self.poll_checks();
            self.poll_build();
            let running: Vec<&'static str> = [
                (self.runs.active.is_some(), "a run"),
                (
                    self.implementation.task.is_some(),
                    "the implementation task",
                ),
                (self.implementation.checking(), "the checks"),
                (self.develop.work.is_some(), "a build"),
            ]
            .into_iter()
            .filter_map(|(running, what)| running.then_some(what))
            .collect();
            if running.is_empty() || started.elapsed() >= within {
                return running;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Started by the launcher after build `failed` did not start: say so.
    pub fn note_recovery(&mut self) {
        let Some(failed) = self.args.recovered_from.clone() else {
            return;
        };
        let reason = Registry::load(&self.builds_root())
            .ok()
            .and_then(|registry| match &registry.entry(&failed)?.state {
                State::Failed { reason } => Some(reason.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "see launcher.log in the builds folder".into());
        self.status = format!(
            "The build {failed} did not start ({reason}). This is the last known good version; your project and its data are as they were."
        );
        self.develop.recovered = Some((failed, reason));
    }
}

/// The task whose commit a build is built from, and the outcome of each of
/// its required checks, for the build's manifest.
pub struct Reviewed {
    pub task: String,
    pub checks: Vec<(String, String)>,
}

impl Studio {
    /// The task whose task commit `commit` is (integrated, so reviewed), if
    /// any.
    pub fn reviewed_task(&self, commit: &str) -> Option<Reviewed> {
        let store = self.job_store()?;
        store.list().into_iter().find_map(|job| {
            let outcome: crate::tasks::TaskOutcome =
                serde_json::from_value(job.detail["outcome"].clone()).ok()?;
            (outcome.commit == commit).then(|| Reviewed {
                task: job.id.clone(),
                checks: outcome
                    .verification
                    .outcomes()
                    .into_iter()
                    .map(|o| (o.name, o.verdict.label().to_string()))
                    .collect(),
            })
        })
    }
}

/// `\\?\C:\x` → `C:\x`.
fn plain_path(path: PathBuf) -> PathBuf {
    let text = path.to_string_lossy();
    match text.strip_prefix(r"\\?\") {
        Some(rest) => PathBuf::from(rest),
        None => path,
    }
}

fn short(commit: &str) -> &str {
    &commit[..commit.len().min(10)]
}

/// The build itself (on its own thread).
fn build(
    repository: &Path,
    commit: &str,
    reviewed: Option<Reviewed>,
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
            task: reviewed.as_ref().map(|r| r.task.clone()),
            toolchain,
            companion: described["companion"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            packages: described["packages"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            checks: reviewed.map(|r| r.checks).unwrap_or_default(),
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

/// Copies the app data's own files (settings, session, usage, each
/// project's conversation, checks, trust choice and job records) into
/// `backup`; build output, worktrees, runs and the runtime are left out.
fn back_up(data: &Path, backup: &Path) -> Result<(), String> {
    let skip = ["worktrees", "targets", "runs", "claude-agent"];
    let mut pending = vec![data.to_path_buf()];
    while let Some(folder) = pending.pop() {
        let entries = std::fs::read_dir(&folder)
            .map_err(|e| format!("{} cannot be backed up: {e}", folder.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("{}: {e}", folder.display()))?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if !skip.contains(&name.as_str()) {
                    pending.push(path);
                }
            } else if name.ends_with(".json") || name.ends_with(".md") || name.ends_with(".sysml") {
                let relative = path.strip_prefix(data).map_err(|e| e.to_string())?;
                let target = backup.join(relative);
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                std::fs::copy(&path, &target).map_err(|e| format!("{}: {e}", path.display()))?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn this_repository_is_agentique() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert!(is_agentique(&repository));
        assert!(!is_agentique(Path::new(env!("CARGO_MANIFEST_DIR"))));
    }

    #[test]
    fn a_build_describes_its_data_formats_and_companion() {
        let described = describe();
        assert_eq!(described["dataFormats"]["conversation"], 2);
        assert_eq!(described["dataFormats"]["settings"], 1);
        assert_eq!(
            described["companion"],
            agq_assistant::claude_agent::companion_digest()
        );
    }

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
            Some(Reviewed {
                task: "t1".into(),
                checks: vec![("Build".into(), "passed".into())],
            }),
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

    #[test]
    fn the_backup_keeps_the_app_datas_own_files_only() {
        let dir = std::env::temp_dir().join(format!("agq-backup-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let data = dir.join("Agentique");
        std::fs::create_dir_all(data.join("projects/p-1/jobs")).unwrap();
        std::fs::create_dir_all(data.join("projects/p-1/worktrees/t1")).unwrap();
        std::fs::write(data.join("settings.json"), "{}").unwrap();
        std::fs::write(data.join("projects/p-1/conversation.json"), "{}").unwrap();
        std::fs::write(data.join("projects/p-1/jobs/t1.json"), "{}").unwrap();
        std::fs::write(data.join("projects/p-1/worktrees/t1/big.rs"), "x").unwrap();
        let backup = dir.join("backup");
        back_up(&data, &backup).unwrap();
        assert!(backup.join("settings.json").is_file());
        assert!(backup.join("projects/p-1/conversation.json").is_file());
        assert!(backup.join("projects/p-1/jobs/t1.json").is_file());
        assert!(!backup.join("projects/p-1/worktrees").exists());
    }
}
