//! The implementation side of the Studio (ROADMAP §4.15, W8.2–W8.3):
//! implementation links, the per-project trusted-local switch, the executor
//! every process goes through, rounds of implementation checks on a
//! background thread, drift, and opening linked code in an editor.
use crate::studio::{Dirty, Studio};
use agq_execution::{Executor, Isolation, Scope};
use agq_implementation::checks::{contract_shapes, linked_tests, module_boundaries};
use agq_implementation::{CheckReport, ImplementationCheck, Links};
use agq_language::ElementId;
use agq_simulation::{Freshness, Verdict};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

/// What the Operator allowed for this project, kept in its app data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionChoice {
    /// Trusted-local execution: builds, tests and the harness may run.
    #[serde(default)]
    pub trusted: bool,
    /// Cargo may use the network (to fetch dependencies).
    #[serde(default)]
    pub network: bool,
}

/// Implementation checks in progress or done.
#[derive(Default)]
pub struct ImplementationState {
    pub choice: Option<ExecutionChoice>,
    pub report: Option<CheckReport>,
    pub freshness: Option<Freshness>,
    /// Whether the code is still what the checks read: worked out when they
    /// load or finish, since it reads the repository.
    code: Option<Freshness>,
    /// The Assistant waits for the checks in progress.
    pub(crate) assistant: Option<std::sync::mpsc::Sender<agq_assistant::ToolResult>>,
    /// The implementation task in progress (one at a time).
    pub task: Option<crate::tasks::ActiveTask>,
    /// The Assistant waits for the Operator to start or decline a task.
    pub(crate) proposal: Option<std::sync::mpsc::Sender<agq_assistant::ToolResult>>,
    active: Option<(Receiver<CheckReport>, Arc<AtomicBool>, Instant)>,
}

impl ImplementationState {
    pub fn checking(&self) -> bool {
        self.active.is_some()
    }
}

impl Studio {
    fn execution_file(&self) -> Option<PathBuf> {
        let project = self.project.as_ref()?;
        Some(
            crate::conversation::project_data(&self.session_path, project.folder())
                .join("execution.json"),
        )
    }

    fn project_checks_file(&self) -> Option<PathBuf> {
        let project = self.project.as_ref()?;
        Some(
            crate::conversation::project_data(&self.session_path, project.folder())
                .join("checks.json"),
        )
    }

