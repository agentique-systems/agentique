//! Evidence on the base and evaluation by behaviour (C-54, ROADMAP §4.16
//! "Evidence, gates and bounds", requirement `DefectShownBefore`): a cycle's
//! criteria run on its base before the change, where none may pass and one
//! must fail with evidence (the replay of its finding on the base build, an
//! observation whose own expectation fails there, or a test run that
//! compiles, runs a test and fails an assertion, with the change's new and
//! changed test files brought over); anything else there (a setup that
//! failed, a broken connection, a base that did not build, an error that is
//! no assertion) is "no evidence", with the reason. The test runs on the
//! base are made again whenever a later attempt's test files differ, so the
//! evidence is always made with the checked commit's own tests. After the
//! change, the replay must pass in a test instance of the change, started
//! from the base's start project (the change cannot make it pass by changing
//! its start data), and a change to user-facing code is explored briefly by
//! the rules in the areas it touched, where no new invariant may fail.

use super::{Driver, number_after, tests_ran};
use crate::control::Options;
use crate::explore::{self, Changes, Plan};
use crate::findings::{self, Check as Found, Finding, Replay};
use crate::knowledge::Knowledge;
use crate::record::{BaseBuild, Check, Criterion, Outcome, REPLAY, TestFile};
use agq_execution::process::{Finished, Program};
use agq_execution::{Executor, Scope};
use std::path::Path;
use std::time::Duration;

/// Output that says a test run did not compile or load: no evidence.
const NOT_BUILT: [&str; 10] = [
    "could not compile",
    "error[E",
    "error: could not find",
    "no test target named",
    "error: package ID specification",
    "SyntaxError",
    "ModuleNotFoundError",
    "ImportError",
    "Cannot find module",
    "ERR_MODULE_NOT_FOUND",
];

/// How many tests a run says failed: cargo's `test result:` lines, Node's
/// `# fail` / `ℹ fail`, Python's `FAILED (failures=…)` (its `errors=` are
/// exceptions, not assertions that failed).
pub fn tests_failed(output: &str) -> Option<usize> {
    let mut total = None;
    for line in output.lines() {
        let n = if line.contains("test result:") {
            number_after(line, "passed; ")
        } else if line.contains("# fail") || line.contains("ℹ fail") {
            number_after(line, "# fail").or_else(|| number_after(line, "ℹ fail"))
        } else if line.starts_with("FAILED (") {
            Some(number_after(line, "failures=").unwrap_or(0))
        } else {
            None
        };
        if let Some(n) = n {
            total = Some(total.unwrap_or(0) + n);
        }
    }
    total
}

/// Whether a run's output shows an assertion that failed: cargo's
/// `assertion failed` and `assertion `left == right` failed`, Node's and
/// Python's `AssertionError` (`ERR_ASSERTION`). A test that ends with
/// another error (a `TypeError`, an `unwrap` on nothing) shows none.
fn asserted(output: &str) -> bool {
    output.to_lowercase().contains("assertion")
}

/// A command criterion's run on the base, judged as evidence: `failed`
/// only when it compiled, ran at least one test and an assertion failed;
/// `passed` when it passed there (the gate fails); otherwise `no evidence`
/// with the reason.
pub fn on_base_verdict(run: Result<Finished, String>) -> (&'static str, String) {
    let finished = match run {
        Ok(finished) => finished,
        Err(error) => return ("no evidence", format!("it did not run: {error}")),
    };
    if finished.timed_out || finished.cancelled {
        return (
            "no evidence",
            format!("it did not finish: {}", finished.summary()),
        );
    }
    let output = format!("{}\n{}", finished.stdout, finished.stderr);
    if let Some(line) = output
        .lines()
        .find(|l| NOT_BUILT.iter().any(|marker| l.contains(marker)))
    {
        return (
            "no evidence",
            format!("it did not compile or load on the base: {}", line.trim()),
        );
    }
    let failed = tests_failed(&output).unwrap_or(0);
    let ran = tests_ran(&output).unwrap_or(0).max(failed);
    if failed > 0 && asserted(&output) {
        return (
            "failed",
            format!(
                "{failed} of its tests failed an assertion on the base: {}",
                agq_execution::process::last_lines(&output, 12)
            ),
        );
    }
    if failed > 0 {
        return (
            "no evidence",
            format!(
                "its tests failed on the base with an error, not an assertion: {}",
                agq_execution::process::last_lines(&output, 8)
            ),
        );
    }
    if ran == 0 {
        return ("no evidence", "it ran no test on the base".into());
    }
    if finished.success {
        ("passed", format!("{ran} test(s) passed on the base"))
    } else {
        (
            "no evidence",
            format!(
                "it failed without a failing test: {}",
                agq_execution::process::last_lines(&output, 8)
            ),
        )
    }
}

