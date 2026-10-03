//! Implementation tasks in the Studio (ROADMAP §4.15, W8.4, W10.1): the
//! Operator (or the Assistant, after the Operator agrees) asks a worker to
//! implement an element. Each task is a job in the project's app data, with
//! its own git worktree and the checks it must pass, fixed when it starts.
//! The worker runs on its own thread; when it ends, the Studio commits the
//! worktree (the task commit), verifies that commit itself, and the
//! Operator reviews it, then integrates exactly that commit (only while the
//! verification is current and passed, or after saying so explicitly) or
//! discards it. A task the app did not see end is found on the next open
//! as interrupted, with what it wrote still there to review.
use crate::studio::{Dirty, Studio};
use agq_assistant::worker::{self, Worker};
use agq_execution::jobs::{Job, JobState, JobStore};
use agq_execution::{Executor, Scope, git};
use agq_implementation::Link;
use agq_implementation::task::{
    ALWAYS_PROTECTED, Brief, Checked, RequiredCheck, Verification, brief, brief_digest,
    not_configured, required_checks, verify,
};
use agq_language::ElementId;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Instant;

/// What a finished task left for the Operator (kept in its job).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskOutcome {
    pub element: u64,
    pub repository: String,
    pub worktree: String,
    pub branch: String,
    pub base: String,
    /// The task commit: what was verified, reviewed and is integrated.
    #[serde(default)]
    pub commit: String,
    /// The worker's own words.
    pub summary: Option<String>,
    pub proposed: Vec<Link>,
    pub contract_requests: Vec<String>,
    pub rounds: usize,
    /// What the Studio found when it checked the worktree itself.
    pub verification: Verification,
    /// Changed files: (path, status, added, removed).
    pub files: Vec<(String, String, usize, usize)>,
    /// Why it ended early, if it did.
    pub notice: Option<String>,
    /// The worker's model and what its calls cost, estimated from the
    /// provider's usage (None when the price is not known).
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub cost_usd: Option<f64>,
}

/// What a task changed in the model, for its review.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModelChanges {
    pub created: Vec<String>,
    pub updated: Vec<String>,
    pub deleted: Vec<String>,
    /// Problems in the task's model, and in its base.
    pub problems: usize,
    pub problems_before: usize,
    /// Why it may not be integrated, if it may not.
    pub refused: Option<String>,
}

/// A task in progress.
pub struct ActiveTask {
    pub job: String,
    pub element: ElementId,
    pub title: String,
    pub started: Instant,
    /// The worker's latest steps, newest last.
    pub progress: Vec<String>,
    /// What the worker is doing now (its latest tool's phase).
    pub phase: &'static str,
    events: Receiver<TaskEvent>,
    cancel: Arc<AtomicBool>,
}

enum TaskEvent {
    Progress(String),
    Phase(&'static str),
    Done(Box<TaskOutcome>),
}

impl Studio {
    /// The project's jobs (app data, never committed).
    pub fn job_store(&self) -> Option<JobStore> {
        let project = self.project.as_ref()?;
        Some(JobStore::new(
            crate::conversation::project_data(&self.session_path, project.folder()).join("jobs"),
        ))
    }

    /// Tasks for `element`, newest first.
    pub fn tasks_for(&self, element: ElementId) -> Vec<Job> {
        let mut jobs: Vec<Job> = self
            .job_store()
            .map(|s| s.list())
            .unwrap_or_default()
            .into_iter()
            .filter(|j| {
                j.kind == "implement" && j.detail["element"].as_u64() == Some(element.raw())
            })
            .collect();
        jobs.reverse();
        jobs
    }

    /// Why a task cannot start now, if it cannot.
    pub fn task_blocker(&self) -> Option<String> {
        if self.project.is_none() {
            return Some("Open a project first.".into());
        }
        if self.implementation.task.is_some() {
            return Some("A task is in progress; one at a time.".into());
        }
        if !self.execution_file_choice().trusted
            && !self.implementation.choice.is_some_and(|c| c.trusted)
        {
            return Some("The worker builds and tests code, which needs trusted-local execution; the Run panel turns it on.".into());
        }
        let repository = self.implementation_repository()?;
        if let Err(refusal) = git::head(&repository) {
            return Some(format!(
                "The code folder {} is not a git repository with a commit: {refusal}",
                repository.display()
            ));
        }
        // The worktree starts from the last commit: with the model in the
        // repository, the model (its locks included) must be committed, or
        // the worker would work on an older one.
        if self.model_folder_in(&repository).is_some()
            && self
                .project
                .as_ref()
                .is_some_and(|p| p.has_uncommitted_changes().unwrap_or(true))
        {
            return Some("The model has changes since the last checkpoint. Make a checkpoint first (Ctrl+S): the task starts from the last commit, and your changes, locks included, would not be in it.".into());
        }
        // The worker uses the Assistant's model.
        if let Some(missing) = &self.conversation.key_missing {
            return Some(missing.clone());
        }
        None
    }

    /// The checks a task on `element` would have to pass, and what would not
    /// be checked because it is not configured, for the Operator to see
    /// before starting it.
    pub fn task_checks(&self, element: ElementId) -> (Vec<String>, Vec<String>) {
        let Some(project) = self.project.as_ref() else {
            return (Vec::new(), Vec::new());
        };
        let Ok(links) = self.implementation_links() else {
            return (Vec::new(), Vec::new());
        };
        match brief(project.state().tree(), &links, element, "") {
            Ok(brief) => (
                required_checks(&links, &brief, &self.project_checks().commands)
                    .iter()
                    .map(RequiredCheck::label)
                    .collect(),
                not_configured(&links, &brief),
            ),
            Err(why) => (Vec::new(), vec![why]),
        }
    }

    /// Starts a task: a job, a worktree, and the worker on its thread.
    pub fn start_implementation(
        &mut self,
        element: ElementId,
        instructions: &str,
    ) -> Result<String, String> {
        if let Some(why) = self.task_blocker() {
            return Err(why);
        }
        let project = self.project.as_ref().ok_or("No project is open.")?;
        let tree = project.state().tree().clone();
        let links = self.implementation_links()?;
        let mut brief = brief(&tree, &links, element, instructions)?;
        let repository = self
            .implementation_repository()
            .ok_or("The project has no code folder.")?;
        // The model is the Operator's: when the code shares the project's
        // folder, its model folder is never written by a task.
        if let Some(model) = self.model_folder_in(&repository) {
            brief.protected.push(model);
        }
        // The checks it must pass, fixed now (§4.15): the worker can add
        // checks, never remove these.
        brief.required = required_checks(&links, &brief, &self.project_checks().commands);
        brief.text.push_str("\n\n");
        brief
            .text
            .push_str(&agq_implementation::task::checks_section(&brief.required));
        let store = self.job_store().ok_or("No project is open.")?;
        let mut job = store
            .create(
                "implement",
                &brief.title,
                serde_json::json!({
                    "element": element.raw(),
                    "instructions": instructions,
                    "required": brief.required,
                    "notConfigured": not_configured(&links, &brief),
                }),
            )
            .map_err(|e| e.to_string())?;
        let data = crate::conversation::project_data(&self.session_path, project.folder());
        let path = data.join("worktrees").join(&job.id);
        let seq = store
            .begin(&mut job, "worktree", "Create the worktree", true)
            .map_err(|e| e.to_string())?;
        let worktree = match git::create_worktree(&repository, &job.id, &path) {
            Ok(worktree) => worktree,
            Err(refusal) => {
                let _ = store.end(&mut job, seq, false, Some(refusal.to_string()));
                let _ = store.set_state(&mut job, JobState::Failed, Some(refusal.to_string()));
                return Err(format!("The worktree could not be made: {refusal}"));
            }
        };
        let _ = store.end(
            &mut job,
            seq,
            true,
            Some(worktree.path.display().to_string()),
        );
        job.detail["worktree"] = serde_json::json!(worktree.path.display().to_string());
        job.detail["branch"] = serde_json::json!(worktree.branch.clone());
        job.detail["base"] = serde_json::json!(worktree.base.clone());
        job.detail["repository"] = serde_json::json!(repository.display().to_string());
        job.detail["brief"] = serde_json::to_value(&brief).unwrap_or_default();
        let _ = store.set_state(&mut job, JobState::Running, None);
        self.run_worker_in(job, element, worktree, repository, tree, links, brief, &[])
    }

