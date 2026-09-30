//! Implementation tasks in the Studio (ROADMAP §4.15, W8.4): the Operator
//! (or the Assistant, after the Operator agrees) asks a worker to implement
//! an element. Each task is a job in the project's app data, with its own
//! git worktree; the worker runs on its own thread; when it ends, the
//! Studio verifies the worktree itself and the Operator reviews the patch,
//! then integrates it (the repository must not have moved) or discards it.
//! A task the app did not see end is found on the next open as
//! interrupted, with what it wrote still there to review.
use crate::studio::{Dirty, Studio};
use agq_assistant::worker::{self, Worker};
use agq_execution::jobs::{Job, JobState, JobStore};
use agq_execution::{Executor, Scope, git};
use agq_implementation::Link;
use agq_implementation::task::{Verification, brief, verify};
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
}

/// A task in progress.
pub struct ActiveTask {
    pub job: String,
    pub element: ElementId,
    pub title: String,
    pub started: Instant,
    /// The worker's latest steps, newest last.
    pub progress: Vec<String>,
    events: Receiver<TaskEvent>,
    cancel: Arc<AtomicBool>,
}

enum TaskEvent {
    Progress(String),
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
        // The worker uses the Assistant's model.
        if let Some(missing) = &self.conversation.key_missing {
            return Some(missing.clone());
        }
        None
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
        if let (Ok(code), Ok(model)) = (
            repository.canonicalize(),
            project.folder().join("model").canonicalize(),
        ) && let Ok(inside) = model.strip_prefix(&code)
        {
            brief
                .protected
                .push(inside.to_string_lossy().replace('\\', "/"));
        }
        let store = self.job_store().ok_or("No project is open.")?;
        let mut job = store
            .create(
                "implement",
                &brief.title,
                serde_json::json!({ "element": element.raw(), "instructions": instructions }),
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
        let repository = PathBuf::from(found.detail["repository"].as_str().unwrap_or_default());
        if let (Ok(code), Ok(model)) = (
            repository.canonicalize(),
            project.folder().join("model").canonicalize(),
        ) && let Ok(inside) = model.strip_prefix(&code)
        {
            brief
                .protected
                .push(inside.to_string_lossy().replace('\\', "/"));
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
        let make_executor = {
            let root = worktree.path.clone();
            let protected = brief.protected.clone();
            move || -> Result<Executor, String> {
                let protected: Vec<&str> = protected.iter().map(String::as_str).collect();
                let scope = Scope::writable(&root, &[""], &protected).map_err(|e| e.to_string())?;
                Ok(Executor::new(scope)
                    .trusted(choice.trusted)
                    .network(choice.network)
                    .target_dir(target.clone()))
            }
        };
        let executor = make_executor()?;
        let carried = carried.to_vec();
        let mut model = (self.conversation.new_model)();
        let cancel = Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        let (sender, events) = std::sync::mpsc::channel();
        let title = brief.title.clone();
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
                // Links an earlier attempt proposed stay proposed.
                worker.proposed = carried;
                let progress = sender.clone();
                let mut on_event = |event| {
                    if let agq_assistant::TurnEvent::ToolFinished(result) = event {
                        let first = result.content.lines().next().unwrap_or("").to_string();
                        let line = if result.is_error {
                            format!("✗ {first}")
                        } else {
                            format!("✓ {first}")
                        };
                        let _ = progress.send(TaskEvent::Progress(line));
                    }
                };
                let conversation =
                    worker::run_worker(model.as_mut(), &mut worker, &mut on_event, &flag);
                let notice = conversation.entries.iter().rev().find_map(|e| match e {
                    agq_assistant::Entry::Notice { text } => Some(text.clone()),
                    _ => None,
                });
                let _ = sender.send(TaskEvent::Progress(
                    "The Studio checks the worktree itself…".into(),
                ));
                // Never the worker's word: the Studio's own verification.
                let verification = match make_executor() {
                    Ok(executor) => verify(&tree, &worker.links(), &brief, &executor, flag.clone()),
                    Err(error) => Verification {
                        build_errors: error,
                        ..Default::default()
                    },
                };
                let files = git::patch(
                    std::path::Path::new(&outcome_base.worktree),
                    &outcome_base.base,
                )
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
                    "{} file(s) changed; {} failing or not verified",
                    outcome.files.len(),
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
                "{} is ready for review: {} file(s), {} failing or not verified.",
                task.title,
                outcome.files.len(),
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

    /// The patch of a task's worktree as text, at most `lines` lines.
    pub fn task_diff(outcome: &TaskOutcome, lines: usize) -> String {
        let text = git::patch(std::path::Path::new(&outcome.worktree), &outcome.base)
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
                    });
                    self.mark(Dirty::OVERLAY);
                }
            }
        }
    }

    /// Integrates a reviewed patch: commits it to the repository (if its
    /// head has not moved) and adds the proposed links to the model.
    pub fn integrate_task(&mut self, job: &str) -> Result<String, String> {
        let (mut found, outcome) = self
            .task_outcome(job)
            .ok_or("The task has nothing to integrate.")?;
        let message = format!("{} (Agentique task {})", found.title, found.id);
        let repository = PathBuf::from(&outcome.repository);
        let commit = match git::integrate(
            &repository,
            std::path::Path::new(&outcome.worktree),
            &outcome.base,
            &message,
        ) {
            Ok(Ok(commit)) => commit,
            Ok(Err(conflict)) => return Err(format!("Not integrated: {conflict}.")),
            Err(refusal) => return Err(format!("Not integrated: {refusal}")),
        };
        if !outcome.proposed.is_empty() {
            let mut links = self.implementation_links()?;
            for link in outcome.proposed {
                if !links.links.contains(&link) {
                    links.links.push(link);
                }
            }
            self.save_implementation_links(&links)?;
        }
        let _ = git::remove_worktree(&repository, &found.id);
        if let Some(store) = self.job_store() {
            let _ = store.set_state(
                &mut found,
                JobState::Done,
                Some(format!("integrated as {}", &commit[..commit.len().min(10)])),
            );
        }
        self.refresh_check_freshness();
        self.status = format!(
            "{} integrated as {}. Run the checks to see the code as it is now.",
            found.title,
            &commit[..commit.len().min(10)]
        );
        self.mark(Dirty::STATUS | Dirty::MODEL | Dirty::LAYOUT);
        Ok(commit)
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
        app.conversation.new_model = crate::conversation::scripted(vec![
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
        assert_eq!(
            outcome.verification.failures(),
            0,
            "{}",
            outcome.verification.describe()
        );
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
        let commit = app.integrate_task(&job).expect("integrated");
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
        app.conversation.new_model = crate::conversation::scripted(vec![reply(vec![tool(
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
        app.conversation.new_model = crate::conversation::scripted(vec![
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
}