/// An observation criterion's outcome, and whether its own expectation
/// decided it (not a setup action that failed, a broken connection, a way
/// that could not be cleared or an instance that did not start). Outcomes
/// that are no criterion's (a dialog open at the start) are never its own.
pub(super) struct Observed {
    pub outcome: Outcome,
    pub own: bool,
}

/// An observation's outcome on the base, as evidence: its own expectation
/// failing there is evidence, holding there passes (the gate fails), and
/// anything else is no evidence, with the reason.
pub fn observed_on_base(observed: &Outcome, own: bool, build: &str) -> Outcome {
    let (verdict, detail) = match (own, observed.verdict.as_str()) {
        (true, "failed") => (
            "failed",
            format!(
                "its expectation fails on the base build {build}: {}",
                observed.detail
            ),
        ),
        (true, "passed") => ("passed", format!("it holds on the base build {build}")),
        _ => (
            "no evidence",
            format!(
                "not its expectation on the base build {build}: {}",
                observed.detail
            ),
        ),
    };
    Outcome::new(observed.name.clone(), verdict, detail)
}

impl Driver {
    /// Runs `words` in `folder` as an exact allowed command, in the shared
    /// build folder, stopping with the objective.
    pub(super) fn execute(
        &self,
        folder: &Path,
        words: &[String],
        timeout: Duration,
    ) -> Result<Finished, String> {
        let program = Program::from_list(words).ok_or("an empty command")?;
        let executor = Executor::new(Scope::read_only(folder).map_err(|e| e.to_string())?)
            .trusted(true)
            .target_dir(self.target())
            .allow(vec![program.clone()])
            .cancel_flag(self.controls.stop.clone());
        executor
            .run(&program, "", timeout)
            .map_err(|e| e.to_string())
    }

    /// The build of the cycle's base: the running build when it is of the
    /// base commit (the adopted build, from the builds folder), otherwise a
    /// debug build of the base checkout, whose executable is kept with the
    /// cycle (the shared build folder is rebuilt for other commits).
    pub(super) fn base_build(&mut self) -> Result<BaseBuild, String> {
        let base = self.cycle().base.clone().ok_or("the cycle has no base")?;
        if let Some(built) = self.cycle().base_build.clone()
            && built.commit == base
            && built.exe.exists()
        {
            return Ok(built);
        }
        let running = self.setup.running_build.clone().and_then(|id| {
            let folder = self.setup.builds.join(&id);
            let manifest = agq_launcher::Manifest::load(&folder).ok()?;
            (manifest.commit == base).then(|| BaseBuild {
                build: id,
                exe: folder.join(agq_launcher::STUDIO),
                commit: base.clone(),
            })
        });
        let built = match running {
            Some(built) => built,
            None => {
                let checkout = self.checkout("base", &base)?;
                self.event(format!(
                    "Building the base {} for its test instances",
                    crate::builds::short(&base)
                ));
                let exe = self.setup.studios.build(
                    &checkout,
                    &self.target(),
                    &self.setup.builds,
                    self.controls.stop.clone(),
                )?;
                // Kept: the shared build folder is rebuilt for the change.
                let kept = if exe.is_file() {
                    let folder = self.folder("base-studio");
                    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
                    let copy = folder.join(exe.file_name().unwrap_or_default());
                    std::fs::copy(&exe, &copy).map_err(|e| format!("{}: {e}", copy.display()))?;
                    copy
                } else {
                    exe
                };
                BaseBuild {
                    build: format!("debug-{}", crate::builds::short(&base)),
                    exe: kept,
                    commit: base.clone(),
                }
            }
        };
        self.cycle_mut().base_build = Some(built.clone());
        self.save();
        Ok(built)
    }

