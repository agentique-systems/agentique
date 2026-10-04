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

use crate::decide::Way;
use crate::explore::Run;
use crate::findings::{Finding, Replay, State};
use crate::record::Store;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The file's format; a build that does not know it does not read it.
pub const FORMAT: u32 = 1;

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

    /// A project's name as a folder name: lowercase letters, digits and
    /// hyphens.
    pub fn key(project: &str) -> String {
        let mut key = String::new();
        for c in project.trim().chars().flat_map(char::to_lowercase) {
            if c.is_ascii_alphanumeric() {
                key.push(c);
            } else if !key.ends_with('-') {
                key.push('-');
            }
        }
        let key = key.trim_matches('-').to_string();
        if key.is_empty() {
            "project".into()
        } else {
            key
        }
    }

    /// Where a project's knowledge is kept: `testing/<project>/knowledge.json`
    /// in the app data folder that holds the objectives' records.
    pub fn file(store: &Store, project: &str) -> PathBuf {
        store
            .folder
            .parent()
            .unwrap_or(&store.folder)
            .join("testing")
            .join(Knowledge::key(project))
            .join("knowledge.json")
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
                    known.state = State::FailingAgain;
                    known.note = format!(
                        "fixed in {}, found again in build {}",
                        known.fixed_in.as_deref().unwrap_or("an earlier change"),
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
            seed: run.plan.seed,
            steps: run.chosen,
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

    /// Keeps a finding as it now is (after reproducing it), by identity.
    pub fn update(&mut self, finding: &Finding) {
        match self
            .findings
            .iter_mut()
            .find(|f| f.identity == finding.identity)
        {
            Some(known) => *known = finding.clone(),
            None => self.findings.push(finding.clone()),
        }
        self.bound();
    }

    /// A merged change fixed the finding `identity`.
    pub fn fixed(&mut self, identity: &str, commit: &str, pull_request: Option<u64>) -> bool {
        let Some(finding) = self.findings.iter_mut().find(|f| f.identity == identity) else {
            return false;
        };
        finding.state = State::Fixed;
        finding.fixed_in = Some(commit.to_string());
        finding.pull_request = pull_request;
        finding.checked_in = None;
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
                finding.state = State::FailingAgain;
                finding.note = format!(
                    "fixed in {}, fails again in build {build}",
                    finding.fixed_in.as_deref().unwrap_or("an earlier change")
                );
            }
            Replay::Diverged { .. } => {}
        }
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
        while self.findings.len() > FINDINGS {
            let closed = self.findings.iter().position(|f| {
                f.state == State::NotReproduced
                    || (f.state == State::Fixed && f.checked_in.is_some())
            });
            self.findings.remove(closed.unwrap_or(0));
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
            },
            build: "b1".into(),
            commit: "abc".into(),
            started: "2026-10-04T10:00:00Z".into(),
            chosen: 10,
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
            notes: Vec::new(),
            ended: String::new(),
            seconds: 3.0,
        }
    }

    #[test]
    fn knowledge_is_saved_read_and_a_later_format_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path().join("objectives"));
        let path = Knowledge::file(&store, "Agentique (main)");
        assert!(path.ends_with("testing/agentique-main/knowledge.json"));
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
}
