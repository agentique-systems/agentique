//! The testing knowledge (C-54, ROADMAP §4.16 "Testing knowledge"): per
//! project, what exploration has covered (coverage keys with counts and the
//! builds first and last seen), the findings and their state (open,
//! reproduced, not reproduced, fixed in a commit, failing again), and how
//! each run did (way of deciding, goal, steps, new coverage, findings, cost,
//! latency). Kept with the Orchestrator's records in the app data
//! (`testing/<project>/knowledge.json`, format 1), written atomically and
//! bounded. The next run prefers what is not covered, first replays the
//! findings fixed since, and the ways of deciding are compared by their
//! runs.
//!
//! Since C-55 a finding keeps its disposition (what the lead judged it to
//! be before anything was fixed), across objectives: one judged a wrong
//! expectation or an unreliable reproduction is not offered again, and
//! found again it is already adjudicated, not new.

use crate::decide::Way;
use crate::explore::Run;
use crate::findings::{Disposition, DispositionKind, Finding, Replay, State};
use crate::record::Store;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The file's format; a build that does not know it does not read it.
pub const FORMAT: u32 = 1;

/// A finding's id in a lead's brief, from its place in its cycle's findings
/// (`f1` for the first): stable for the cycle, never renumbered when other
/// findings are set aside, so an id names one finding in every session.
pub fn finding_id(index: usize) -> String {
    format!("f{}", index + 1)
}

/// Bounds: coverage keys, runs and findings kept. The least recently
/// covered keys go first; the oldest runs; the oldest findings that propose
/// nothing (not reproduced, or fixed and passing), then the oldest.
pub const COVERAGE: usize = 5000;
pub const RUNS: usize = 50;
pub const FINDINGS: usize = 200;

/// One coverage key.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Covered {
    pub count: u64,
    pub first_build: String,
    pub last_build: String,
    /// The run that last covered it (runs are numbered from 1).
    pub last_run: u64,
}

/// How one run did.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRecord {
    pub n: u64,
    pub at: String,
    pub way: Way,
    pub goal: String,
    pub build: String,
    /// The commit the build was made from (C-54: the next run prefers what
    /// changed since).
    #[serde(default)]
    pub commit: String,
    pub seed: u64,
    pub steps: u32,
    pub new_coverage: u32,
    /// The identities of its findings.
    pub findings: Vec<String>,
    pub usd: f64,
    pub unpriced: u32,
    pub latency_p50_ms: u64,
    pub latency_p95_ms: u64,
    pub recoveries: u32,
    pub unwanted: u32,
    pub seconds: f64,
}

/// What the runs of one way of deciding did, together.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WayResults {
    pub runs: u32,
    pub steps: u32,
    pub new_coverage: u32,
    pub findings: u32,
    pub unwanted: u32,
    pub usd: f64,
}

/// A project's testing knowledge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Knowledge {
    pub format: u32,
    pub project: String,
    /// Runs recorded so far (they number the runs).
    #[serde(default)]
    pub runs_made: u64,
    #[serde(default)]
    pub coverage: BTreeMap<String, Covered>,
    #[serde(default)]
    pub findings: Vec<Finding>,
    #[serde(default)]
    pub runs: Vec<RunRecord>,
}

impl Knowledge {
    pub fn new(project: &str) -> Knowledge {
        Knowledge {
            format: FORMAT,
            project: project.to_string(),
            runs_made: 0,
            coverage: BTreeMap::new(),
            findings: Vec::new(),
            runs: Vec::new(),
        }
    }

    /// A project's folder as a folder name: its own name in lowercase
    /// letters, digits and hyphens, and eight hexadecimal digits of its
    /// full path (as the system resolves it), so two projects with one name
    /// keep apart. `repository` must be the project's own folder (the
    /// objective's repository, which W12.5 passes), never a cycle's
    /// worktree or checkout: those change from cycle to cycle, and the
    /// knowledge would start empty each time.
    pub fn key(repository: &Path) -> String {
        let full = repository
            .canonicalize()
            .unwrap_or_else(|_| repository.to_path_buf());
        let full = full.display().to_string();
        let full = if cfg!(windows) {
            full.to_lowercase()
        } else {
            full
        };
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for byte in full.bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        let name = repository
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let mut key = String::new();
        for c in name.trim().chars().flat_map(char::to_lowercase) {
            if c.is_ascii_alphanumeric() {
                key.push(c);
            } else if !key.ends_with('-') {
                key.push('-');
            }
        }
        let key = key.trim_matches('-');
        let key = if key.is_empty() { "project" } else { key };
        format!("{key}-{:08x}", hash as u32)
    }