    /// The test files `commit` changes against `base`, each with its blob
    /// id (`deleted` for one it deletes), in order: what its test runs on
    /// the base are made with.
    pub(super) fn test_files(&self, base: &str, commit: &str) -> Result<Vec<TestFile>, String> {
        let repository = &self.objective.repository;
        let patch =
            agq_execution::git::patch_of(repository, base, commit).map_err(|e| e.to_string())?;
        let mut tests = Vec::new();
        let mut present = Vec::new();
        for file in patch
            .files
            .iter()
            .filter(|f| crate::gates::test_path(&f.path))
        {
            let path = file.path.replace('\\', "/");
            if file.status == "deleted" {
                tests.push(TestFile {
                    path,
                    blob: "deleted".into(),
                });
            } else {
                present.push(path);
            }
        }
        if !present.is_empty() {
            let mut words = vec!["git", "ls-tree", commit, "--"];
            words.extend(present.iter().map(String::as_str));
            let listed = crate::forge::run(repository, &words, Duration::from_secs(60))?;
            for line in listed.stdout.lines() {
                // `<mode> blob <id>\t<path>`
                if let Some((meta, path)) = line.split_once('\t')
                    && let Some(blob) = meta.split_whitespace().nth(2)
                {
                    tests.push(TestFile {
                        path: path.to_string(),
                        blob: blob.to_string(),
                    });
                }
            }
        }
        tests.sort();
        Ok(tests)
    }

    /// Each criterion on the base, before the change: the replay of its
    /// finding, its test runs with `tests` (the commit's test files) brought
    /// over, its observations in a test instance of the base build; a
    /// judgment is no evidence. A base that does not build is no evidence.
    pub(super) fn on_base(
        &mut self,
        commit: &str,
        tests: &[TestFile],
    ) -> Result<Vec<Outcome>, String> {
        let proposal = self.cycle().proposal.clone().ok_or("no proposal")?;
        let base = self.cycle().base.clone().ok_or("the cycle has no base")?;
        let mut outcomes = Vec::new();
        if let Some(finding) = self.cycle().replay.clone() {
            outcomes.push(self.replay_on_base(&finding)?);
        }
        outcomes.extend(self.commands_on_base(commit, tests)?);
        let observed: Vec<&Criterion> = proposal
            .criteria
            .iter()
            .filter(|c| matches!(c.check, Check::Observation { .. }))
            .collect();
        if !observed.is_empty() {
            match self.base_build() {
                Err(error) => {
                    for criterion in &observed {
                        outcomes.push(Outcome::new(
                            criterion.id.clone(),
                            "no evidence",
                            format!("the base could not be built: {error}"),
                        ));
                    }
                }
                Ok(built) => {
                    let source = self.checkout("base", &base)?;
                    let ids: Vec<&str> = observed.iter().map(|c| c.id.as_str()).collect();
                    let template = Options {
                        speed: Some("instant".into()),
                        ..Options::default()
                    };
                    for seen in
                        self.observe_criteria(&built.exe, &source, &observed, "base", &template)
                    {
                        // Only the criteria's own outcomes: a dialog open at
                        // the start is no criterion's.
                        if ids.contains(&seen.outcome.name.as_str()) {
                            outcomes.push(observed_on_base(&seen.outcome, seen.own, &built.build));
                        }
                    }
                }
            }
        }
        for criterion in proposal
            .criteria
            .iter()
            .filter(|c| matches!(c.check, Check::Judgment))
        {
            outcomes.push(Outcome::new(
                criterion.id.clone(),
                "no evidence",
                "a judgment shows nothing on the base",
            ));
        }
        Ok(outcomes)
    }

