//! Jobs (ROADMAP §4.15): long-running work with side effects, with a stable
//! id, a state, and a journal in the app's per-project data. Each side
//! effect is written to the journal as started before it happens and as
//! completed after, so a crash leaves an honest record: completed steps are
//! never repeated on resume, and a step that started but did not record its
//! end is shown as unknown until someone reconciles it.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Where a job stands; the Studio's state vocabulary (§3.2) plus
/// `cancelled` and `interrupted`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JobState {
    Pending,
    Running,
    WaitingForYou,
    Done,
    Failed,
    Refused,
    Cancelled,
    /// The app stopped while it was running (found on the next open).
    Interrupted,
}

impl JobState {
    pub fn label(self) -> &'static str {
        match self {
            JobState::Pending => "pending",
            JobState::Running => "running",
            JobState::WaitingForYou => "waiting for you",
            JobState::Done => "done",
            JobState::Failed => "failed",
            JobState::Refused => "refused",
            JobState::Cancelled => "cancelled",
            JobState::Interrupted => "interrupted",
        }
    }

    /// Whether the job can go on (after the Operator resumes it).
    pub fn open(self) -> bool {
        matches!(
            self,
            JobState::Pending | JobState::Running | JobState::WaitingForYou | JobState::Interrupted
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StepState {
    Started,
    Completed,
    Failed,
}

/// One step of a job.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobStep {
    pub seq: u32,
    /// A stable key for the step, such as `worktree` or `integrate`.
    pub key: String,
    /// What it does, in plain words.
    pub what: String,
    /// Whether it changes anything outside the job's own worktree.
    pub side_effect: bool,
    pub state: StepState,
    pub started: String,
    #[serde(default)]
    pub finished: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

/// A job and its journal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub format: u32,
    pub id: String,
    /// `implementation task`, `implementation run`, `live evaluation`.
    pub kind: String,
    pub title: String,
    pub state: JobState,
    pub created: String,
    pub updated: String,
    pub steps: Vec<JobStep>,
    /// What the job needs to resume, owned by whoever runs it.
    #[serde(default)]
    pub detail: Value,
    /// The last outcome, in plain words.
    #[serde(default)]
    pub outcome: Option<String>,
}

const FORMAT: u32 = 1;

impl Job {
    /// The step with `key` that completed, if any: resume skips it.
    pub fn completed(&self, key: &str) -> Option<&JobStep> {
        self.steps
            .iter()
            .rev()
            .find(|s| s.key == key && s.state == StepState::Completed)
    }

    /// Steps that started and never recorded their end.
    pub fn unknown(&self) -> Vec<&JobStep> {
        self.steps
            .iter()
            .filter(|s| s.state == StepState::Started)
            .collect()
    }
}

/// The jobs of one project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JobStore {
    folder: PathBuf,
}

impl JobStore {
    pub fn new(folder: impl Into<PathBuf>) -> JobStore {
        JobStore {
            folder: folder.into(),
        }
    }

    pub fn folder(&self) -> &Path {
        &self.folder
    }