    /// Where a project's knowledge is kept: `testing/<key>/knowledge.json`
    /// in the app data folder that holds the objectives' records.
    pub fn file(store: &Store, repository: &Path) -> PathBuf {
        store
            .folder
            .parent()
            .unwrap_or(&store.folder)
            .join("testing")
            .join(Knowledge::key(repository))
            .join("knowledge.json")
    }

    /// Changes the knowledge in `path` under its lock file: read, changed
    /// by `change`, written atomically. Another writer waits for the lock,
    /// so no change is lost.
    pub fn change<T>(
        path: &Path,
        project: &str,
        change: impl FnOnce(&mut Knowledge) -> T,
    ) -> Result<T, String> {
        let folder = path.parent().unwrap_or(Path::new("."));
        std::fs::create_dir_all(folder).map_err(|e| format!("{}: {e}", folder.display()))?;
        let lock = folder.join("knowledge.lock");
        let lock = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&lock)
            .map_err(|e| format!("{}: {e}", lock.display()))?;
        lock.lock()
            .map_err(|e| format!("the knowledge's lock: {e}"))?;
        let mut knowledge = Knowledge::load(path, project)?;
        let out = change(&mut knowledge);
        knowledge.save(path)?;
        Ok(out)
    }

    /// The knowledge in `path`, or a new one when there is none yet; a file
    /// in another format, or unreadable, is an error (never overwritten
    /// unread).
    pub fn load(path: &Path, project: &str) -> Result<Knowledge, String> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Knowledge::new(project));
            }
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        let knowledge: Knowledge =
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        if knowledge.format != FORMAT {
            return Err(format!(
                "{} is in format {}, which this build does not read",
                path.display(),
                knowledge.format
            ));
        }
        Ok(knowledge)
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        agq_launcher::write_atomically(path, text.as_bytes())
    }

    /// How often a coverage key was covered.
    pub fn count(&self, key: &str) -> u64 {
        self.coverage.get(key).map_or(0, |c| c.count)
    }

    /// The keys never covered.
    pub fn uncovered<'a>(&self, keys: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
        keys.into_iter().filter(|k| self.count(k) == 0).collect()
    }

    /// The fixed findings not yet replayed in `build`: a run there replays
    /// them first.
    pub fn to_replay(&self, build: &str) -> Vec<&Finding> {
        self.findings
            .iter()
            .filter(|f| f.state == State::Fixed && f.checked_in.as_deref() != Some(build))
            .collect()
    }

    /// The commit of the last run that says which: what changed since is
    /// what the next run prefers.
    pub fn last_commit(&self) -> Option<&str> {
        self.runs
            .iter()
            .rev()
            .map(|r| r.commit.as_str())
            .find(|c| !c.is_empty())
    }

    /// The runs' results by way of deciding.
    pub fn ways(&self) -> BTreeMap<Way, WayResults> {
        let mut ways: BTreeMap<Way, WayResults> = BTreeMap::new();
        for run in &self.runs {
            let w = ways.entry(run.way).or_default();
            w.runs += 1;
            w.steps += run.steps;
            w.new_coverage += run.new_coverage;
            w.findings += run.findings.len() as u32;
            w.unwanted += run.unwanted;
            w.usd += run.usd;
        }
        ways
    }

    /// Adds what a run covered and found, and how it did. A finding already
    /// known is not added twice; one that was fixed and is found again is
    /// failing again.
    pub fn add_run(&mut self, run: &Run) {
        self.runs_made += 1;
        let n = self.runs_made;
        for (key, count) in &run.covered {
            let entry = self.coverage.entry(key.clone()).or_insert(Covered {
                count: 0,
                first_build: run.build.clone(),
                last_build: run.build.clone(),
                last_run: n,
            });
            entry.count += u64::from(*count);
            entry.last_build = run.build.clone();
            entry.last_run = n;
        }
        for finding in &run.findings {
            match self
                .findings
                .iter_mut()
                .find(|f| f.identity == finding.identity)
            {
                Some(known) if known.state == State::Fixed => {
                    let note = format!(
                        "fixed in {}, found again in build {}",
                        known.fixed_in.as_deref().unwrap_or("an earlier change"),
                        run.build
                    );
                    known.set_state(State::FailingAgain, &note);
                }
                // Found again after it was judged a wrong expectation: it
                // stays adjudicated (C-55), and says when it was seen.
                Some(known)
                    if known
                        .disposition
                        .as_ref()
                        .is_some_and(|d| d.kind == DispositionKind::WrongExpectation) =>
                {
                    known.note = format!(
                        "found again in build {}; already adjudicated as a wrong expectation",
                        run.build
                    );
                }
                // Judged an unreliable reproduction, and found again on
                // another build than the one it was judged on: offered
                // again, to be reproduced there.
                Some(known) if Knowledge::found_again(known, &run.build) => {
                    known.disposition = None;
                    known.note = format!(
                        "found again in build {}, another than it was judged an unreliable reproduction on: offered again",
                        run.build
                    );
                }
                Some(_) => {}
                None => self.findings.push(finding.clone()),
            }
        }
        self.runs.push(RunRecord {
            n,
            at: run.started.clone(),
            way: run.plan.way,
            goal: run.plan.goal.clone(),
            build: run.build.clone(),
            commit: run.commit.clone(),
            seed: run.plan.seed,
            steps: run.actions,
            new_coverage: run.new_coverage.len() as u32,
            findings: run.findings.iter().map(|f| f.identity.clone()).collect(),
            usd: run.usd,
            unpriced: run.unpriced,
            latency_p50_ms: run.latency(0.5),
            latency_p95_ms: run.latency(0.95),
            recoveries: run.recoveries.len() as u32,
            unwanted: run.unwanted.total(),
            seconds: run.seconds,
        });
        self.bound();
    }

    /// Keeps what became of a finding (after reproducing it), by
    /// identity, merged into what is known: its fix (commit, pull request,
    /// the build it was checked in) and its history stay, its replays are
    /// added, and a fixed finding that reproduces again is failing again,
    /// while one that did not reproduce stays as it was.
    pub fn update(&mut self, finding: &Finding) {
        let Some(known) = self
            .findings
            .iter_mut()
            .find(|f| f.identity == finding.identity)
        else {
            self.findings.push(finding.clone());
            self.bound();
            return;
        };
        if finding.replays.starts_with(&known.replays) {
            known.replays = finding.replays.clone();
        } else {
            known.replays.extend(finding.replays.iter().cloned());
        }
        if finding.reduced.is_some() {
            known.reduced = finding.reduced.clone();
        }
        known.fixed_in = known.fixed_in.take().or(finding.fixed_in.clone());
        known.pull_request = known.pull_request.or(finding.pull_request);
        known.checked_in = known.checked_in.take().or(finding.checked_in.clone());
        // A later judgment replaces an earlier one; none, or an older copy,
        // leaves it as it is.
        if let Some(newer) = &finding.disposition
            && known.disposition.as_ref().is_none_or(|d| d.at <= newer.at)
        {
            known.disposition = Some(newer.clone());
        }
        let state = match (known.state, finding.state) {
            (State::Fixed | State::FailingAgain, State::Reproduced) => State::FailingAgain,
            (State::Fixed | State::FailingAgain, State::Open | State::NotReproduced) => known.state,
            (_, new) => new,
        };
        if state != known.state || finding.note != known.note {
            let note = if state == State::FailingAgain && known.state == State::Fixed {
                format!(
                    "fixed in {}, reproduced again",
                    known.fixed_in.as_deref().unwrap_or("an earlier change")
                )
            } else {
                finding.note.clone()
            };
            known.set_state(state, &note);
        }
        self.bound();
    }

    /// A merged change fixed the finding `identity`.
    pub fn fixed(&mut self, identity: &str, commit: &str, pull_request: Option<u64>) -> bool {
        let Some(finding) = self.findings.iter_mut().find(|f| f.identity == identity) else {
            return false;
        };
        finding.fixed_in = Some(commit.to_string());
        finding.pull_request = pull_request;
        finding.checked_in = None;
        finding.set_state(State::Fixed, &format!("fixed in {commit}"));
        true
    }

    /// A fixed finding replayed in `build`: passing keeps it fixed (checked
    /// there); failing makes it failing again, a regression.
    pub fn replayed(&mut self, identity: &str, build: &str, replay: &Replay) {
        let Some(finding) = self.findings.iter_mut().find(|f| f.identity == identity) else {
            return;
        };
        finding.replays.push(replay.clone());
        match replay {
            Replay::Passed => finding.checked_in = Some(build.to_string()),
            Replay::Failed { .. } => {
                let note = format!(
                    "fixed in {}, fails again in build {build}",
                    finding.fixed_in.as_deref().unwrap_or("an earlier change")
                );
                finding.set_state(State::FailingAgain, &note);
            }
            Replay::Diverged { .. } => {}
        }
    }

    /// Records what `finding` was judged to be (C-55), replacing an earlier
    /// judgment; a finding the knowledge no longer keeps is kept again.
    pub fn adjudicate(&mut self, finding: &Finding, disposition: Disposition) {
        match self
            .findings
            .iter_mut()
            .find(|f| f.identity == finding.identity)
        {
            Some(known) => known.disposition = Some(disposition),
            None => {
                let mut kept = finding.clone();
                kept.disposition = Some(disposition);
                self.findings.push(kept);
            }
        }
        self.bound();
    }

    /// The disposition of the finding `identity`, if it was adjudicated.
    pub fn disposition(&self, identity: &str) -> Option<&Disposition> {
        self.findings
            .iter()
            .find(|f| f.identity == identity)
            .and_then(|f| f.disposition.as_ref())
    }

    /// Every finding's disposition, by identity.
    pub fn dispositions(&self) -> BTreeMap<String, Disposition> {
        self.findings
            .iter()
            .filter_map(|f| Some((f.identity.clone(), f.disposition.clone()?)))
            .collect()
    }

    /// The findings of `found` that are new: not known by their identity
    /// (one judged a wrong expectation is known, so found again it is not
    /// new).
    pub fn new_findings(&self, found: &[Finding], build: &str) -> Vec<Finding> {
        found
            .iter()
            .filter(|f| {
                self.findings
                    .iter()
                    .find(|k| k.identity == f.identity)
                    .is_none_or(|known| Knowledge::found_again(known, build))
            })
            .cloned()
            .collect()
    }

    /// Whether `known`, judged an unreliable reproduction, is found again in
    /// `build`, another build than the one it was judged on (or, for a
    /// judgment that does not say, found on): then it is new again.
    fn found_again(known: &Finding, build: &str) -> bool {
        known.disposition.as_ref().is_some_and(|d| {
            d.kind == DispositionKind::UnreliableReproduction
                && d.build.as_deref().unwrap_or(&known.build) != build
        })
    }

    /// The findings of `found` (in `build`) already adjudicated, with their
    /// dispositions: found again, they are not new (but for an unreliable
    /// reproduction found on another build than the one it was judged on,
    /// which is).
    pub fn already_adjudicated<'a>(
        &'a self,
        found: &'a [Finding],
        build: &str,
    ) -> Vec<(&'a Finding, &'a Disposition)> {
        found
            .iter()
            .filter(|f| {
                !self
                    .findings
                    .iter()
                    .any(|k| k.identity == f.identity && Knowledge::found_again(k, build))
            })
            .filter_map(|f| Some((f, self.disposition(&f.identity)?)))
            .collect()
    }

    /// The reproduced findings of `findings` (a cycle's) a lead may still be
    /// offered, each with its place there, which gives its id
    /// ([`finding_id`]): none whose disposition (its own, or the
    /// knowledge's) sets it aside.
    pub fn offered<'a>(&self, findings: &'a [Finding]) -> Vec<(usize, &'a Finding)> {
        findings
            .iter()
            .enumerate()
            .filter(|(_, f)| f.state == State::Reproduced)
            .filter(|(_, f)| {
                !f.disposition
                    .as_ref()
                    .or_else(|| self.disposition(&f.identity))
                    .is_some_and(Disposition::sets_aside)
            })
            .collect()
    }

    fn bound(&mut self) {
        if self.coverage.len() > COVERAGE {
            let mut keys: Vec<(u64, u64, String)> = self
                .coverage
                .iter()
                .map(|(k, c)| (c.last_run, c.count, k.clone()))
                .collect();
            keys.sort();
            for (_, _, key) in keys.into_iter().take(self.coverage.len() - COVERAGE) {
                self.coverage.remove(&key);
            }
        }
        if self.runs.len() > RUNS {
            self.runs.drain(..self.runs.len() - RUNS);
        }
        // Adjudicated findings go last (C-55), those set aside last of all:
        // their dispositions keep them from being proposed again.
        while self.findings.len() > FINDINGS {
            let closed = |f: &Finding| {
                f.state == State::NotReproduced
                    || (f.state == State::Fixed && f.checked_in.is_some())
            };
            let kind = |f: &Finding| f.disposition.as_ref().map(|d| d.kind);
            let order: [&dyn Fn(&Finding) -> bool; 5] = [
                &|f| kind(f).is_none() && closed(f),
                &|f| kind(f).is_none(),
                &|f| closed(f),
                &|f| kind(f) == Some(DispositionKind::Defect),
                &|f| kind(f) == Some(DispositionKind::AmbiguousRequirement),
            ];
            let at = order
                .iter()
                .find_map(|first| self.findings.iter().position(first))
                .unwrap_or(0);
            self.findings.remove(at);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::explore::{Plan, Run, Unwanted};
    use crate::findings::{Check, Failed};
    use serde_json::json;

    /// A finding about its own control (named in letters: an identity
    /// ignores numbers).
    fn finding(n: usize, state: State) -> Finding {
        let mut name = String::new();
        let mut rest = n;
        loop {
            name.push(char::from(b'a' + (rest % 26) as u8));
            rest /= 26;
            if rest == 0 {
                break;
            }
        }
        let mut f = Finding::new(
            Failed {
                check: Check::ReadableLabels,
                control: format!("control-{name}"),
                message: "a button in title has no readable label".into(),
                evidence: json!({}),
            },
            Vec::new(),
            "b1",
            "abc",
            "url-shortener",
        );
        f.state = state;
        f
    }

    fn run(covered: &[(&str, u32)], findings: Vec<Finding>) -> Run {
        Run {
            plan: Plan {
                goal: "Look at the history".into(),
                way: Way::Rules,
                seed: 7,
                steps: 10,
                seconds: 60,
                usd: 0.1,
                changes: Default::default(),
                start: "url-shortener".into(),
                conversation: false,
                turn_ms: 1000,
                stop_ms: 1000,
            },
            build: "b1".into(),
            commit: "abc".into(),
            started: "2026-10-04T10:00:00Z".into(),
            actions: 10,
            steps: Vec::new(),
            covered: covered.iter().map(|(k, n)| (k.to_string(), *n)).collect(),
            new_coverage: covered.iter().map(|(k, _)| k.to_string()).collect(),
            areas: Default::default(),
            findings,
            recoveries: Vec::new(),
            unwanted: Unwanted::default(),
            latencies: vec![0, 1, 2],
            usd: 0.0,
            unpriced: 0,
            assistant_usd: 0.0,
            notes: Vec::new(),
            conditions: Vec::new(),
            ended: String::new(),
            seconds: 3.0,
        }
    }

    #[test]
    fn knowledge_is_saved_read_and_a_later_format_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().join("objectives"));
        let project = dir.path().join("Agentique (main)");
        let path = Knowledge::file(&store, &project);
        let key = path
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert!(
            key.starts_with("agentique-main-") && key.len() == "agentique-main-".len() + 8,
            "{key}"
        );
        assert!(path.ends_with(format!("testing/{key}/knowledge.json")));
        // Another project of the same name keeps apart.
        let other = Knowledge::file(
            &store,
            &dir.path().join("elsewhere").join("Agentique (main)"),
        );
        assert_ne!(other, path);
        let mut k = Knowledge::load(&path, "agentique").unwrap();
        assert_eq!(k, Knowledge::new("agentique"));
        k.add_run(&run(
            &[("surface|history|History|click", 2)],
            vec![finding(1, State::Open)],
        ));
        k.save(&path).unwrap();
        let read = Knowledge::load(&path, "agentique").unwrap();
        assert_eq!(read, k);
        assert_eq!(read.count("surface|history|History|click"), 2);
        assert_eq!(
            read.uncovered(["surface|history|History|click", "surface|keys|escape|key"]),
            vec!["surface|keys|escape|key"]
        );
        let mut later = k.clone();
        later.format = FORMAT + 1;
        later.save(&path).unwrap();
        assert!(Knowledge::load(&path, "agentique").is_err());
    }

    #[test]
    fn findings_are_kept_once_and_a_fixed_one_found_again_fails_again() {
        let mut k = Knowledge::new("agentique");
        k.add_run(&run(&[], vec![finding(1, State::Open)]));
        k.add_run(&run(&[], vec![finding(1, State::Open)]));
        assert_eq!(k.findings.len(), 1);
        let identity = k.findings[0].identity.clone();
        assert!(k.fixed(&identity, "f1x", Some(12)));
        assert_eq!(k.to_replay("b2").len(), 1);
        k.replayed(&identity, "b2", &Replay::Passed);
        assert!(k.to_replay("b2").is_empty(), "replayed in b2 already");
        assert_eq!(k.to_replay("b3").len(), 1);
        k.replayed(
            &identity,
            "b3",
            &Replay::Failed {
                message: "x".into(),
            },
        );
        assert_eq!(k.findings[0].state, State::FailingAgain);
        // Fixed, then found again by exploration.
        k.fixed(&identity, "f2x", None);
        k.add_run(&run(&[], vec![finding(1, State::Open)]));
        assert_eq!(k.findings[0].state, State::FailingAgain);
        assert_eq!(k.ways()[&Way::Rules].runs, 3);
    }

    #[test]
    fn knowledge_stays_within_its_bounds() {
        let mut k = Knowledge::new("agentique");
        let keys: Vec<String> = (0..COVERAGE + 10).map(|i| format!("k{i}")).collect();
        let first: Vec<(&str, u32)> = keys[..10].iter().map(|k| (k.as_str(), 1)).collect();
        k.add_run(&run(&first, vec![]));
        let rest: Vec<(&str, u32)> = keys[10..].iter().map(|k| (k.as_str(), 1)).collect();
        k.add_run(&run(&rest, vec![]));
        assert_eq!(k.coverage.len(), COVERAGE);
        assert!(
            k.coverage.contains_key(&keys[COVERAGE + 9]),
            "the latest are kept"
        );
        assert!(!k.coverage.contains_key(&keys[0]), "the least recent go");
        for _ in 0..RUNS + 5 {
            k.add_run(&run(&[], vec![]));
        }
        assert_eq!(k.runs.len(), RUNS);
        assert_eq!(k.runs.last().unwrap().n, k.runs_made);
        for n in 0..FINDINGS + 3 {
            let state = if n == 5 {
                State::NotReproduced
            } else {
                State::Reproduced
            };
            k.update(&finding(n, state));
        }
        assert_eq!(k.findings.len(), FINDINGS);
        assert!(
            !k.findings.iter().any(|f| f.state == State::NotReproduced),
            "what proposes nothing goes first"
        );
    }

    #[test]
    fn what_became_of_a_finding_is_merged_with_its_fix_and_history() {
        let mut k = Knowledge::new("agentique");
        k.add_run(&run(&[], vec![finding(1, State::Open)]));
        let identity = k.findings[0].identity.clone();
        k.fixed(&identity, "f1x", Some(12));
        // A later run finds it again and reproduces it: failing again, and
        // the fix it failed after stays known.
        let mut again = finding(1, State::Open);
        again.set_state(State::Reproduced, "");
        again.replays.push(Replay::Failed {
            message: "x".into(),
        });
        k.update(&again);
        let known = &k.findings[0];
        assert_eq!(known.state, State::FailingAgain);
        assert_eq!(known.fixed_in.as_deref(), Some("f1x"));
        assert_eq!(known.pull_request, Some(12));
        let states: Vec<State> = known.history.iter().map(|c| c.state).collect();
        assert_eq!(states, vec![State::Open, State::Fixed, State::FailingAgain]);
        assert_eq!(known.replays.len(), 1);
        // A replay that does not reproduce leaves a fix as it was.
        k.fixed(&identity, "f2x", None);
        let mut flaky = finding(1, State::Open);
        flaky.set_state(State::NotReproduced, "replay 1: the check held");
        k.update(&flaky);
        assert_eq!(k.findings[0].state, State::Fixed);
        assert_eq!(k.findings[0].fixed_in.as_deref(), Some("f2x"));
    }

    fn disposed(kind: DispositionKind) -> Disposition {
        Disposition {
            kind,
            reason: "judged against the requirement".into(),
            requirement: Some("Shop::LabelsReadable".into()),
            objective: "objective-1".into(),
            cycle: 2,
            role: "lead".into(),
            at: "2026-10-09T10:00:00Z".into(),
            build: None,
        }
    }

    /// C-55: a disposition is kept in the knowledge across a reload (and so
    /// across objectives), a later one replaces it, and a file written
    /// before dispositions existed still loads, its findings unjudged.
    #[test]
    fn dispositions_survive_a_reload_and_older_knowledge_still_loads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("testing").join("p").join("knowledge.json");
        let mut reproduced = finding(1, State::Reproduced);
        reproduced.set_state(State::Reproduced, "");
        Knowledge::change(&path, "p", |k| {
            k.add_run(&run(&[("a|b|c|click", 1)], vec![reproduced.clone()]));
            k.adjudicate(&reproduced, disposed(DispositionKind::WrongExpectation));
        })
        .unwrap();
        let read = Knowledge::load(&path, "p").unwrap();
        let kept = read.disposition(&reproduced.identity).unwrap();
        assert_eq!(kept.kind, DispositionKind::WrongExpectation);
        assert_eq!(kept.requirement.as_deref(), Some("Shop::LabelsReadable"));
        assert_eq!((kept.cycle, kept.role.as_str()), (2, "lead"));
        assert_eq!(read.dispositions().len(), 1);
        // A later judgment replaces it; a finding the knowledge lost is kept.
        let other = finding(2, State::Reproduced);
        Knowledge::change(&path, "p", |k| {
            k.adjudicate(&reproduced, disposed(DispositionKind::Defect));
            k.adjudicate(&other, disposed(DispositionKind::AmbiguousRequirement));
        })
        .unwrap();
        let read = Knowledge::load(&path, "p").unwrap();
        assert_eq!(
            read.disposition(&reproduced.identity).unwrap().kind,
            DispositionKind::Defect
        );
        assert_eq!(read.findings.len(), 2);
        // An update of what became of it keeps its disposition.
        let mut again = read.clone();
        again.update(&finding(1, State::Reproduced));
        assert!(again.disposition(&reproduced.identity).is_some());
        // Written by the build before C-55: no `disposition` anywhere.
        let text = std::fs::read_to_string(&path).unwrap();
        let mut old: serde_json::Value = serde_json::from_str(&text).unwrap();
        for f in old["findings"].as_array_mut().unwrap() {
            f.as_object_mut().unwrap().remove("disposition");
        }
        assert!(!old.to_string().contains("disposition"));
        std::fs::write(&path, old.to_string()).unwrap();
        let read = Knowledge::load(&path, "p").unwrap();
        assert_eq!(read.findings.len(), 2);
        assert!(read.dispositions().is_empty());
        assert_eq!(read.format, FORMAT);
    }

    /// C-55: a finding judged a wrong expectation or an unreliable
    /// reproduction is not offered again; found again, it is not new but
    /// already adjudicated; a defect and an unjudged finding are offered.
    #[test]
    fn a_wrong_expectation_is_not_offered_again_nor_new_when_found_again() {
        let mut k = Knowledge::new("p");
        let mut findings: Vec<Finding> = (1..=4)
            .map(|n| {
                let mut f = finding(n, State::Open);
                f.set_state(State::Reproduced, "");
                f
            })
            .collect();
        k.add_run(&run(&[], findings.clone()));
        k.adjudicate(&findings[0], disposed(DispositionKind::WrongExpectation));
        k.adjudicate(
            &findings[1],
            disposed(DispositionKind::UnreliableReproduction),
        );
        // Its own copy says it is a defect: offered.
        findings[2].disposition = Some(disposed(DispositionKind::Defect));
        // Their ids are their places among the cycle's findings: f3 and f4,
        // whatever was set aside before them.
        let offered: Vec<(String, &str)> = k
            .offered(&findings)
            .into_iter()
            .map(|(i, f)| (finding_id(i), f.identity.as_str()))
            .collect();
        assert_eq!(
            offered,
            vec![
                ("f3".to_string(), findings[2].identity.as_str()),
                ("f4".to_string(), findings[3].identity.as_str())
            ]
        );
        // Found again by a later run: not new, already adjudicated.
        let again = vec![finding(1, State::Open), finding(9, State::Open)];
        let new = k.new_findings(&again, "b1");
        assert_eq!(new.len(), 1);
        assert_eq!(new[0].identity, again[1].identity);
        let adjudicated = k.already_adjudicated(&again, "b1");
        assert_eq!(adjudicated.len(), 1);
        assert_eq!(adjudicated[0].1.kind, DispositionKind::WrongExpectation);
        let before = k.findings.len();
        k.add_run(&Run {
            build: "b7".into(),
            ..run(&[], vec![finding(1, State::Open)])
        });
        assert_eq!(k.findings.len(), before, "not recorded as new");
        let known = &k.findings[0];
        assert_eq!(
            known.disposition.as_ref().unwrap().kind,
            DispositionKind::WrongExpectation
        );
        assert!(known.note.contains("already adjudicated"), "{}", known.note);
        assert!(known.note.contains("b7"), "{}", known.note);
    }

    /// C-55: an unreliable reproduction found again in the build it was
    /// judged on stays set aside; found on another build, it is new again
    /// and offered.
    #[test]
    fn an_unreliable_reproduction_found_on_another_build_is_offered_again() {
        let mut k = Knowledge::new("p");
        let mut flaky = finding(1, State::Open);
        flaky.set_state(State::Reproduced, "");
        k.add_run(&run(&[], vec![flaky.clone()]));
        k.adjudicate(&flaky, disposed(DispositionKind::UnreliableReproduction));
        let found = vec![finding(1, State::Open)];
        // The same build (`b1`): still set aside, not new.
        assert!(k.new_findings(&found, "b1").is_empty());
        assert_eq!(k.already_adjudicated(&found, "b1").len(), 1);
        assert!(k.offered(std::slice::from_ref(&flaky)).is_empty());
        // Another build: new, not already adjudicated, and once the run is
        // added, no longer set aside.
        assert_eq!(k.new_findings(&found, "b2").len(), 1);
        assert!(k.already_adjudicated(&found, "b2").is_empty());
        k.add_run(&Run {
            build: "b2".into(),
            ..run(&[], found.clone())
        });
        assert!(k.disposition(&flaky.identity).is_none());
        assert!(
            k.findings[0].note.contains("offered again"),
            "{}",
            k.findings[0].note
        );
        assert_eq!(k.offered(std::slice::from_ref(&flaky)).len(), 1);
        // Judged on another build than it was first found on (`b4`): the
        // build it was judged on counts.
        let mut later = Knowledge::new("p");
        let mut other = finding(2, State::Open);
        other.set_state(State::Reproduced, "");
        later.add_run(&run(&[], vec![other.clone()]));
        later.adjudicate(
            &other,
            Disposition {
                build: Some("b4".into()),
                ..disposed(DispositionKind::UnreliableReproduction)
            },
        );
        let again = vec![finding(2, State::Open)];
        assert!(later.new_findings(&again, "b4").is_empty(), "judged there");
        assert_eq!(later.new_findings(&again, "b1").len(), 1, "another build");
    }

    /// C-55: an older copy of a finding's disposition never replaces a
    /// newer one; when the knowledge is full of judged findings, a defect
    /// goes before what is set aside.
    #[test]
    fn a_newer_disposition_is_kept_and_judged_findings_are_bounded_in_order() {
        let mut k = Knowledge::new("p");
        let first = finding(1, State::Reproduced);
        let older = Disposition {
            at: "2026-10-01T00:00:00Z".into(),
            ..disposed(DispositionKind::Defect)
        };
        k.adjudicate(&first, disposed(DispositionKind::WrongExpectation));
        let mut stale = first.clone();
        stale.disposition = Some(older);
        k.update(&stale);
        assert_eq!(
            k.disposition(&first.identity).map(|d| d.kind),
            Some(DispositionKind::WrongExpectation)
        );
        let mut newer = first.clone();
        newer.disposition = Some(Disposition {
            at: "2026-10-12T00:00:00Z".into(),
            ..disposed(DispositionKind::Defect)
        });
        k.update(&newer);
        assert_eq!(
            k.disposition(&first.identity).map(|d| d.kind),
            Some(DispositionKind::Defect)
        );
        // Full of judged findings: the defects go first, oldest first.
        let mut full = Knowledge::new("p");
        for n in 0..FINDINGS {
            let kind = if n % 2 == 0 {
                DispositionKind::WrongExpectation
            } else {
                DispositionKind::Defect
            };
            full.adjudicate(&finding(n, State::Reproduced), disposed(kind));
        }
        let extra = finding(FINDINGS + 1, State::Reproduced);
        full.adjudicate(&extra, disposed(DispositionKind::WrongExpectation));
        assert_eq!(full.findings.len(), FINDINGS);
        assert_eq!(
            full.findings
                .iter()
                .filter(
                    |f| f.disposition.as_ref().unwrap().kind == DispositionKind::WrongExpectation
                )
                .count(),
            FINDINGS / 2 + 1,
            "what is set aside is kept"
        );
        assert!(
            full.disposition(&finding(1, State::Reproduced).identity)
                .is_none()
        );
    }

    #[test]
    fn writers_of_the_knowledge_take_turns_and_lose_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("testing").join("p").join("knowledge.json");
        let writers: Vec<_> = (0..4)
            .map(|n| {
                let path = path.clone();
                std::thread::spawn(move || {
                    for i in 0..5 {
                        let key = format!("k{n}-{i}");
                        Knowledge::change(&path, "p", |k| {
                            k.add_run(&run(&[(key.as_str(), 1)], vec![]))
                        })
                        .unwrap();
                    }
                })
            })
            .collect();
        for writer in writers {
            writer.join().unwrap();
        }
        let k = Knowledge::load(&path, "p").unwrap();
        assert_eq!(k.runs_made, 20);
        assert_eq!(k.coverage.len(), 20);
    }
}