    /// The command criteria on the base, with `tests` (the checked commit's
    /// test files) brought over into a checkout of the base of their own.
    pub(super) fn commands_on_base(
        &mut self,
        commit: &str,
        tests: &[TestFile],
    ) -> Result<Vec<Outcome>, String> {
        let proposal = self.cycle().proposal.clone().ok_or("no proposal")?;
        let base = self.cycle().base.clone().ok_or("the cycle has no base")?;
        let commands: Vec<&Criterion> = proposal
            .criteria
            .iter()
            .filter(|c| matches!(c.check, Check::Command { .. }))
            .collect();
        let mut outcomes = Vec::new();
        if commands.is_empty() {
            return Ok(outcomes);
        }
        // Its own checkout: the base's stays as it is, for test instances.
        let folder = self.checkout("base-tests", &base)?;
        let brought = self.bring_tests(&folder, commit, tests)?;
        for criterion in commands {
            let Check::Command { program } = &criterion.check else {
                continue;
            };
            let (verdict, detail) = match crate::roles::test_command(program) {
                Err(problem) => ("no evidence", problem),
                Ok(()) => {
                    on_base_verdict(self.execute(&folder, program, Duration::from_secs(1800)))
                }
            };
            let detail = if brought.is_empty() {
                detail
            } else {
                format!(
                    "{detail} (with {} of {} brought over)",
                    brought.join(", "),
                    crate::builds::short(commit)
                )
            };
            outcomes.push(Outcome::new(criterion.id.clone(), verdict, detail));
            if self.controls.stopped() {
                return Err("stopped".into());
            }
        }
        Ok(outcomes)
    }

    /// `tests` (those `commit` has, not those it deletes) written into the
    /// base's checkout `folder`; returns their paths.
    fn bring_tests(
        &self,
        folder: &Path,
        commit: &str,
        tests: &[TestFile],
    ) -> Result<Vec<String>, String> {
        let repository = &self.objective.repository;
        let mut brought = Vec::new();
        for test in tests.iter().filter(|t| t.blob != "deleted") {
            let shown = crate::forge::run(
                repository,
                &["git", "show", &format!("{commit}:{}", test.path)],
                Duration::from_secs(60),
            )?;
            let to = folder.join(&test.path);
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(&to, shown.stdout).map_err(|e| format!("{}: {e}", to.display()))?;
            brought.push(test.path.clone());
        }
        Ok(brought)
    }

    /// The replay of the chosen finding on the base build: its reproduction
    /// is reused when it was made on that build (the finding's own build is
    /// the base build, and two replays failed the same way); otherwise it
    /// runs again there, from the base's start project. A base that does
    /// not build is no evidence.
    fn replay_on_base(&mut self, finding: &Finding) -> Result<Outcome, String> {
        let built = match self.base_build() {
            Ok(built) => built,
            Err(error) => {
                return Ok(Outcome::new(
                    REPLAY,
                    "no evidence",
                    format!("the base could not be built: {error}"),
                ));
            }
        };
        let reproduced_here = self.cycle().findings.iter().any(|f| {
            f.identity == finding.identity
                && f.build == built.build
                && f.state == findings::State::Reproduced
                && f.replays.iter().filter(|r| r.failed()).count() >= 2
        });
        if reproduced_here {
            return Ok(Outcome::new(
                REPLAY,
                "failed",
                format!(
                    "it failed the same way twice on the base build {} when it was reproduced: {}",
                    built.build, finding.message
                ),
            ));
        }
        let base = self.cycle().base.clone().ok_or("the cycle has no base")?;
        let source = self.checkout("base", &base)?;
        let replay = self.replay_in(&built.exe, &source, finding, Options::default());
        Ok(match replay {
            Replay::Failed { message } => Outcome::new(
                REPLAY,
                "failed",
                format!("it fails on the base build {}: {message}", built.build),
            ),
            Replay::Passed => Outcome::new(
                REPLAY,
                "passed",
                format!("it holds on the base build {}", built.build),
            ),
            Replay::Diverged { at, reason } => Outcome::new(
                REPLAY,
                "no evidence",
                format!("it could not be replayed on the base (step {at}): {reason}"),
            ),
        })
    }