    /// A new job, saved as pending.
    pub fn create(&self, kind: &str, title: &str, detail: Value) -> std::io::Result<Job> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let now = now();
        let job = Job {
            format: FORMAT,
            id: format!(
                "job-{millis:x}-{:x}",
                COUNTER.fetch_add(1, Ordering::SeqCst)
            ),
            kind: kind.to_string(),
            title: title.to_string(),
            state: JobState::Pending,
            created: now.clone(),
            updated: now,
            steps: Vec::new(),
            detail,
            outcome: None,
        };
        self.save(&job)?;
        Ok(job)
    }

    /// Writes the journal atomically.
    pub fn save(&self, job: &Job) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.folder)?;
        let path = self.folder.join(format!("{}.json", job.id));
        let temporary = self.folder.join(format!("{}.json.tmp", job.id));
        let text = serde_json::to_string_pretty(job).map_err(std::io::Error::other)?;
        {
            use std::io::Write;
            let mut file = std::fs::File::create(&temporary)?;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
        }
        std::fs::rename(temporary, path)
    }

    pub fn load(&self, id: &str) -> Option<Job> {
        if id.contains(['/', '\\']) || id.contains("..") {
            return None;
        }
        let text = std::fs::read_to_string(self.folder.join(format!("{id}.json"))).ok()?;
        let job: Job = serde_json::from_str(&text).ok()?;
        (job.format == FORMAT).then_some(job)
    }

    /// Every readable job, newest first.
    pub fn list(&self) -> Vec<Job> {
        let Ok(entries) = std::fs::read_dir(&self.folder) else {
            return Vec::new();
        };
        let mut jobs: Vec<Job> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            .filter_map(|p| std::fs::read_to_string(p).ok())
            .filter_map(|t| serde_json::from_str::<Job>(&t).ok())
            .filter(|j| j.format == FORMAT)
            .collect();
        jobs.sort_by(|a, b| b.created.cmp(&a.created).then(b.id.cmp(&a.id)));
        jobs
    }

    /// On opening a project: jobs left running are interrupted. Returns them.
    pub fn recover(&self) -> Vec<Job> {
        let mut interrupted = Vec::new();
        for mut job in self.list() {
            if job.state == JobState::Running {
                job.state = JobState::Interrupted;
                job.updated = now();
                let unknown = job.unknown().len();
                job.outcome = Some(if unknown > 0 {
                    format!(
                        "Agentique stopped while this job was running; {unknown} step(s) started and did not record their end, so whether they completed is unknown"
                    )
                } else {
                    "Agentique stopped while this job was running; every step it began had finished"
                        .into()
                });
                if self.save(&job).is_ok() {
                    interrupted.push(job);
                }
            }
        }
        interrupted
    }

    pub fn set_state(
        &self,
        job: &mut Job,
        state: JobState,
        outcome: Option<String>,
    ) -> std::io::Result<()> {
        job.state = state;
        job.updated = now();
        if outcome.is_some() {
            job.outcome = outcome;
        }
        self.save(job)
    }

    /// Records a step as started, durably, before it happens.
    pub fn begin(
        &self,
        job: &mut Job,
        key: &str,
        what: &str,
        side_effect: bool,
    ) -> std::io::Result<u32> {
        let seq = job.steps.len() as u32 + 1;
        job.steps.push(JobStep {
            seq,
            key: key.to_string(),
            what: what.to_string(),
            side_effect,
            state: StepState::Started,
            started: now(),
            finished: None,
            note: None,
        });
        job.updated = now();
        self.save(job)?;
        Ok(seq)
    }

    /// Records how a step ended.
    pub fn end(
        &self,
        job: &mut Job,
        seq: u32,
        ok: bool,
        note: Option<String>,
    ) -> std::io::Result<()> {
        if let Some(step) = job.steps.iter_mut().find(|s| s.seq == seq) {
            step.state = if ok {
                StepState::Completed
            } else {
                StepState::Failed
            };
            step.finished = Some(now());
            step.note = note;
        }
        job.updated = now();
        self.save(job)
    }
}

fn now() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // RFC 3339 in UTC, as results write it.
    let days = (seconds / 86_400) as i64;
    let rest = seconds % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        (rest % 3600) / 60,
        rest % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_crash_leaves_an_honest_journal_and_resume_skips_completed_steps() {
        let dir = tempfile::tempdir().unwrap();
        let store = JobStore::new(dir.path().join("jobs"));
        let mut job = store
            .create(
                "implementation task",
                "Implement link screening",
                serde_json::json!({"base": "abc"}),
            )
            .unwrap();
        store.set_state(&mut job, JobState::Running, None).unwrap();
        let worktree = store
            .begin(&mut job, "worktree", "Create the worktree", false)
            .unwrap();
        store.end(&mut job, worktree, true, None).unwrap();
        let _integrate = store
            .begin(&mut job, "integrate", "Integrate the patch", true)
            .unwrap();
        // The app stops here: the integration's end is never recorded.
        drop(job);
        let interrupted = store.recover();
        assert_eq!(interrupted.len(), 1);
        let job = &interrupted[0];
        assert_eq!(job.state, JobState::Interrupted);
        assert!(job.completed("worktree").is_some());
        assert!(job.completed("integrate").is_none());
        assert_eq!(job.unknown().len(), 1);
        assert!(job.outcome.as_ref().unwrap().contains("unknown"));
        // Recovering again changes nothing.
        assert!(store.recover().is_empty());
        let listed = store.list();
        assert_eq!(listed[0].id, job.id);
        assert_eq!(store.load(&job.id).unwrap().state, JobState::Interrupted);
        assert!(store.load("../x").is_none());
    }
}