    /// The project's required check commands (`checks.json` in its app
    /// data, ROADMAP §4.9): the Operator's, never a worker's. None when the
    /// file is missing or cannot be read (a later format is never guessed).
    pub fn project_checks(&self) -> agq_implementation::task::ProjectChecks {
        self.project_checks_file()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| {
                serde_json::from_str::<agq_implementation::task::ProjectChecks>(&text).ok()
            })
            .filter(|checks| checks.format == agq_implementation::task::ProjectChecks::FORMAT)
            .unwrap_or_default()
    }

    /// Saves the project's required check commands, atomically.
    pub fn save_project_checks(
        &mut self,
        checks: &agq_implementation::task::ProjectChecks,
    ) -> Result<(), String> {
        let path = self.project_checks_file().ok_or("No project is open.")?;
        let text = serde_json::to_string_pretty(checks).map_err(|e| e.to_string())?;
        crate::session::write_presentation(&path, text.as_bytes()).map_err(|e| e.to_string())
    }

    /// What the Operator allowed for this project.
    pub fn execution_choice(&mut self) -> ExecutionChoice {
        if let Some(choice) = self.implementation.choice {
            return choice;
        }
        let choice = self.execution_file_choice();
        self.implementation.choice = Some(choice);
        choice
    }

    /// What the Operator allowed, as saved (nothing when never chosen).
    pub fn execution_file_choice(&self) -> ExecutionChoice {
        self.execution_file()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Turns trusted-local execution on or off for this project.
    pub fn set_execution_choice(&mut self, choice: ExecutionChoice) {
        self.implementation.choice = Some(choice);
        if let Some(path) = self.execution_file() {
            let _ = std::fs::create_dir_all(path.parent().unwrap_or(Path::new(".")));
            let text = serde_json::to_string_pretty(&choice).unwrap_or_default();
            if let Err(error) = std::fs::write(&path, text) {
                self.status = format!("The choice could not be saved: {error}");
            }
        }
        self.status = if choice.trusted {
            "Trusted-local execution is on for this project.".into()
        } else {
            "Trusted-local execution is off for this project: no code runs.".into()
        };
        self.mark(Dirty::STATUS | Dirty::LAYOUT);
    }

    /// A part as C1 asks about it (ROADMAP §2.8): from the model, its links
    /// and its locks; the Inspector and the Assistant read the same answer.
    pub fn responsibility(
        &self,
        element: ElementId,
    ) -> Option<agq_implementation::responsibility::Responsibility> {
        let project = self.project.as_ref()?;
        let state = project.state();
        let links = self.implementation_links().unwrap_or_default();
        let locks: Vec<ElementId> = state.locks().iter().copied().collect();
        agq_implementation::responsibility::responsibility(state.tree(), &links, &locks, element)
    }

    /// [`Studio::responsibility`] in words, for the Assistant.
    pub fn explain(&self, element: ElementId) -> Option<String> {
        let project = self.project.as_ref()?;
        self.responsibility(element)
            .map(|r| r.describe(project.state().tree()))
    }

    /// The implementation links of the project (none when it has no file).
    pub fn implementation_links(&self) -> Result<Links, String> {
        let project = self.project.as_ref().ok_or("No project is open.")?;
        match project.links() {
            Some(text) => Links::parse(text),
            None => Ok(Links::default()),
        }
    }

    /// Saves the links with the model (`model/links.json`).
    pub fn save_implementation_links(&mut self, links: &Links) -> Result<(), String> {
        let project = self.project.as_mut().ok_or("No project is open.")?;
        let mut links = links.clone();
        links.refresh_names(project.state().tree());
        project
            .save_links(Some(links.to_text()))
            .map_err(|e| e.to_string())?;
        self.mark(Dirty::MODEL | Dirty::STATUS);
        Ok(())
    }

    /// Links code to a model element (`model/links.json`).
    pub fn link_code(
        &mut self,
        element: ElementId,
        kind: agq_implementation::LinkKind,
        path: &str,
        symbol: Option<&str>,
    ) {
        let result = self.implementation_links().and_then(|mut links| {
            let tree = self
                .project
                .as_ref()
                .ok_or("No project is open.")?
                .state()
                .tree();
            links.add(
                tree,
                element,
                kind,
                path.trim(),
                symbol.map(str::trim).filter(|s| !s.is_empty()),
            );
            self.save_implementation_links(&links)
        });
        self.status = match result {
            Ok(()) => format!("Linked {} {} to the model.", kind.label(), path.trim()),
            Err(error) => format!("The link could not be saved: {error}"),
        };
        self.refresh_check_freshness();
        self.mark(Dirty::STATUS | Dirty::MODEL | Dirty::LAYOUT);
    }

    /// Removes a link.
    pub fn unlink_code(&mut self, element: ElementId, path: &str, symbol: Option<&str>) {
        let result = self.implementation_links().and_then(|mut links| {
            links.remove(element, path, symbol);
            self.save_implementation_links(&links)
        });
        self.status = match result {
            Ok(()) => format!("Removed the link to {path}."),
            Err(error) => format!("The link could not be removed: {error}"),
        };
        self.refresh_check_freshness();
        self.mark(Dirty::STATUS | Dirty::MODEL | Dirty::LAYOUT);
    }

    /// The implementation repository: the project folder, or where
    /// `links.json` says.
    pub fn implementation_repository(&self) -> Option<PathBuf> {
        let project = self.project.as_ref()?;
        let links = self.implementation_links().ok()?;
        let folder = project.folder().join(&links.repository);
        folder.canonicalize().ok().or(Some(folder))
    }

    /// The executor for `repository`: nothing runs unless trusted-local
    /// execution is on; each working copy builds in its own target folder.
    pub fn implementation_executor(
        &self,
        repository: &Path,
        _writable: bool,
    ) -> Result<Executor, String> {
        let choice = self
            .implementation
            .choice
            .unwrap_or_else(|| self.execution_file_choice());
        let project = self.project.as_ref().ok_or("No project is open.")?;
        // The project's own copy of its code builds in `targets/main` (task
        // worktrees in `targets/<job>`): short, since Windows' linker still
        // fails on paths longer than 260 characters.
        let target = crate::conversation::project_data(&self.session_path, project.folder())
            .join("targets")
            .join("main");
        let scope = Scope::read_only(repository).map_err(|e| e.to_string())?;
        Ok(Executor::new(scope)
            .trusted(choice.trusted)
            .network(choice.network)
            .target_dir(target))
    }

    /// The isolation this host offers, in words.
    pub fn isolation(&self) -> &'static str {
        Isolation::available().describe()
    }

    /// Runs the implementation checks on a background thread.
    pub fn start_checks(&mut self) {
        if self.implementation.active.is_some() {
            return;
        }
        let (Some(project), Some(repository)) =
            (self.project.as_ref(), self.implementation_repository())
        else {
            self.status = "Open a project with linked code to check it.".into();
            return;
        };
        let links = match self.implementation_links() {
            Ok(links) => links,
            Err(error) => {
                self.status = error;
                return;
            }
        };
        if links.links.is_empty() {
            self.status =
                "No code is linked yet: link modules, types and tests to elements first.".into();
            return;
        }
        let executor = match self.implementation_executor(&repository, false) {
            Ok(executor) => executor,
            Err(error) => {
                self.status = error;
                return;
            }
        };
        let tree = project.state().tree().clone();
        let (sender, receiver) = std::sync::mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let spawned = std::thread::Builder::new()
            .name("agentique-checks".into())
            .spawn(move || {
                let read = |path: &str| std::fs::read_to_string(repository.join(path)).ok();
                let mut checks = vec![module_boundaries(&tree, &links, &read)];
                checks.extend(contract_shapes(&tree, &links, &read));
                if !flag.load(Ordering::SeqCst) {
                    checks.extend(linked_tests(
                        &links,
                        &executor.cancel_flag(flag),
                        "",
                        Duration::from_secs(900),
                    ));
                }
                let _ = sender.send(CheckReport::new(&tree, &links, &repository, checks));
            });
        if let Err(error) = spawned {
            self.status = format!("The checks could not start: {error}");
            return;
        }
        self.implementation.active = Some((receiver, cancel, Instant::now()));
        self.status = "Checking the implementation…".into();
        self.mark(Dirty::STATUS | Dirty::LAYOUT);
    }

    pub fn stop_checks(&mut self) {
        if let Some((_, cancel, _)) = &self.implementation.active {
            cancel.store(true, Ordering::SeqCst);
        }
    }

    /// Takes a finished round of checks. Returns whether anything changed.
    pub fn poll_checks(&mut self) -> bool {
        let Some((receiver, ..)) = &self.implementation.active else {
            return false;
        };
        let report = match receiver.try_recv() {
            Ok(report) => report,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => {
                self.implementation.active = None;
                self.status = "The checks stopped without a result.".into();
                if let Some(reply) = self.implementation.assistant.take() {
                    let _ = reply.send(agq_assistant::ToolResult::error(self.status.clone()));
                }
                return true;
            }
        };
        self.implementation.active = None;
        let count = |verdict: Verdict| {
            report
                .checks
                .iter()
                .filter(|c| c.verdict == verdict)
                .count()
        };
        let (passed, failed) = (count(Verdict::Passed), count(Verdict::Failed));
        // The one summary (ROADMAP §5.6 item 8): a check that did not run is
        // never counted as passing.
        self.status = match agq_simulation::summary(report.checks.iter().map(|c| c.verdict)) {
            Verdict::Passed => format!(
                "{} implementation check(s): every one passed",
                report.checks.len()
            ),
            Verdict::Failed => format!(
                "{failed} of {} implementation check(s) failed: drift is shown at the elements",
                report.checks.len()
            ),
            verdict => format!(
                "{passed} of {} implementation check(s) passed; none failed, but not every one could be decided ({})",
                report.checks.len(),
                verdict.label()
            ),
        };
        if let Some(store) = self.check_store()
            && let Err(error) = store.save(&report)
        {
            self.status = format!("The checks could not be saved: {error}");
        }
        if let Some(reply) = self.implementation.assistant.take() {
            let _ = reply.send(agq_assistant::ToolResult::answer(describe_report(&report)));
        }
        self.implementation.report = Some(report);
        self.refresh_check_freshness();
        self.mark(Dirty::MODEL | Dirty::STATUS | Dirty::LAYOUT);
        true
    }

    fn check_store(&self) -> Option<CheckStore> {
        let project = self.project.as_ref()?;
        Some(CheckStore(
            crate::conversation::project_data(&self.session_path, project.folder()).join("checks"),
        ))
    }

    /// Loads the newest round of checks when a project opens, and finds
    /// tasks the app did not see end.
    pub fn load_checks(&mut self) {
        self.implementation = ImplementationState::default();
        self.implementation.report = self.check_store().and_then(|s| s.latest());
        self.refresh_check_freshness();
        self.recover_tasks();
    }

    /// Works out whether the shown checks still describe the model and the
    /// code (this reads the repository).
    pub fn refresh_check_freshness(&mut self) {
        let fresh = match (
            &self.implementation.report,
            &self.project,
            self.implementation_links(),
            self.implementation_repository(),
        ) {
            (Some(report), Some(project), Ok(links), Some(repository)) => {
                Some(report.freshness(project.state().tree(), &links, &repository))
            }
            _ => None,
        };
        self.implementation.code = fresh.clone().filter(|f| match f {
            Freshness::Outdated(why) => why.contains("code"),
            Freshness::Current => true,
        });
        self.implementation.freshness = fresh;
    }

    /// The same after a model change: only the model side is read again.
    pub fn refresh_check_freshness_for_model(&mut self) {
        let (Some(report), Some(project), Ok(links)) = (
            &self.implementation.report,
            &self.project,
            self.implementation_links(),
        ) else {
            return;
        };
        self.implementation.freshness = Some(
            if agq_implementation::links_digest(project.state().tree(), &links)
                != report.model_digest
            {
                Freshness::Outdated(
                    "the linked model elements changed since these checks ran".into(),
                )
            } else {
                self.implementation
                    .code
                    .clone()
                    .unwrap_or(Freshness::Current)
            },
        );
    }

    /// Failing current checks by element: drift. Outdated checks show none.
    pub fn drift(&self) -> BTreeMap<ElementId, Vec<ImplementationCheck>> {
        let current = self
            .implementation
            .freshness
            .as_ref()
            .is_some_and(Freshness::is_current);
        let Some(report) = self.implementation.report.as_ref().filter(|_| current) else {
            return BTreeMap::new();
        };
        agq_implementation::drift(&report.checks)
            .into_iter()
            .map(|(element, checks)| (element, checks.into_iter().cloned().collect()))
            .collect()
    }

    /// Opens a linked file in an editor: VS Code when it is installed, else
    /// shows it in Explorer. Never the file's own default program, which for
    /// a script would run it.
    pub fn open_in_editor(&mut self, relative: &str, line: Option<u32>) {
        let Some(repository) = self.implementation_repository() else {
            return;
        };
        let scope = match Scope::read_only(&repository) {
            Ok(scope) => scope,
            Err(error) => {
                self.status = error.to_string();
                return;
            }
        };
        let path = match scope.resolve(relative) {
            Ok(path) => path,
            Err(error) => {
                self.status = error.to_string();
                return;
            }
        };
        let target = match line {
            Some(line) => format!("{}:{line}", path.display()),
            None => path.display().to_string(),
        };
        let editor = std::process::Command::new(if cfg!(windows) { "code.cmd" } else { "code" })
            .args(["--goto", &target])
            .spawn();
        if editor.is_err() {
            let shown = if cfg!(windows) {
                std::process::Command::new("explorer")
                    .arg(format!("/select,{}", path.display()))
                    .spawn()
                    .map(|_| ())
            } else {
                Err(std::io::Error::other("no editor"))
            };
            self.status = match shown {
                Ok(()) => format!("No editor was found; {relative} is shown in Explorer."),
                Err(_) => format!("No editor was found to open {relative}."),
            };
        } else {
            self.status = format!("Opened {relative} in the editor.");
        }
        self.mark(Dirty::STATUS);
    }
}