    /// A finding replayed in a fresh test instance of `exe`, started from
    /// its start state in `source` (the base's checkout).
    fn replay_in(
        &mut self,
        exe: &Path,
        source: &Path,
        finding: &Finding,
        options: Options,
    ) -> Replay {
        let mut instance = self.setup.studios.instance(
            exe,
            &source.join(&finding.start),
            &self.folder("replay"),
            options,
        );
        let controls = self.controls.clone();
        findings::replay(instance.as_mut(), finding, &mut || controls.stopped())
    }

    /// Observation criteria in test instances of `exe` (the project is
    /// `source`), started as `template` says: those without a condition
    /// share one instance, each with a condition has its own. Each outcome
    /// says whether its own expectation decided it.
    pub(super) fn observe_criteria(
        &mut self,
        exe: &Path,
        source: &Path,
        criteria: &[&Criterion],
        name: &str,
        template: &Options,
    ) -> Vec<Observed> {
        let mut outcomes = Vec::new();
        let mut groups: Vec<(Option<String>, Vec<&Criterion>)> = Vec::new();
        for criterion in criteria {
            let Check::Observation { condition, .. } = &criterion.check else {
                continue;
            };
            match groups
                .iter_mut()
                .find(|(c, _)| c == condition && c.is_none())
            {
                Some((_, group)) => group.push(criterion),
                None => groups.push((condition.clone(), vec![criterion])),
            }
        }
        for (i, (condition, group)) in groups.into_iter().enumerate() {
            let options = Options {
                condition: condition.clone(),
                ..template.clone()
            };
            let folder = self.folder(&format!("{name}-instance-{i}"));
            let started =
                crate::control::TestInstance::start_with(exe, &folder, source, source, &options)
                    .and_then(|mut instance| {
                        let client = instance.connect(Duration::from_secs(180))?;
                        Ok((instance, client))
                    });
            let (instance, mut client) = match started {
                Ok(started) => started,
                Err(problem) => {
                    for criterion in group {
                        outcomes.push(Observed {
                            outcome: Outcome::new(
                                criterion.id.clone(),
                                "not run",
                                format!("its test instance did not start: {problem}"),
                            ),
                            own: false,
                        });
                    }
                    continue;
                }
            };
            if let Some(dialog) = Self::dialog_at_start(&mut client) {
                outcomes.push(Observed {
                    outcome: Outcome::new(
                        format!(
                            "no dialog when it starts{}",
                            match &condition {
                                Some(c) => format!(" ({c})"),
                                None => String::new(),
                            }
                        ),
                        "failed",
                        format!("the {dialog} dialog is open when the test instance starts"),
                    ),
                    own: false,
                });
            }
            for criterion in group {
                if let Check::Observation { setup, expect, .. } = &criterion.check {
                    let (mut outcome, own) =
                        self.observe_criterion(&mut client, criterion, setup, expect);
                    if let Some(condition) = &condition {
                        outcome.detail = format!("{} (started {condition})", outcome.detail);
                    }
                    outcomes.push(Observed { outcome, own });
                }
            }
            drop(client);
            drop(instance);
        }
        outcomes
    }

    /// After the change, in test instances of its (unreviewed) build: the
    /// replay of its finding must pass, started from `start` (the base's
    /// checkout, whose start projects the change cannot alter), and a change
    /// to user-facing code is explored by the rules in the areas it touched,
    /// from the same start, where no invariant that held before may fail.
    pub(super) fn evaluate_by_behaviour(
        &mut self,
        exe: &Path,
        start: &Path,
        user_facing: &[String],
    ) -> Vec<Outcome> {
        let mut outcomes = Vec::new();
        let unreviewed = Options {
            speed: Some("instant".into()),
            stand_in: true,
            unreviewed: true,
            ..Options::default()
        };
        if let Some(finding) = self.cycle().replay.clone() {
            let outcome = match self.replay_in(exe, start, &finding, unreviewed.clone()) {
                Replay::Passed => Outcome::new(REPLAY, "passed", "it holds on the change"),
                Replay::Failed { message } => {
                    Outcome::new(REPLAY, "failed", format!("it still fails: {message}"))
                }
                Replay::Diverged { at, reason } => Outcome::new(
                    REPLAY,
                    "not run",
                    format!("it could not be replayed on the change (step {at}): {reason}"),
                ),
            };
            outcomes.push(outcome);
        }
        if !user_facing.is_empty() {
            outcomes.push(self.explore_changed(exe, start, user_facing, unreviewed));
        }
        outcomes
    }