    /// Continues a task that was interrupted, stopped or reviewed, in the
    /// worktree it already has: the worktree is not made again (the job's
    /// journal says it was), and what the worker wrote is where it left it.
    pub fn resume_task(&mut self, job: &str) -> Result<String, String> {
        if self.implementation.task.is_some() {
            return Err("A task is in progress; one at a time.".into());
        }
        let store = self.job_store().ok_or("No project is open.")?;
        let mut found = store.load(job).ok_or("There is no such task.")?;
        if !matches!(
            found.state,
            JobState::Interrupted | JobState::WaitingForYou | JobState::Failed
        ) {
            return Err(format!(
                "A {} task cannot be continued.",
                found.state.label()
            ));
        }
        if found.completed("worktree").is_none() {
            return Err("The task has no worktree to continue in; start a new one.".into());
        }
        let path = PathBuf::from(found.detail["worktree"].as_str().unwrap_or_default());
        if !path.is_dir() {
            return Err(format!(
                "The worktree {} is gone; start a new task.",
                path.display()
            ));
        }
        if let Some(why) = self.task_blocker() {
            return Err(why);
        }
        let element = ElementId::from_raw(found.detail["element"].as_u64().unwrap_or_default());
        let project = self.project.as_ref().ok_or("No project is open.")?;
        let tree = project.state().tree().clone();
        if !tree.contains(element) {
            return Err("The element is no longer in the model.".into());
        }
        let links = self.implementation_links()?;
        let instructions = format!(
            "{}\n\nThis continues an earlier attempt: the repository already has its work. Read it before you change anything, then finish what is left.",
            found.detail["instructions"].as_str().unwrap_or_default()
        );
        let mut brief = brief(&tree, &links, element, &instructions)?;
        // The checks fixed when it was approved, not worked out again.
        brief.required =
            match serde_json::from_value::<Vec<RequiredCheck>>(found.detail["required"].clone()) {
                Ok(required) if !required.is_empty() => required,
                _ => required_checks(&links, &brief, &self.project_checks().commands),
            };
        brief.text.push_str("\n\n");
        brief
            .text
            .push_str(&agq_implementation::task::checks_section(&brief.required));
        let repository = PathBuf::from(found.detail["repository"].as_str().unwrap_or_default());
        if let Some(model) = self.model_folder_in(&repository) {
            brief.protected.push(model);
        }
        let carried: Vec<Link> =
            serde_json::from_value::<TaskOutcome>(found.detail["outcome"].clone())
                .map(|o| o.proposed)
                .unwrap_or_default();
        let worktree = git::Worktree {
            path,
            branch: found.detail["branch"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
            base: found.detail["base"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        };
        let seq = store
            .begin(
                &mut found,
                "resume",
                "Continue the worker in the same worktree",
                false,
            )
            .map_err(|e| e.to_string())?;
        let _ = store.end(&mut found, seq, true, None);
        let _ = store.set_state(&mut found, JobState::Running, Some("continued".into()));
        self.run_worker_in(
            found, element, worktree, repository, tree, links, brief, &carried,
        )
    }

    /// Runs the worker for `job` in `worktree`, and the Studio's own
    /// verification after it.
    #[allow(clippy::too_many_arguments)]
    fn run_worker_in(
        &mut self,
        job: Job,
        element: ElementId,
        worktree: git::Worktree,
        repository: PathBuf,
        tree: agq_language::Tree,
        links: agq_implementation::Links,
        brief: agq_implementation::task::Brief,
        carried: &[Link],
    ) -> Result<String, String> {
        let project = self.project.as_ref().ok_or("No project is open.")?;
        let data = crate::conversation::project_data(&self.session_path, project.folder());
        let choice = self.execution_choice();
        let target = data.join("targets").join(&job.id);
        // Stop ends whatever the worker or the verification is running.
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let make_executor_at = {
            let protected = brief.protected.clone();
            let allowed = project_programs(&brief.required);
            let cancel = cancel.clone();
            move |root: &std::path::Path| -> Result<Executor, String> {
                let protected: Vec<&str> = protected.iter().map(String::as_str).collect();
                let scope = Scope::writable(root, &[""], &protected).map_err(|e| e.to_string())?;
                Ok(Executor::new(scope)
                    .trusted(choice.trusted)
                    .network(choice.network)
                    .target_dir(target.clone())
                    .allow(allowed.clone())
                    .cancel_flag(cancel.clone()))
            }
        };
        let make_executor = {
            let root = worktree.path.clone();
            let at = make_executor_at.clone();
            move || at(&root)
        };
        let verify_name = format!("{}-verify", job.id);
        let executor = make_executor()?;
        let carried = carried.to_vec();
        // The worker's own copy of the model: the project folder in its
        // worktree, when the project's model is in the repository.
        let working_model = self.model_folder_in(&repository).and_then(|model| {
            worktree
                .path
                .join(model)
                .parent()
                .map(std::path::Path::to_path_buf)
        });
        // The worker works through its own code tools in the task's worktree,
        // never in the Conversation's development session.
        {
            let mut inputs = self.conversation.inputs.borrow_mut();
            inputs.development = None;
            // Its own pause gate and messages, never the Conversation's.
            inputs.steering = None;
        }
        let mut runtime = (self.conversation.new_runtime)();
        let (sender, events) = std::sync::mpsc::channel();
        let title = brief.title.clone();
        let commit_message = format!("{title} (Agentique task {})", job.id);
        let outcome_base = TaskOutcome {
            element: brief.element,
            repository: repository.display().to_string(),
            worktree: worktree.path.display().to_string(),
            branch: worktree.branch.clone(),
            base: worktree.base.clone(),
            ..Default::default()
        };
        std::thread::Builder::new()
            .name("agentique-task".into())
            .spawn(move || {
                let mut worker =
                    Worker::new(tree.clone(), links, brief.clone(), executor, flag.clone());
                if let Some(folder) = working_model {
                    worker = worker.with_model(folder);
                }
                // Links an earlier attempt proposed stay proposed.
                worker.proposed = carried;
                let progress = sender.clone();
                let author = runtime.model();
                let mut usage = agq_assistant::Usage::default();
                // Each call by its tool and what it was about, as it ends.
                let mut calls: std::collections::HashMap<String, (String, String)> =
                    std::collections::HashMap::new();
                let mut on_event = |event| match event {
                    agq_assistant::TurnEvent::Stream(
                        agq_assistant::StreamEvent::ToolCallStarted { id, name },
                    ) => {
                        let _ = progress.send(TaskEvent::Phase(agq_assistant::phase(&name)));
                        calls.insert(id, (name, String::new()));
                    }
                    agq_assistant::TurnEvent::Stream(agq_assistant::StreamEvent::ToolInput {
                        id,
                        json,
                    }) => {
                        if let Some(call) = calls.get_mut(&id) {
                            call.1.push_str(&json);
                        }
                    }
                    agq_assistant::TurnEvent::Stream(agq_assistant::StreamEvent::ToolCallId {
                        stream_id,
                        id,
                    }) => {
                        if let Some(call) = calls.remove(&stream_id) {
                            calls.insert(id, call);
                        }
                    }
                    agq_assistant::TurnEvent::Stream(agq_assistant::StreamEvent::Usage(u)) => {
                        usage.add(u)
                    }
                    agq_assistant::TurnEvent::ToolFinished(result) => {
                        let (name, input) = calls.remove(&result.tool_use_id).unwrap_or_default();
                        let path = serde_json::from_str::<serde_json::Value>(&input)
                            .ok()
                            .and_then(|v| {
                                v["path"]
                                    .as_str()
                                    .or(v["element"].as_str())
                                    .map(str::to_string)
                            })
                            .unwrap_or_default();
                        let lines: Vec<&str> = result.content.lines().collect();
                        let what = match name.as_str() {
                            "read_code" | "list_files" => path,
                            "run_checks" => format!(
                                "{} {}",
                                lines.first().unwrap_or(&""),
                                lines.last().unwrap_or(&"")
                            ),
                            _ => lines.first().unwrap_or(&"").to_string(),
                        };
                        let mark = if result.is_error { "✗" } else { "✓" };
                        let _ = progress.send(TaskEvent::Progress(format!("{mark} {name} {what}")));
                    }
                    _ => {}
                };
                let conversation =
                    worker::run_worker(runtime.as_mut(), &mut worker, &mut on_event, &flag);
                // The model the task is checked against: its own copy, if
                // the worker used it.
                let task_tree = worker.close_model();
                let cost_usd = author.as_ref().and_then(|m| usage.cost_usd(m));
                let model_name = author.map(|m| format!("{}/{}", m.provider.id(), m.model));
                let notice = conversation.entries.iter().rev().find_map(|e| match e {
                    agq_assistant::Entry::Notice { text } => Some(text.clone()),
                    _ => None,
                });
                // The task commit: verification, review and integration all
                // refer to it, so what is integrated is what was checked.
                // Cargo first writes its lock file (a build would write it
                // after the commit and make the verification outdated at once).
                let worktree_path = std::path::Path::new(&outcome_base.worktree);
                if let Ok(executor) = make_executor() {
                    let _ = executor.run(
                        &agq_execution::Program::cargo(&[
                            "metadata",
                            "--offline",
                            "--format-version",
                            "1",
                        ]),
                        "",
                        std::time::Duration::from_secs(300),
                    );
                }
                let commit = match git::commit_worktree(worktree_path, &commit_message) {
                    Ok(commit) => commit,
                    Err(refusal) => {
                        let _ = sender.send(TaskEvent::Progress(format!(
                            "✗ The worktree could not be committed: {refusal}"
                        )));
                        String::new()
                    }
                };
                let _ = sender.send(TaskEvent::Progress(
                    "The Studio checks the task commit itself…".into(),
                ));
                // Never the worker's word: the Studio's own verification, of
                // exactly the task commit: a clean checkout of it, so files the
                // commit does not hold (ignored ones the worker wrote) count
                // for nothing.
                let repository_path = std::path::Path::new(&outcome_base.repository);
                let clean = std::path::PathBuf::from(format!("{}-verify", outcome_base.worktree));
                let _ = git::remove_worktree(repository_path, &verify_name);
                let checked_out = !commit.is_empty()
                    && git::checkout_worktree(repository_path, &verify_name, &clean, &commit)
                        .is_ok();
                let verifier = if checked_out {
                    make_executor_at(&clean)
                } else {
                    Err("The task commit could not be checked out to be verified.".to_string())
                };
                let mut verification = match verifier {
                    Ok(executor) => verify(
                        task_tree.as_ref().unwrap_or(&tree),
                        &worker.links(),
                        &brief,
                        &executor,
                        flag.clone(),
                    ),
                    Err(error) => Verification {
                        build_errors: error,
                        required: brief.required.clone(),
                        ..Default::default()
                    },
                };
                if checked_out {
                    let _ = git::remove_worktree(repository_path, &verify_name);
                }
                verification.checked = Some(Checked {
                    commit: commit.clone(),
                    model_digest: brief_digest(&tree, &brief),
                });
                let files = if commit.is_empty() {
                    git::patch(worktree_path, &outcome_base.base)
                } else {
                    git::patch_of(worktree_path, &outcome_base.base, &commit)
                }
                .map(|p| {
                    p.files
                        .into_iter()
                        .map(|f| (f.path, f.status, f.added, f.removed))
                        .collect()
                })
                .unwrap_or_default();
                let outcome = TaskOutcome {
                    summary: worker.summary.clone(),
                    proposed: worker.proposed.clone(),
                    contract_requests: worker.contract_requests.clone(),
                    rounds: worker.rounds,
                    verification,
                    files,
                    notice: if flag.load(Ordering::SeqCst) {
                        Some("Stopped by the Operator.".into())
                    } else {
                        notice
                    },
                    model: model_name,
                    cost_usd,
                    commit,
                    ..outcome_base
                };
                let _ = sender.send(TaskEvent::Done(Box::new(outcome)));
            })
            .map_err(|e| e.to_string())?;
        self.implementation.task = Some(ActiveTask {
            job: job.id.clone(),
            element,
            title: title.clone(),
            started: Instant::now(),
            progress: Vec::new(),
            phase: "Starting",
            events,
            cancel,
        });
        self.status = format!("{title}: the worker is working in its own worktree.");
        self.mark(Dirty::STATUS | Dirty::LAYOUT);
        Ok(job.id)
    }

    /// Tells the Assistant whether the Operator started the task it
    /// proposed.
    pub fn answer_proposal(&mut self, started: Result<String, String>) {
        if let Some(reply) = self.implementation.proposal.take() {
            let _ = reply.send(match started {
                Ok(job) => agq_assistant::ToolResult::answer(format!(
                    "The Operator started task {job}. The worker runs on its own; the Operator reviews the patch when it ends."
                )),
                Err(why) => agq_assistant::ToolResult::error(format!("Not started: {why}")),
            });
        }
    }

    /// Stops the task in progress; what it wrote stays for review.
    pub fn stop_task(&mut self) {
        if let Some(task) = &self.implementation.task {
            task.cancel.store(true, Ordering::SeqCst);
            self.status = "Stopping the task…".into();
        }
    }

    /// Takes the task's progress and outcome. Returns whether anything changed.
    pub fn poll_task(&mut self) -> bool {
        let Some(task) = self.implementation.task.as_mut() else {
            return false;
        };
        let mut changed = false;
        let mut done = None;
        loop {
            match task.events.try_recv() {
                Ok(TaskEvent::Progress(line)) => {
                    task.progress.push(line);
                    if task.progress.len() > 40 {
                        task.progress.remove(0);
                    }
                    changed = true;
                }
                Ok(TaskEvent::Phase(phase)) => {
                    changed |= task.phase != phase;
                    task.phase = phase;
                }
                Ok(TaskEvent::Done(outcome)) => {
                    done = Some(outcome);
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    done = Some(Box::new(TaskOutcome {
                        notice: Some("The worker stopped without a result.".into()),
                        ..Default::default()
                    }));
                    break;
                }
            }
        }
        let Some(outcome) = done else {
            return changed;
        };
        let task = self.implementation.task.take().expect("checked above");
        // The worker's spend counts in the day's total, as the Assistant's.
        if let Some(cost) = outcome.cost_usd {
            self.daily_cost.add(cost);
        }
        if let Some(store) = self.job_store()
            && let Some(mut job) = store.load(&task.job)
        {
            job.detail["outcome"] = serde_json::to_value(&*outcome).unwrap_or_default();
            let state = if outcome.files.is_empty() {
                JobState::Failed
            } else {
                JobState::WaitingForYou
            };
            let note = outcome.notice.clone().or_else(|| {
                Some(format!(
                    "{} file(s) changed; verification {} ({} failing or not verified)",
                    outcome.files.len(),
                    outcome.verification.verdict().label(),
                    outcome.verification.failures()
                ))
            });
            let _ = store.set_state(&mut job, state, note);
        }
        self.status = if outcome.files.is_empty() {
            format!(
                "{}: nothing was changed. {}",
                task.title,
                outcome.notice.clone().unwrap_or_default()
            )
        } else {
            format!(
                "{} is ready for review: {} file(s); verification {}, {} failing or not verified.",
                task.title,
                outcome.files.len(),
                outcome.verification.verdict().label(),
                outcome.verification.failures()
            )
        };
        self.mark(Dirty::STATUS | Dirty::LAYOUT | Dirty::MODEL);
        true
    }

    /// What a job left, to review.
    pub fn task_outcome(&self, job: &str) -> Option<(Job, TaskOutcome)> {
        let job = self.job_store()?.load(job)?;
        let outcome = serde_json::from_value(job.detail["outcome"].clone()).ok()?;
        Some((job, outcome))
    }

    /// The patch of a task as text, at most `lines` lines: its task commit,
    /// or (for a task interrupted before it had one) its worktree.
    pub fn task_diff(outcome: &TaskOutcome, lines: usize) -> String {
        let worktree = std::path::Path::new(&outcome.worktree);
        let patch = if outcome.commit.is_empty() {
            git::patch(worktree, &outcome.base)
        } else {
            git::patch_of(worktree, &outcome.base, &outcome.commit)
        };
        let text = patch
            .map(|p| p.text())
            .unwrap_or_else(|refusal| format!("The patch cannot be read: {refusal}"));
        let total = text.lines().count();
        let mut shown: String = text.lines().take(lines).collect::<Vec<_>>().join("\n");
        if total > lines {
            shown.push_str(&format!("\n… {} more lines", total - lines));
        }
        shown
    }

    /// Opens the review of a job's patch.
    pub fn review_task(&mut self, job: &str) {
        match self.task_outcome(job) {
            Some((_, outcome)) => {
                self.dialog = Some(crate::edit::Dialog::ReviewTask {
                    job: job.to_string(),
                    problem: None,
                    diff: Self::task_diff(&outcome, 1_500),
                    confirmed: false,
                });
                self.mark(Dirty::OVERLAY);
            }
            None => {
                // An interrupted task: review what it wrote.
                if let Some(store) = self.job_store()
                    && let Some(mut found) = store.load(job)
                    && let Some(worktree) = found.detail["worktree"].as_str().map(PathBuf::from)
                {
                    let base = found.detail["base"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string();
                    let files = git::patch(&worktree, &base)
                        .map(|p| {
                            p.files
                                .into_iter()
                                .map(|f| (f.path, f.status, f.added, f.removed))
                                .collect()
                        })
                        .unwrap_or_default();
                    let outcome = TaskOutcome {
                        element: found.detail["element"].as_u64().unwrap_or_default(),
                        repository: found.detail["repository"].as_str().unwrap_or_default().into(),
                        worktree: worktree.display().to_string(),
                        branch: found.detail["branch"].as_str().unwrap_or_default().into(),
                        base,
                        files,
                        notice: Some("The task was interrupted: the Studio did not check it. Run the implementation checks after integrating.".into()),
                        ..Default::default()
                    };
                    found.detail["outcome"] = serde_json::to_value(&outcome).unwrap_or_default();
                    let _ = store.save(&found);
                    self.dialog = Some(crate::edit::Dialog::ReviewTask {
                        job: job.to_string(),
                        problem: None,
                        diff: Self::task_diff(&outcome, 1_500),
                        confirmed: false,
                    });
                    self.mark(Dirty::OVERLAY);
                }
            }
        }
    }

    /// Whether a task's verification still describes it: the same task
    /// commit with nothing uncommitted since, and the same model.
    pub fn task_freshness(&self, job: &Job, outcome: &TaskOutcome) -> agq_simulation::Freshness {
        let worktree = std::path::Path::new(&outcome.worktree);
        let commit = git::head(worktree).map(|h| h.commit).unwrap_or_default();
        match git::changed_files(worktree) {
            Ok(changed) if !changed.is_empty() => {
                return agq_simulation::Freshness::Outdated(format!(
                    "the working copy changed after it was checked ({})",
                    changed.join(", ")
                ));
            }
            Err(refusal) => {
                return agq_simulation::Freshness::Outdated(format!(
                    "the working copy cannot be read: {refusal}"
                ));
            }
            Ok(_) => {}
        }
        let uncommitted = false;
        let digest = match (
            self.project.as_ref(),
            serde_json::from_value::<Brief>(job.detail["brief"].clone()),
        ) {
            (Some(project), Ok(brief)) => brief_digest(project.state().tree(), &brief),
            _ => String::new(),
        };
        outcome
            .verification
            .freshness(&commit, uncommitted, &digest)
    }

    /// Why a task cannot be integrated as it stands: its verification did
    /// not pass, or no longer describes the task. `None` when it can.
    pub fn integration_blocker(&self, job: &Job, outcome: &TaskOutcome) -> Option<String> {
        if outcome.commit.is_empty() {
            return Some(
                "The task has no task commit: the Studio did not check it. Continue the task to have it checked.".into(),
            );
        }
        if let agq_simulation::Freshness::Outdated(why) = self.task_freshness(job, outcome) {
            return Some(format!(
                "The verification is outdated: {why}. Continue the task to check it again."
            ));
        }
        let verification = &outcome.verification;
        if !verification.passed() {
            let not_passed: Vec<String> = verification
                .outcomes()
                .into_iter()
                .filter(|o| o.verdict != agq_simulation::Verdict::Passed)
                .map(|o| format!("{} ({})", o.name, o.verdict.label()))
                .collect();
            return Some(format!(
                "The verification did not pass: {}.",
                not_passed.join(", ")
            ));
        }
        None
    }

    /// Integrates a reviewed task: exactly its task commit, into the
    /// repository's branch (if it has not moved), writing only the task's
    /// files; then adds the proposed links to the model. Refused while the
    /// verification is not current and passed, or while the patch changes
    /// the code of locked parts, unless the Operator `confirmed` after seeing
    /// why. Each step is journaled, so an interruption is finished by
    /// integrating again, never repeated.
    pub fn integrate_task(&mut self, job: &str, confirmed: bool) -> Result<String, String> {
        let (mut found, outcome) = self
            .task_outcome(job)
            .ok_or("The task has nothing to integrate.")?;
        let blocker = self.integration_blocker(&found, &outcome);
        if let Some(why) = &blocker
            && (!confirmed || outcome.commit.is_empty())
        {
            return Err(format!("Not integrated. {why}"));
        }
        let locked = self.locked_code(&outcome);
        if !locked.is_empty() && !confirmed {
            return Err(format!(
                "Not integrated: it changes the code of locked parts ({}). Confirm to integrate it.",
                locked.join(", ")
            ));
        }
        let repository = PathBuf::from(&outcome.repository);
        let store = self.job_store().ok_or("No project is open.")?;
        let mut protected: Vec<String> = ALWAYS_PROTECTED.iter().map(|p| p.to_string()).collect();
        protected.extend(self.implementation_links()?.protected);
        // The task's model changes are checked again here, whatever the
        // worker did: through operations, no locked element, the same locks.
        let model_changes = self.task_model_changes(&outcome);
        if let Some(changes) = &model_changes
            && let Some(why) = &changes.refused
        {
            return Err(format!("Not integrated: {why}"));
        }
        let seq = store
            .begin(
                &mut found,
                "integrate",
                &format!("Integrate the task commit {}", short(&outcome.commit)),
                true,
            )
            .map_err(|e| e.to_string())?;
        let commit =
            match git::integrate_commit(&repository, &outcome.base, &outcome.commit, &protected) {
                Ok(Ok(commit)) => commit,
                Ok(Err(conflict)) => {
                    let _ = store.end(&mut found, seq, false, Some(conflict.to_string()));
                    return Err(format!("Not integrated: {conflict}."));
                }
                Err(refusal) => {
                    let _ = store.end(&mut found, seq, false, Some(refusal.to_string()));
                    return Err(format!("Not integrated: {refusal}"));
                }
            };
        // The accepted model is now the task's: read it again from disk
        // before anything is saved through it.
        if model_changes.is_some()
            && let Some(folder) = self.project.as_ref().map(|p| p.folder().to_path_buf())
        {
            self.open_project(&folder);
        }
        let _ = store.end(
            &mut found,
            seq,
            true,
            Some({
                let mut note = format!("integrated {}", short(&commit));
                if blocker.is_some() {
                    note.push_str(" without a passing verification");
                }
                if !locked.is_empty() {
                    note.push_str(&format!(
                        "; the Operator confirmed its changes to the code of locked parts ({})",
                        locked.join(", ")
                    ));
                }
                note
            }),
        );
        if !outcome.proposed.is_empty() && found.completed("links").is_none() {
            let seq = store
                .begin(
                    &mut found,
                    "links",
                    "Add the proposed links to the model",
                    true,
                )
                .map_err(|e| e.to_string())?;
            let mut links = self.implementation_links()?;
            for link in outcome.proposed {
                if !links.links.contains(&link) {
                    links.links.push(link);
                }
            }
            let saved = self.save_implementation_links(&links);
            let _ = store.end(&mut found, seq, saved.is_ok(), saved.clone().err());
            saved?;
        }
        let _ = git::remove_worktree(&repository, &found.id);
        let _ = store.set_state(
            &mut found,
            JobState::Done,
            Some(format!("integrated as {}", short(&commit))),
        );
        self.refresh_check_freshness();
        self.status = format!(
            "{} integrated: the task commit {} is on the branch. Run the checks to see the code as it is now.",
            found.title,
            short(&commit)
        );
        self.mark(Dirty::STATUS | Dirty::MODEL | Dirty::LAYOUT);
        Ok(commit)
    }

    /// What a task changed in the model (by element identity, from its base
    /// to its task commit), and why it may not be integrated if it may not.
    /// `None` when the task commit leaves the model folder as it was.
    pub fn task_model_changes(&self, outcome: &TaskOutcome) -> Option<ModelChanges> {
        let project = self.project.as_ref()?;
        let model = self.model_folder_in(std::path::Path::new(&outcome.repository))?;
        if outcome.commit.is_empty()
            || !outcome
                .files
                .iter()
                .any(|(path, ..)| agq_execution::within(path, &model))
        {
            return None;
        }
        let read = |commit: &str| -> Result<agq_system_state::SystemState, String> {
            let tree = project.tree_at(commit).map_err(|e| e.to_string())?;
            let locks = project.locks_at(commit).map_err(|e| e.to_string())?;
            Ok(agq_system_state::SystemState::new(tree, locks))
        };
        let (before, after) = match (read(&outcome.base), read(&outcome.commit)) {
            (Ok(before), Ok(after)) => (before, after),
            (Err(why), _) | (_, Err(why)) => {
                return Some(ModelChanges {
                    refused: Some(format!("the task's model cannot be read ({why})")),
                    ..ModelChanges::default()
                });
            }
        };
        let difference = agq_system_state::compare(before.tree(), after.tree());
        let names = |state: &agq_system_state::SystemState, ids: &[ElementId]| -> Vec<String> {
            ids.iter()
                .map(|id| state.tree().qualified_name(*id))
                .collect()
        };
        // A created element's own members (its doc, its features) are part
        // of it: only the outermost are listed.
        let outermost: Vec<ElementId> = difference
            .created
            .iter()
            .copied()
            .filter(|id| {
                after
                    .tree()
                    .get(*id)
                    .and_then(agq_language::Element::owner)
                    .is_none_or(|owner| !difference.created.contains(&owner))
            })
            .collect();
        let mut changes = ModelChanges {
            created: names(&after, &outermost),
            updated: names(&after, &difference.updated),
            deleted: names(&before, &difference.deleted),
            problems: after.diagnostics().len(),
            problems_before: before.diagnostics().len(),
            refused: None,
        };
        // Locked at the task's base, or now (a lock set since).
        let locked: Vec<String> = difference
            .updated
            .iter()
            .chain(&difference.deleted)
            .filter(|id| before.is_locked(**id) || project.state().is_locked(**id))
            .map(|id| before.tree().qualified_name(*id))
            .collect();
        // Only model documents and identities change through a task; the
        // links (with the protected paths) and the rest are the Operator's.
        let others: Vec<&str> = outcome
            .files
            .iter()
            .map(|(path, ..)| path.as_str())
            .filter(|path| agq_execution::within(path, &model))
            .filter(|path| {
                let name = path.rsplit('/').next().unwrap_or_default();
                !(name.ends_with(".sysml") || name == "agentique.json")
            })
            .collect();
        if !others.is_empty() {
            changes.refused = Some(format!(
                "the task changed {}, which only you change",
                others.join(", ")
            ));
        } else if before.locks() != after.locks() {
            changes.refused = Some(
                "the task changed the model's locks, which only you change, in the Studio".into(),
            );
        } else if !locked.is_empty() {
            changes.refused = Some(format!(
                "the task changed locked elements ({}); a task never changes a locked element",
                locked.join(", ")
            ));
        }
        Some(changes)
    }

    /// The locked parts whose linked code a task's patch changes (by the
    /// implementation links, a linked folder covering its files): integrating
    /// it needs the Operator's explicit confirmation.
    pub fn locked_code(&self, outcome: &TaskOutcome) -> Vec<String> {
        let (Some(project), Ok(links)) = (self.project.as_ref(), self.implementation_links())
        else {
            return Vec::new();
        };
        let state = project.state();
        let mut names: Vec<String> = links
            .links
            .iter()
            .filter(|l| {
                outcome
                    .files
                    .iter()
                    .any(|(path, ..)| agq_execution::within(path, &l.path))
            })
            .filter_map(|l| state.lock_of(ElementId::from_raw(l.element)))
            .map(|lock| state.tree().qualified_name(lock))
            .collect();
        names.sort();
        names.dedup();
        names
    }

    /// Whether a task's worker gets its own copy of the model: the model is
    /// in the code repository.
    pub fn task_model_in_repository(&self) -> bool {
        self.implementation_repository()
            .is_some_and(|repository| self.model_folder_in(&repository).is_some())
    }

    /// The project's model folder relative to `repository`, when the code
    /// shares the project's folder: written only through the System State.
    fn model_folder_in(&self, repository: &std::path::Path) -> Option<String> {
        let project = self.project.as_ref()?;
        let code = repository.canonicalize().ok()?;
        let model = project.folder().join("model").canonicalize().ok()?;
        let inside = model.strip_prefix(&code).ok()?;
        Some(inside.to_string_lossy().replace('\\', "/"))
    }

    /// Discards a task's worktree and patch.
    pub fn discard_task(&mut self, job: &str) {
        let Some(store) = self.job_store() else {
            return;
        };
        let Some(mut found) = store.load(job) else {
            return;
        };
        if let Some(repository) = found.detail["repository"].as_str() {
            let _ = git::remove_worktree(std::path::Path::new(repository), &found.id);
        }
        let _ = store.set_state(
            &mut found,
            JobState::Cancelled,
            Some("discarded by the Operator".into()),
        );
        self.status = format!("{} discarded.", found.title);
        self.mark(Dirty::STATUS | Dirty::LAYOUT);
    }

    /// On open: tasks the app did not see end are marked interrupted.
    pub fn recover_tasks(&mut self) {
        let Some(store) = self.job_store() else {
            return;
        };
        let interrupted = store.recover();
        if !interrupted.is_empty() {
            self.status = format!(
                "{} implementation task(s) were interrupted when Agentique closed; review what they wrote or discard it (Inspector › Implementation).",
                interrupted.len()
            );
        }
    }
}

/// The project commands among a task's required checks, as programs the
/// task's executor may run exactly as written.
fn project_programs(required: &[RequiredCheck]) -> Vec<agq_execution::Program> {
    required
        .iter()
        .filter_map(|check| match check {
            RequiredCheck::Command { program, .. } => agq_execution::Program::from_list(program),
            _ => None,
        })
        .collect()
}

/// The first ten characters of a commit id.
fn short(commit: &str) -> &str {
    &commit[..commit.len().min(10)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edit::app_tests::studio;
    use crate::studio::Sample;
    use agq_assistant::Reply;
    use serde_json::json;
    use std::time::Duration;

    fn tool(id: &str, name: &str, input: serde_json::Value) -> serde_json::Value {
        json!({ "type": "tool_use", "id": id, "name": name, "input": input })
    }

    fn reply(content: Vec<serde_json::Value>) -> Reply {
        let stop = if content.iter().any(|c| c["type"] == "tool_use") {
            "tool_use"
        } else {
            "end_turn"
        };
        Reply {
            content,
            stop_reason: stop.into(),
        }
    }

    /// The screening sample with a code folder beside it, trusted.
    fn project(name: &str) -> (Studio, crate::edit::app_tests::Folder, PathBuf) {
        let (mut app, folder) = studio(name);
        app.create_sample(
            &folder.0.join("Screening"),
            crate::studio::SAMPLE_NAME,
            Sample::Screening,
        );
        let code = folder.0.join("code");
        std::fs::create_dir_all(code.join("src")).unwrap();
        std::fs::write(
            code.join("Cargo.toml"),
            "[package]\nname = \"shortener\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\n",
        )
        .unwrap();
        std::fs::write(code.join("src/lib.rs"), "//! The URL shortener.\n").unwrap();
        git::init_and_commit(&code, "Start").unwrap();
        let links = agq_implementation::Links {
            repository: "../code".into(),
            language: "Rust".into(),
            ..Default::default()
        };
        app.save_implementation_links(&links).unwrap();
        let mut choice = app.execution_choice();
        choice.trusted = true;
        app.set_execution_choice(choice);
        app.conversation.key_missing = None;
        (app, folder, code)
    }

    #[test]
    fn a_task_is_verified_by_the_studio_reviewed_and_integrated() {
        let (mut app, _folder, code) = project("task-integrate");
        let tree = app.project.as_ref().unwrap().state().tree();
        let store = tree.find("UrlShortener::LinkStore").unwrap();
        let status = "//! The URL shortener.\n\n/// UrlShortener::LinkStatus\n#[derive(Clone, Copy, Debug, PartialEq, Eq)]\npub enum LinkStatus {\n    Active,\n    Held,\n    Blocked,\n}\n";
        app.conversation.new_runtime = crate::conversation::scripted(vec![
            reply(vec![tool(
                "a",
                "write_code",
                json!({ "path": "src/lib.rs", "text": status }),
            )]),
            reply(vec![tool(
                "b",
                "link_code",
                json!({ "element": "UrlShortener::LinkStatus", "kind": "type", "path": "src/lib.rs", "symbol": "LinkStatus" }),
            )]),
            reply(vec![tool("c", "run_checks", json!({}))]),
            reply(vec![tool(
                "d",
                "finish_implementation",
                json!({ "summary": "LinkStatus, checked." }),
            )]),
            reply(vec![json!({ "type": "text", "text": "Done." })]),
        ]);
        let job = app
            .start_implementation(store, "Start with the status.")
            .expect("started");
        // One task at a time.
        assert!(app.task_blocker().is_some());
        let started = Instant::now();
        while app.implementation.task.is_some() {
            app.poll_task();
            assert!(
                started.elapsed() < Duration::from_secs(300),
                "the task ends"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        let (found, outcome) = app.task_outcome(&job).expect("an outcome");
        assert_eq!(found.state, JobState::WaitingForYou, "{:?}", found.outcome);
        assert!(
            outcome.verification.built,
            "{}",
            outcome.verification.describe()
        );
        // No harness is linked: the six scenarios that guard the link store
        // cannot run against the code. They were said to be not checked when
        // the task started, and the verification still shows each as not
        // run instead of leaving it out (ROADMAP §5.6 item 1); the required
        // check (the build) passed.
        let described = outcome.verification.describe();
        let unchecked = found.detail["notConfigured"].to_string();
        assert!(
            unchecked.contains("No harness is linked: the 6 scenario(s)"),
            "{unchecked}"
        );
        assert!(outcome.verification.passed(), "{described}");
        assert!(
            described.contains("Also checked (not required)"),
            "{described}"
        );
        assert_eq!(
            described
                .matches("no harness is linked, so it cannot run against the code")
                .count(),
            6,
            "{described}"
        );
        assert!(!outcome.commit.is_empty(), "the task commit");
        assert!(
            outcome.files.iter().any(|f| f.0 == "src/lib.rs"),
            "{:?}",
            outcome.files
        );
        assert_eq!(outcome.proposed.len(), 1);
        // Nothing reached the code before the review.
        assert_eq!(
            std::fs::read_to_string(code.join("src/lib.rs")).unwrap(),
            "//! The URL shortener.\n"
        );
        app.review_task(&job);
        assert!(
            matches!(&app.dialog, Some(crate::edit::Dialog::ReviewTask { diff, .. }) if diff.contains("+pub enum LinkStatus"))
        );
        let commit = app.integrate_task(&job, false).expect("integrated");
        assert_eq!(commit, outcome.commit, "exactly the reviewed commit");
        assert_eq!(git::head(&code).unwrap().commit, commit);
        assert!(
            std::fs::read_to_string(code.join("src/lib.rs"))
                .unwrap()
                .contains("pub enum LinkStatus")
        );
        // The proposed link is in the model's links now, and the job is done.
        let links = app.implementation_links().unwrap();
        assert!(
            links
                .links
                .iter()
                .any(|l| l.symbol.as_deref() == Some("LinkStatus"))
        );
        assert_eq!(app.task_outcome(&job).unwrap().0.state, JobState::Done);
        // The model's files were never written.
        assert!(
            app.project
                .as_ref()
                .unwrap()
                .state()
                .diagnostics()
                .is_empty()
        );
    }

    #[test]
    fn an_interrupted_task_is_found_on_open_and_can_be_reviewed() {
        let (mut app, folder, _code) = project("task-recover");
        let tree = app.project.as_ref().unwrap().state().tree();
        let store = tree.find("UrlShortener::LinkStore").unwrap();
        // A worker that writes and then never answers again: the app
        // "closes" while it runs.
        app.conversation.new_runtime = crate::conversation::scripted(vec![reply(vec![tool(
            "a",
            "write_code",
            json!({ "path": "src/store.rs", "text": "pub struct Store;\n" }),
        )])]);
        let job = app.start_implementation(store, "").unwrap();
        let started = Instant::now();
        while app.implementation.task.is_some() {
            app.poll_task();
            assert!(started.elapsed() < Duration::from_secs(300));
            std::thread::sleep(Duration::from_millis(20));
        }
        // Make it look as if the app stopped while the job ran.
        let jobs = app.job_store().unwrap();
        let mut found = jobs.load(&job).unwrap();
        found.detail.as_object_mut().unwrap().remove("outcome");
        jobs.set_state(&mut found, JobState::Running, None).unwrap();
        app.recover_tasks();
        assert_eq!(jobs.load(&job).unwrap().state, JobState::Interrupted);
        assert!(app.status.contains("interrupted"), "{}", app.status);
        app.review_task(&job);
        let (_, outcome) = app.task_outcome(&job).expect("what it wrote is there");
        assert!(
            outcome.files.iter().any(|f| f.0 == "src/store.rs"),
            "{:?}",
            outcome.files
        );
        app.dialog = None;
        // Continue it: the same worktree, and the worktree is not made again.
        app.conversation.new_runtime = crate::conversation::scripted(vec![
            reply(vec![tool(
                "b",
                "read_code",
                json!({ "path": "src/store.rs" }),
            )]),
            reply(vec![tool(
                "c",
                "finish_implementation",
                json!({ "summary": "Continued." }),
            )]),
            reply(vec![json!({ "type": "text", "text": "Done." })]),
        ]);
        app.resume_task(&job).expect("continued");
        let started = Instant::now();
        while app.implementation.task.is_some() {
            app.poll_task();
            assert!(started.elapsed() < Duration::from_secs(300));
            std::thread::sleep(Duration::from_millis(20));
        }
        let continued = jobs.load(&job).unwrap();
        assert_eq!(
            continued
                .steps
                .iter()
                .filter(|s| s.key == "worktree")
                .count(),
            1
        );
        assert!(continued.steps.iter().any(|s| s.key == "resume"));
        let (_, outcome) = app.task_outcome(&job).unwrap();
        assert_eq!(outcome.summary.as_deref(), Some("Continued."));
        assert!(
            outcome.files.iter().any(|f| f.0 == "src/store.rs"),
            "{:?}",
            outcome.files
        );
        app.discard_task(&job);
        assert_eq!(jobs.load(&job).unwrap().state, JobState::Cancelled);
        let _ = folder;
    }

    /// Runs the task in progress to its end.
    fn finish(app: &mut Studio) {
        let started = Instant::now();
        while app.implementation.task.is_some() {
            app.poll_task();
            assert!(
                started.elapsed() < Duration::from_secs(900),
                "the task ends"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Regression (ROADMAP §5.6 item 4): Stop did not reach the processes a
    /// task ran; a required check still running ran on to its timeout. Now
    /// Stop ends it, and it is reported as not run, not as a pass.
    #[test]
    fn stop_ends_the_checks_a_task_is_running() {
        let (mut app, _folder, _code) = project("task-stop");
        app.save_project_checks(&agq_implementation::task::ProjectChecks {
            format: agq_implementation::task::ProjectChecks::FORMAT,
            commands: vec![agq_implementation::task::ProjectCommand {
                id: "slow".into(),
                label: "A slow check".into(),
                program: ["python", "-c", "import time; time.sleep(120)"]
                    .map(String::from)
                    .to_vec(),
            }],
        })
        .unwrap();
        let store = app
            .project
            .as_ref()
            .unwrap()
            .state()
            .tree()
            .find("UrlShortener::LinkStore")
            .unwrap();
        app.conversation.new_runtime = crate::conversation::scripted(vec![
            reply(vec![tool(
                "a",
                "write_code",
                json!({ "path": "src/lib.rs", "text": "//! The URL shortener, edited.\n" }),
            )]),
            reply(vec![tool(
                "b",
                "finish_implementation",
                json!({ "summary": "Edited." }),
            )]),
            reply(vec![json!({ "type": "text", "text": "Done." })]),
        ]);
        let job = app.start_implementation(store, "").unwrap();
        // Wait until the slow check is running, then stop.
        let started = Instant::now();
        while !app.implementation.task.as_ref().is_some_and(|t| {
            t.progress
                .iter()
                .any(|p| p.contains("checks the task commit"))
        }) {
            app.poll_task();
            assert!(started.elapsed() < Duration::from_secs(300));
            std::thread::sleep(Duration::from_millis(20));
        }
        std::thread::sleep(Duration::from_secs(3));
        let stopped = Instant::now();
        app.stop_task();
        finish(&mut app);
        assert!(
            stopped.elapsed() < Duration::from_secs(60),
            "Stop ended the check: {:?}",
            stopped.elapsed()
        );
        let (_, outcome) = app.task_outcome(&job).unwrap();
        let slow = outcome
            .verification
            .outcomes()
            .into_iter()
            .find(|o| o.name == "A slow check")
            .expect("the required check has an outcome");
        assert_eq!(
            slow.verdict,
            agq_simulation::Verdict::NotRun,
            "{}",
            slow.message
        );
        assert!(!outcome.verification.passed());
        // Integrating is refused while a required check has not passed…
        let refused = app.integrate_task(&job, false).unwrap_err();
        assert!(refused.contains("did not pass"), "{refused}");
        assert!(refused.contains("A slow check (not run)"), "{refused}");
        // …and the Operator may still integrate it, explicitly; the task's
        // record says so.
        let commit = app.integrate_task(&job, true).expect("integrated");
        assert_eq!(commit, outcome.commit, "exactly the reviewed commit");
        let journal = app.job_store().unwrap().load(&job).unwrap();
        assert!(
            journal.steps.iter().any(|s| s.key == "integrate"
                && s.note
                    .as_deref()
                    .is_some_and(|r| r.contains("without a passing verification"))),
            "{:?}",
            journal.steps
        );
    }

    /// A verification describes its task commit: a worktree changed after
    /// it was checked makes it outdated, and integration waits.
    #[test]
    fn a_verification_of_another_commit_is_outdated() {
        let (mut app, _folder, _code) = project("task-outdated");
        let store = app
            .project
            .as_ref()
            .unwrap()
            .state()
            .tree()
            .find("UrlShortener::LinkStore")
            .unwrap();
        app.conversation.new_runtime = crate::conversation::scripted(vec![
            reply(vec![tool(
                "a",
                "write_code",
                json!({ "path": "src/lib.rs", "text": "//! The URL shortener, edited.\n" }),
            )]),
            reply(vec![json!({ "type": "text", "text": "Done." })]),
        ]);
        let job = app.start_implementation(store, "").unwrap();
        finish(&mut app);
        let (found, outcome) = app.task_outcome(&job).unwrap();
        assert!(matches!(
            app.task_freshness(&found, &outcome),
            agq_simulation::Freshness::Current
        ));
        std::fs::write(
            std::path::Path::new(&outcome.worktree).join("src/lib.rs"),
            "//! Changed after the check.\n",
        )
        .unwrap();
        assert!(matches!(
            app.task_freshness(&found, &outcome),
            agq_simulation::Freshness::Outdated(_)
        ));
        let blocked = app.integration_blocker(&found, &outcome).unwrap();
        assert!(blocked.contains("outdated"), "{blocked}");
    }

    /// With everything configured (the sample with its code: links, a
    /// harness, linked tests), a task's verification can pass: every
    /// required check passes, and the task integrates without an override.
    #[test]
    fn a_fully_checked_task_passes_and_integrates() {
        let (mut app, folder) = studio("task-passes");
        app.create_sample(
            &folder.0.join("Shortener"),
            "Shortener",
            Sample::ScreeningWithCode,
        );
        let code = folder.0.join("Shortener-code");
        let mut choice = app.execution_choice();
        choice.trusted = true;
        app.set_execution_choice(choice);
        app.conversation.key_missing = None;
        let store = app
            .project
            .as_ref()
            .unwrap()
            .state()
            .tree()
            .find("Shortener::LinkStore")
            .unwrap();
        let api = std::fs::read_to_string(code.join("src/api.rs")).unwrap();
        app.conversation.new_runtime = crate::conversation::scripted(vec![
            reply(vec![tool(
                "a",
                "write_code",
                json!({ "path": "src/api.rs", "text": format!("{api}\n// Reviewed.\n") }),
            )]),
            reply(vec![tool(
                "b",
                "finish_implementation",
                json!({ "summary": "A comment." }),
            )]),
            reply(vec![json!({ "type": "text", "text": "Done." })]),
        ]);
        let job = app.start_implementation(store, "").unwrap();
        finish(&mut app);
        let (found, outcome) = app.task_outcome(&job).unwrap();
        let described = outcome.verification.describe();
        assert!(outcome.verification.passed(), "{described}");
        assert!(
            outcome
                .verification
                .outcomes()
                .iter()
                .any(|o| matches!(o.check, RequiredCheck::Scenario { .. })),
            "{described}"
        );
        assert_eq!(app.integration_blocker(&found, &outcome), None);
        let commit = app.integrate_task(&job, false).expect("integrated");
        assert_eq!(commit, outcome.commit);
        assert!(
            std::fs::read_to_string(code.join("src/api.rs"))
                .unwrap()
                .contains("// Reviewed.")
        );
    }

    /// A project whose model and code share one repository (as Agentique's
    /// own does), with the link store locked; the path is that folder.
    fn one_repository(name: &str) -> (Studio, crate::edit::app_tests::Folder, PathBuf) {
        let (mut app, folder) = studio(name);
        let project = folder.0.join("Shortener");
        app.create_sample(&project, "Shortener", Sample::ScreeningWithCode);
        // The code moves into the project folder: one repository.
        let code = folder.0.join("Shortener-code");
        let mut pending = vec![code.clone()];
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(&dir).unwrap().flatten() {
                let path = entry.path();
                let relative = path.strip_prefix(&code).unwrap().to_path_buf();
                if relative.starts_with(".git") || relative.starts_with("target") {
                    continue;
                }
                if path.is_dir() {
                    std::fs::create_dir_all(project.join(&relative)).unwrap();
                    pending.push(path);
                } else {
                    std::fs::copy(&path, project.join(&relative)).unwrap();
                }
            }
        }
        let mut links = app.implementation_links().unwrap();
        links.repository = ".".into();
        app.save_implementation_links(&links).unwrap();
        let store = app
            .project
            .as_ref()
            .unwrap()
            .state()
            .tree()
            .find("Shortener::LinkStore")
            .unwrap();
        let outcome = app.apply_change(agq_system_state::Change::new(
            agq_system_state::Actor::Operator,
            "Lock the link store",
            vec![agq_system_state::Operation::Lock { element: store }],
        ));
        assert!(matches!(outcome, crate::edit::Outcome::Applied(_)));
        agq_execution::git::init_and_commit(&project, "Model and code in one repository").unwrap();
        let mut choice = app.execution_choice();
        choice.trusted = true;
        app.set_execution_choice(choice);
        app.conversation.key_missing = None;
        (app, folder, project)
    }

    /// W10.4: a worker changes the model of its working copy through System
    /// State operations only (a locked element is refused); the review lists
    /// the model changes beside the code; integration brings exactly the
    /// task commit, and the Studio reads the accepted model again.
    #[test]
    fn a_tasks_model_changes_are_reviewed_and_integrated_with_its_code() {
        let (mut app, _folder, project) = one_repository("task-model");
        let tree = app.project.as_ref().unwrap().state().tree().clone();
        let store = tree.find("Shortener::LinkStore").unwrap();
        let api = std::fs::read_to_string(project.join("src/api.rs")).unwrap();
        app.conversation.new_runtime = crate::conversation::scripted(vec![
            reply(vec![tool(
                "a",
                "apply_changes",
                json!({ "description": "Rename the locked store", "operations": [
                    { "op": "rename", "element": "Shortener::LinkStore", "name": "Links" }
                ] }),
            )]),
            reply(vec![tool(
                "b",
                "apply_changes",
                json!({ "description": "Add a cache", "operations": [
                    { "op": "create", "parent": "Shortener", "kind": "part def", "name": "LinkCache", "doc": "Keeps recent links." }
                ] }),
            )]),
            reply(vec![tool(
                "c",
                "write_code",
                json!({ "path": "src/api.rs", "text": format!("{api}\n// With a cache, later.\n") }),
            )]),
            // A proposed link: saved through the model read again after
            // integration, not the one the task's files replaced.
            reply(vec![tool(
                "l",
                "link_code",
                json!({ "element": "Shortener::LinkApi", "kind": "test", "path": "tests/behaviour.rs", "symbol": "a_confident_allow_redirects" }),
            )]),
            reply(vec![tool(
                "d",
                "finish_implementation",
                json!({ "summary": "Added the cache to the model." }),
            )]),
            reply(vec![json!({ "type": "text", "text": "Done." })]),
        ]);
        let job = app.start_implementation(store, "").unwrap();
        finish(&mut app);
        let (found, outcome) = app.task_outcome(&job).unwrap();
        // The accepted model is untouched until integration.
        let accepted = app.project.as_ref().unwrap().state().tree();
        assert!(accepted.find("Shortener::LinkCache").is_none());
        assert!(
            outcome
                .files
                .iter()
                .any(|(path, ..)| path.starts_with("model/")),
            "{:?}",
            outcome.files
        );
        let changes = app.task_model_changes(&outcome).expect("model changes");
        assert_eq!(
            changes.created,
            vec!["Shortener::LinkCache".to_string()],
            "{changes:?}"
        );
        assert!(changes.refused.is_none(), "{changes:?}");
        assert!(
            !changes.updated.iter().any(|n| n.contains("LinkStore")),
            "the locked store was not renamed: {changes:?}"
        );
        assert!(
            outcome.verification.passed(),
            "{}",
            outcome.verification.describe()
        );
        assert_eq!(app.integration_blocker(&found, &outcome), None);
        let commit = app.integrate_task(&job, false).expect("integrated");
        assert_eq!(commit, outcome.commit);
        let state = app.project.as_ref().unwrap().state();
        assert!(
            state.tree().find("Shortener::LinkCache").is_some(),
            "read again"
        );
        assert_eq!(state.tree().find("Shortener::LinkStore"), Some(store));
        assert!(state.is_locked(store));
        let links = app.implementation_links().unwrap();
        assert!(
            links.links.iter().any(|l| l.name == "Shortener::LinkApi"
                && l.symbol.as_deref() == Some("a_confident_allow_redirects")),
            "the proposed link was saved"
        );
    }

    /// Fail closed: a task commit that changed a locked element without the
    /// System State (here, by editing the model text) is never integrated.
    #[test]
    fn a_task_commit_that_changes_a_locked_element_is_refused() {
        let (app, folder, project) = one_repository("task-locked");
        let base = git::head(&project).unwrap().commit;
        let worktree = folder.0.join("bypass");
        git::checkout_worktree(&project, "bypass", &worktree, &base).unwrap();
        let document = std::fs::read_dir(worktree.join("model"))
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .find(|p| p.extension().is_some_and(|e| e == "sysml"))
            .unwrap();
        let text = std::fs::read_to_string(&document).unwrap();
        let changed = text.replacen(
            "part def LinkStore {",
            "part def LinkStore {\n        attribute bypassed : Boolean;",
            1,
        );
        assert_ne!(text, changed, "the sample has the link store");
        std::fs::write(&document, changed).unwrap();
        let commit = git::commit_worktree(&worktree, "Bypass").unwrap();
        let relative = document
            .strip_prefix(&worktree)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let outcome = TaskOutcome {
            repository: project.display().to_string(),
            worktree: worktree.display().to_string(),
            base,
            commit,
            files: vec![(relative, "modified".into(), 1, 0)],
            ..Default::default()
        };
        let changes = app.task_model_changes(&outcome).expect("model changes");
        let refused = changes.refused.expect("refused");
        assert!(refused.contains("Shortener::LinkStore"), "{refused}");
    }

    /// A lock set since the last checkpoint counts too: a task cannot start
    /// from a model without it, and integration checks the locks of now.
    #[test]
    fn a_lock_set_since_the_last_checkpoint_holds_for_tasks() {
        let (mut app, folder, project) = one_repository("task-new-lock");
        assert_eq!(app.task_blocker(), None);
        let base = git::head(&project).unwrap().commit;
        // A task commit (made by hand) changes the API part's text…
        let worktree = folder.0.join("bypass");
        git::checkout_worktree(&project, "bypass", &worktree, &base).unwrap();
        let document = std::fs::read_dir(worktree.join("model"))
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .find(|p| p.extension().is_some_and(|e| e == "sysml"))
            .unwrap();
        let text = std::fs::read_to_string(&document).unwrap();
        let changed = text.replacen(
            "part def LinkApi {",
            "part def LinkApi {\n        attribute bypassed : Boolean;",
            1,
        );
        assert_ne!(text, changed, "the sample has the API");
        std::fs::write(&document, changed).unwrap();
        // …and links.json, which only the Operator changes.
        let links = worktree.join("model/links.json");
        let without = std::fs::read_to_string(&links)
            .unwrap()
            .replace("tests/behaviour.rs", "tests/other.rs");
        std::fs::write(&links, without).unwrap();
        let commit = git::commit_worktree(&worktree, "Bypass").unwrap();
        let relative = |path: &std::path::Path| {
            path.strip_prefix(&worktree)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        };
        let outcome = TaskOutcome {
            repository: project.display().to_string(),
            worktree: worktree.display().to_string(),
            base,
            commit,
            files: vec![(relative(&document), "modified".into(), 1, 0)],
            ..Default::default()
        };
        // The Operator locks the API now, without a checkpoint.
        let api = app
            .project
            .as_ref()
            .unwrap()
            .state()
            .tree()
            .find("Shortener::LinkApi")
            .unwrap();
        let outcome_of_lock = app.apply_change(agq_system_state::Change::new(
            agq_system_state::Actor::Operator,
            "Lock the API",
            vec![agq_system_state::Operation::Lock { element: api }],
        ));
        assert!(matches!(outcome_of_lock, crate::edit::Outcome::Applied(_)));
        let blocked = app
            .task_blocker()
            .expect("no task from an uncommitted model");
        assert!(blocked.contains("checkpoint"), "{blocked}");
        let refused = app
            .task_model_changes(&outcome)
            .and_then(|c| c.refused)
            .expect("refused");
        assert!(refused.contains("Shortener::LinkApi"), "{refused}");
        // The links file in the same commit is refused on its own.
        let with_links = TaskOutcome {
            files: vec![
                (relative(&document), "modified".into(), 1, 0),
                (relative(&links), "modified".into(), 1, 1),
            ],
            ..outcome
        };
        let refused = app
            .task_model_changes(&with_links)
            .and_then(|c| c.refused)
            .expect("refused");
        assert!(refused.contains("links.json"), "{refused}");
    }

    /// Code linked to a locked part changes only with the Operator's explicit
    /// confirmation at integration, which the task's record keeps.
    #[test]
    fn changing_the_code_of_a_locked_part_asks_at_integration() {
        let (mut app, _folder, project) = one_repository("task-locked-code");
        let links = app.implementation_links().unwrap();
        let path = links
            .links
            .iter()
            .find(|l| l.name == "Shortener::LinkStore" && l.path.ends_with(".rs"))
            .map(|l| l.path.clone())
            .expect("the store's code is linked");
        let tree = app.project.as_ref().unwrap().state().tree().clone();
        let api = tree.find("Shortener::LinkApi").unwrap();
        let text = std::fs::read_to_string(project.join(&path)).unwrap();
        app.conversation.new_runtime = crate::conversation::scripted(vec![
            reply(vec![tool(
                "a",
                "write_code",
                json!({ "path": path, "text": format!("{text}\n// Touched.\n") }),
            )]),
            reply(vec![tool(
                "b",
                "finish_implementation",
                json!({ "summary": "A comment in the store." }),
            )]),
            reply(vec![json!({ "type": "text", "text": "Done." })]),
        ]);
        let job = app.start_implementation(api, "").unwrap();
        finish(&mut app);
        let (_, outcome) = app.task_outcome(&job).unwrap();
        assert_eq!(
            app.locked_code(&outcome),
            vec!["Shortener::LinkStore".to_string()]
        );
        let refused = app.integrate_task(&job, false).unwrap_err();
        assert!(refused.contains("locked parts"), "{refused}");
        app.integrate_task(&job, true)
            .expect("integrated once confirmed");
        let record = app.job_store().unwrap().load(&job).unwrap();
        assert!(
            serde_json::to_string(&record)
                .unwrap()
                .contains("the Operator confirmed its changes to the code of locked parts"),
            "{record:?}"
        );
    }

    /// I4 for real: a worker on a real model implements the link store of
    /// the URL shortener from its model, in a worktree; the Studio checks
    /// it, the patch is integrated, and the scenarios run against the code.
    /// It spends money, so it runs only when asked: `AGQ_LIVE=1 cargo test
    /// -p agq-studio-native live_worker -- --ignored --nocapture`, with a
    /// provider key set (the Assistant's model choice).
    #[test]
    #[ignore = "calls a real model"]
    fn live_worker_implements_the_link_store() {
        if std::env::var("AGQ_LIVE").as_deref() != Ok("1") {
            eprintln!("Set AGQ_LIVE=1 to run the live worker.");
            return;
        }
        let (mut app, folder) = studio("task-live");
        app.create_sample(
            &folder.0.join("Shortener"),
            "Shortener",
            Sample::ScreeningWithCode,
        );
        let code = folder.0.join("Shortener-code");
        // The link store is still to be written: the base does not build.
        std::fs::write(
            code.join("src/store.rs"),
            "//! Shortener::LinkStore: to be implemented from the model.\n",
        )
        .unwrap();
        git::init_and_commit(&code, "The link store is still to be written").unwrap();
        let mut choice = app.execution_choice();
        choice.trusted = true;
        app.set_execution_choice(choice);
        let model_choice = agq_assistant::ModelChoice::from_env();
        assert!(
            model_choice.has_key(),
            "{}",
            model_choice.missing_key_message()
        );
        println!("MODEL {}", model_choice.label());
        app.conversation.new_runtime =
            Box::new(move || agq_assistant::LoopRuntime::boxed(model_choice.start()));
        app.conversation.key_missing = None;
        let store = app
            .project
            .as_ref()
            .unwrap()
            .state()
            .tree()
            .find("Shortener::LinkStore")
            .unwrap();
        let started = Instant::now();
        let job = app
            .start_implementation(
                store,
                "Keep it small; the rest of the crate already exists.",
            )
            .expect("started");
        let mut shown = 0;
        while app.implementation.task.is_some() {
            app.poll_task();
            if let Some(task) = &app.implementation.task
                && task.progress.len() > shown
            {
                for line in &task.progress[shown..] {
                    println!("  {line}");
                }
                shown = task.progress.len();
            }
            assert!(
                started.elapsed() < Duration::from_secs(1800),
                "the task ends"
            );
            std::thread::sleep(Duration::from_millis(200));
        }
        let (_, outcome) = app.task_outcome(&job).expect("an outcome");
        println!("TOOK {:.0} s", started.elapsed().as_secs_f64());
        println!("ROUNDS {}", outcome.rounds);
        println!("COST {:?} on {:?}", outcome.cost_usd, outcome.model);
        println!("SUMMARY {:?}", outcome.summary);
        println!("NOTICE {:?}", outcome.notice);
        println!("CONTRACT REQUESTS {:?}", outcome.contract_requests);
        println!("FILES {:?}", outcome.files);
        println!("PROPOSED {}", outcome.proposed.len());
        println!("STUDIO CHECK\n{}", outcome.verification.describe());
        if outcome.verification.failures() > 0 {
            println!("NOT INTEGRATED: the Studio's check failed");
            return;
        }
        let commit = app.integrate_task(&job, false).expect("integrated");
        println!("INTEGRATED {commit}");
        let tree = app.project.as_ref().unwrap().state().tree();
        let scenarios: Vec<ElementId> = tree
            .walk()
            .into_iter()
            .filter(|id| tree[*id].kind == agq_language::ElementKind::VerificationDef)
            .collect();
        for scenario in scenarios {
            app.select_scenario(scenario);
            app.start_run(agq_simulation::Mode::Implementation);
            let started = Instant::now();
            while !app.poll_runs() {
                assert!(started.elapsed() < Duration::from_secs(600));
                std::thread::sleep(Duration::from_millis(20));
            }
            let result = app.runs.result.clone().unwrap();
            println!(
                "RUN {}: {}{}",
                result.scenario_name,
                result.status.label(),
                if result.all_passed() {
                    ", every check passed"
                } else {
                    ""
                }
            );
        }
    }
}