/// A round of checks in plain words, for the Assistant.
pub fn describe_report(report: &CheckReport) -> String {
    let mut lines = vec![format!(
        "Implementation checks on {} at {}{}:",
        report.repository,
        &report.commit[..report.commit.len().min(10)],
        if report.dirty {
            " (with uncommitted changes)"
        } else {
            ""
        }
    )];
    for check in &report.checks {
        lines.push(format!(
            "- {} ({}): {}. {}",
            check.name,
            check.kind.label(),
            check.verdict.label(),
            check.message
        ));
        for detail in check.details.iter().take(6) {
            lines.push(format!("    {detail}"));
        }
    }
    lines.join("\n")
}

impl Studio {
    /// The code links, for one element or all, with the newest checks.
    pub fn describe_links(&self, element: Option<ElementId>) -> Result<String, String> {
        let links = self.implementation_links()?;
        let tree = self
            .project
            .as_ref()
            .ok_or("No project is open.")?
            .state()
            .tree();
        let shown: Vec<&agq_implementation::Link> = match element {
            Some(element) => links.for_element(element),
            None => links.links.iter().collect(),
        };
        let mut lines = vec![format!(
            "Code folder: {} ({}); trusted-local execution is {}.",
            links.repository,
            links.language,
            if self.implementation.choice.is_some_and(|c| c.trusted) {
                "on"
            } else {
                "off"
            }
        )];
        if shown.is_empty() {
            lines.push("No code is linked here.".into());
        }
        for link in shown {
            let name = if tree.contains(link.element()) {
                tree.qualified_name(link.element())
            } else {
                format!("{} (no longer in the model)", link.name)
            };
            lines.push(format!(
                "- {name}: {} {}{}",
                link.kind.label(),
                link.path,
                link.symbol
                    .as_ref()
                    .map(|s| format!("#{s}"))
                    .unwrap_or_default()
            ));
        }
        if let Some(report) = &self.implementation.report {
            match &self.implementation.freshness {
                Some(Freshness::Outdated(why)) => {
                    lines.push(format!("The newest checks are outdated: {why}."))
                }
                _ => lines.push("The newest checks:".into()),
            }
            lines.push(describe_report(report));
        }
        Ok(lines.join("\n"))
    }
}

/// Rounds of checks in the app's per-project data.
struct CheckStore(PathBuf);

impl CheckStore {
    fn save(&self, report: &CheckReport) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.0)?;
        let text = serde_json::to_string(report).map_err(std::io::Error::other)?;
        let temporary = self.0.join(format!("{}.json.tmp", report.id));
        std::fs::write(&temporary, text)?;
        std::fs::rename(temporary, self.0.join(format!("{}.json", report.id)))
    }

    fn latest(&self) -> Option<CheckReport> {
        let mut reports: Vec<CheckReport> = std::fs::read_dir(&self.0)
            .ok()?
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            .filter_map(|p| std::fs::read_to_string(p).ok())
            .filter_map(|t| serde_json::from_str::<CheckReport>(&t).ok())
            .collect();
        reports.sort_by(|a, b| a.started.cmp(&b.started).then(a.id.cmp(&b.id)));
        reports.pop()
    }
}