    /// A short exploration by the rules of the areas the change touched: a
    /// failed invariant that the testing knowledge did not hold before is a
    /// failure (an expectation is never one here: the rules state none).
    fn explore_changed(
        &mut self,
        exe: &Path,
        start: &Path,
        changed: &[String],
        options: Options,
    ) -> Outcome {
        let changes = Changes {
            paths: changed.to_vec(),
            subjects: self
                .cycle()
                .proposal
                .as_ref()
                .map(|p| vec![p.title.clone()])
                .unwrap_or_default(),
        };
        let areas: Vec<String> = changes.areas().into_iter().collect();
        let plan = Plan {
            goal: format!(
                "Explore the areas the change touched: {}",
                if areas.is_empty() {
                    "the Studio".to_string()
                } else {
                    areas.join(", ")
                }
            ),
            way: crate::decide::Way::Rules,
            seed: self.cycle().n as u64,
            steps: CHANGED_STEPS,
            seconds: 600,
            // The rules spend nothing; a run stops once its spend reaches
            // its budget, so the budget is above nothing.
            usd: RULES_USD,
            changes,
            start: super::explore::STARTS[0].to_string(),
            conversation: false,
            turn_ms: findings::TURN_BUDGET_MS,
            stop_ms: findings::STOP_BUDGET_MS,
        };
        let repository = self.objective.repository.clone();
        let knowledge = Knowledge::load(
            &Knowledge::file(&self.setup.store, &repository),
            &Knowledge::key(&repository),
        )
        .unwrap_or_else(|_| Knowledge::new("unread"));
        let mut instance = self.setup.studios.instance(
            exe,
            &start.join(super::explore::STARTS[0]),
            &self.folder("changed"),
            options,
        );
        let mut watch = super::Watch {
            controls: self.controls.clone(),
        };
        let answers = crate::decide::Decider::default();
        let run = explore::explore(
            instance.as_mut(),
            &plan,
            &super::by_rules(&answers),
            &knowledge,
            &mut watch,
        );
        drop(instance);
        if run.actions == 0 {
            return Outcome::new(
                CHANGED_AREAS,
                "not run",
                format!("it did not explore: {}", run.ended),
            );
        }
        let new: Vec<String> = run
            .findings
            .iter()
            .filter(|f| f.check != Found::Expectation)
            .filter(|f| knowledge.findings.iter().all(|k| k.identity != f.identity))
            .map(super::explore::finding_line)
            .collect();
        if new.is_empty() {
            Outcome::new(
                CHANGED_AREAS,
                "passed",
                format!(
                    "{} steps by the rules in {}; no invariant failed that held before",
                    run.actions,
                    run.areas.iter().cloned().collect::<Vec<_>>().join(", ")
                ),
            )
        } else {
            Outcome::new(CHANGED_AREAS, "failed", new.join("\n"))
        }
    }
}

/// The outcome of the rules' exploration of a change's areas.
pub(super) const CHANGED_AREAS: &str = "no new invariant failure in the changed areas";

/// The steps of the rules' exploration of a change's areas.
const CHANGED_STEPS: u32 = 10;

/// The spend budget of an exploration by the rules (which ask no model):
/// above nothing, since a run stops when its spend reaches its budget.
const RULES_USD: f64 = 0.01;

#[cfg(test)]
mod tests {
    use super::*;

    fn finished(success: bool, stdout: &str) -> Result<Finished, String> {
        Ok(Finished {
            code: Some(if success { 0 } else { 101 }),
            success,
            stdout: stdout.into(),
            stderr: String::new(),
            timed_out: false,
            cancelled: false,
            duration: Duration::ZERO,
        })
    }

    /// `DefectShownBefore`: a run on the base is evidence only if it
    /// compiled, ran a test and an assertion failed.
    #[test]
    fn a_run_on_the_base_is_evidence_only_when_a_test_ran_and_failed() {
        let failed = on_base_verdict(finished(
            false,
            "running 2 tests\ntest a ... FAILED\nthread 'a' panicked at src/lib.rs:3:5:\nassertion `left == right` failed\ntest result: FAILED. 1 passed; 1 failed; 0 ignored",
        ));
        assert_eq!(failed.0, "failed", "{}", failed.1);
        let compile = on_base_verdict(finished(
            false,
            "error[E0425]: cannot find function `fixed` in this scope\nerror: could not compile `agq-x`",
        ));
        assert_eq!(compile.0, "no evidence");
        assert!(compile.1.contains("did not compile"));
        let none = on_base_verdict(finished(
            true,
            "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out",
        ));
        assert_eq!(
            none,
            ("no evidence", "it ran no test on the base".to_string())
        );
        let passed = on_base_verdict(finished(true, "test result: ok. 3 passed; 0 failed"));
        assert_eq!(passed.0, "passed");
        let timeout = on_base_verdict(Err("timed out after 1800 s".into()));
        assert_eq!(timeout.0, "no evidence");
        let node = on_base_verdict(finished(
            false,
            "not ok 1 - x\n  code: 'ERR_ASSERTION'\n  name: 'AssertionError'\n# tests 2\n# pass 1\n# fail 1",
        ));
        assert_eq!(node.0, "failed");
        let python = on_base_verdict(finished(
            false,
            "AssertionError: 1 != 2\nRan 3 tests in 0.1s\n\nFAILED (failures=1)",
        ));
        assert_eq!(python.0, "failed");
        let missing = on_base_verdict(finished(
            false,
            "Error: Cannot find module './note.mjs'\n# tests 0\n# fail 0",
        ));
        assert_eq!(missing.0, "no evidence");
        let crashed = on_base_verdict(finished(
            false,
            "running 1 test\nerror: test failed, to rerun",
        ));
        assert_eq!(crashed.0, "no evidence");
        // An error that is no assertion shows nothing.
        let type_error = on_base_verdict(finished(
            false,
            "not ok 1 - x\n  error: 'fixed is not a function'\n  name: 'TypeError'\n# tests 1\n# pass 0\n# fail 1",
        ));
        assert_eq!(type_error.0, "no evidence", "{}", type_error.1);
        let python_error = on_base_verdict(finished(
            false,
            "NameError: name 'fixed' is not defined\nRan 1 test in 0.1s\n\nFAILED (errors=1)",
        ));
        assert_eq!(python_error.0, "no evidence");
        let unwrap = on_base_verdict(finished(
            false,
            "thread 'a' panicked at src/lib.rs:3:5:\ncalled `Option::unwrap()` on a `None` value\ntest result: FAILED. 0 passed; 1 failed",
        ));
        assert_eq!(unwrap.0, "no evidence");
    }

    /// An observation on the base is evidence only when its own expectation
    /// failed there: a setup action refused, a broken connection or an
    /// instance that did not start is no evidence.
    #[test]
    fn only_an_observations_own_expectation_failing_on_the_base_is_evidence() {
        let own = Outcome::new("c2", "failed", "no control on screen says “Archive”");
        assert_eq!(observed_on_base(&own, true, "b1").verdict, "failed");
        let held = Outcome::new("c2", "passed", "observed");
        assert_eq!(observed_on_base(&held, true, "b1").verdict, "passed");
        for (verdict, detail) in [
            (
                "failed",
                "setup action failed: {\"ok\":false,\"kind\":\"gone\"}",
            ),
            ("failed", "the Studio is gone: connection reset"),
            ("not run", "its test instance did not start: it ended"),
        ] {
            let broken = observed_on_base(&Outcome::new("c2", verdict, detail), false, "b1");
            assert_eq!(broken.verdict, "no evidence", "{detail}");
            assert!(broken.detail.contains(detail));
        }
    }
}
