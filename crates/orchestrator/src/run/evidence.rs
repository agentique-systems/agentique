//! Evidence on the base and evaluation by behaviour (C-54, ROADMAP §4.16
//! "Evidence, gates and bounds", requirement `DefectShownBefore`): a cycle's
//! criteria run on its base before the change, where none may pass and one
//! must fail with evidence (the replay of its finding on the base build, an
//! observation that fails there, or a test run that compiles, runs a test
//! and fails, with the change's new and changed test files brought over);
//! anything else there is "no evidence", with the reason. After the change,
//! the replay must pass in a test instance of the change, and a change to
//! user-facing code is explored briefly by the rules in the areas it
//! touched, where no new invariant may fail.

use super::{Driver, tests_ran};
use crate::control::Options;
use crate::explore::{self, Changes, Plan};
use crate::findings::{self, Check as Found, Finding, Replay};
use crate::knowledge::Knowledge;
use crate::record::{BaseBuild, Check, Criterion, Outcome, REPLAY};
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
/// `# fail` / `ℹ fail`, Python's `FAILED (failures=…, errors=…)`.
pub fn tests_failed(output: &str) -> Option<usize> {
    let number_after = |line: &str, marker: &str| -> Option<usize> {
        let rest = &line[line.find(marker)? + marker.len()..];
        rest.trim_start()
            .split(|c: char| !c.is_ascii_digit())
            .next()?
            .parse()
            .ok()
    };
    let mut total = None;
    for line in output.lines() {
        let n = if line.contains("test result:") {
            number_after(line, "passed; ")
        } else if line.contains("# fail") || line.contains("ℹ fail") {
            number_after(line, "# fail").or_else(|| number_after(line, "ℹ fail"))
        } else if line.starts_with("FAILED (") {
            Some(
                number_after(line, "failures=").unwrap_or(0)
                    + number_after(line, "errors=").unwrap_or(0),
            )
        } else {
            None
        };
        if let Some(n) = n {
            total = Some(total.unwrap_or(0) + n);
        }
    }
    total
}

/// A command criterion's run on the base, judged as evidence: `failed`
/// only when it compiled, ran at least one test and a test failed;
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
    if failed > 0 {
        return (
            "failed",
            format!(
                "{failed} of its tests failed on the base: {}",
                agq_execution::process::last_lines(&output, 12)
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

    /// Each criterion on the base, before the change (once per cycle): the
    /// replay of its finding, its test runs with the change's test files
    /// brought over, its observations in a test instance of the base build;
    /// a judgment is no evidence.
    pub(super) fn on_base(&mut self, commit: &str) -> Result<Vec<Outcome>, String> {
        let proposal = self.cycle().proposal.clone().ok_or("no proposal")?;
        let base = self.cycle().base.clone().ok_or("the cycle has no base")?;
        let mut outcomes = Vec::new();
        if let Some(finding) = self.cycle().replay.clone() {
            outcomes.push(self.replay_on_base(&finding)?);
        }
        let commands: Vec<&Criterion> = proposal
            .criteria
            .iter()
            .filter(|c| matches!(c.check, Check::Command { .. }))
            .collect();
        if !commands.is_empty() {
            let folder = self.checkout("base", &base)?;
            let brought = self.bring_tests(&folder, &base, commit)?;
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
                    format!("{detail} (with {} brought over)", brought.join(", "))
                };
                outcomes.push(Outcome::new(criterion.id.clone(), verdict, detail));
                if self.controls.stopped() {
                    return Err("stopped".into());
                }
            }
        }
        let observed: Vec<&Criterion> = proposal
            .criteria
            .iter()
            .filter(|c| matches!(c.check, Check::Observation { .. }))
            .collect();
        if !observed.is_empty() {
            let built = self.base_build()?;
            let source = self.checkout("base", &base)?;
            for outcome in self.observe_criteria(&built.exe, &source, &observed, "base", false)? {
                let (verdict, detail) = match outcome.verdict.as_str() {
                    "failed" => (
                        "failed",
                        format!(
                            "it fails on the base build {}: {}",
                            built.build, outcome.detail
                        ),
                    ),
                    "passed" => (
                        "passed",
                        format!("it holds on the base build {}", built.build),
                    ),
                    _ => ("no evidence", outcome.detail.clone()),
                };
                outcomes.push(Outcome::new(outcome.name, verdict, detail));
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

    /// The change's new and changed test files, written into the base's
    /// checkout `folder`, so its test runs show the defect there if they
    /// can; returns their paths.
    fn bring_tests(&self, folder: &Path, base: &str, commit: &str) -> Result<Vec<String>, String> {
        let repository = &self.objective.repository;
        let patch =
            agq_execution::git::patch_of(repository, base, commit).map_err(|e| e.to_string())?;
        let mut brought = Vec::new();
        for file in patch
            .files
            .iter()
            .filter(|f| f.status != "deleted" && crate::gates::test_path(&f.path))
        {
            let path = file.path.replace('\\', "/");
            let shown = crate::forge::run(
                repository,
                &["git", "show", &format!("{commit}:{path}")],
                Duration::from_secs(60),
            )?;
            let to = folder.join(&path);
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(&to, shown.stdout).map_err(|e| format!("{}: {e}", to.display()))?;
            brought.push(path);
        }
        Ok(brought)
    }

    /// The replay of the chosen finding on the base build: its reproduction
    /// in this cycle is reused (it ran on the same build); otherwise it runs
    /// again there.
    fn replay_on_base(&mut self, finding: &Finding) -> Result<Outcome, String> {
        let built = self.base_build()?;
        let reproduced_here = self.cycle().findings.iter().any(|f| {
            f.identity == finding.identity
                && f.state == findings::State::Reproduced
                && f.replays.iter().filter(|r| r.failed()).count() >= 2
        }) && self
            .cycle()
            .explorations
            .iter()
            .any(|e| e.build == built.build);
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
    /// its start state in `source` (a checkout).
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
    /// `source`): those without a condition share one instance, each with a
    /// condition has its own. `stand_in` puts an unreviewed build's
    /// Assistant on the scripted stand-in.
    pub(super) fn observe_criteria(
        &mut self,
        exe: &Path,
        source: &Path,
        criteria: &[&Criterion],
        name: &str,
        stand_in: bool,
    ) -> Result<Vec<Outcome>, String> {
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
                speed: Some("instant".into()),
                stand_in,
                key: None,
                condition: condition.clone(),
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
                        outcomes.push(Outcome::new(
                            criterion.id.clone(),
                            "not run",
                            format!("its test instance did not start: {problem}"),
                        ));
                    }
                    continue;
                }
            };
            if let Some(dialog) = Self::dialog_at_start(&mut client) {
                outcomes.push(Outcome::new(
                    format!(
                        "no dialog when it starts{}",
                        match &condition {
                            Some(c) => format!(" ({c})"),
                            None => String::new(),
                        }
                    ),
                    "failed",
                    format!("the {dialog} dialog is open when the test instance starts"),
                ));
            }
            for criterion in group {
                if let Check::Observation { setup, expect, .. } = &criterion.check {
                    let mut outcome = self.observe_criterion(&mut client, criterion, setup, expect);
                    if let Some(condition) = &condition {
                        outcome.detail = format!("{} (started {condition})", outcome.detail);
                    }
                    outcomes.push(outcome);
                }
            }
            drop(client);
            drop(instance);
        }
        Ok(outcomes)
    }

    /// After the change, in test instances of its build: the replay of its
    /// finding must pass, and a change to user-facing code is explored by
    /// the rules in the areas it touched, where no invariant that held
    /// before may fail.
    pub(super) fn evaluate_by_behaviour(
        &mut self,
        exe: &Path,
        verify: &Path,
        user_facing: &[String],
    ) -> Vec<Outcome> {
        let mut outcomes = Vec::new();
        let stand_in = Options {
            speed: Some("instant".into()),
            stand_in: true,
            key: None,
            condition: None,
        };
        if let Some(finding) = self.cycle().replay.clone() {
            let outcome = match self.replay_in(exe, verify, &finding, stand_in.clone()) {
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
            outcomes.push(self.explore_changed(exe, verify, user_facing, stand_in));
        }
        outcomes
    }

    /// A short exploration by the rules of the areas the change touched: a
    /// failed invariant that the testing knowledge did not hold before is a
    /// failure (an expectation is never one here: the rules state none).
    fn explore_changed(
        &mut self,
        exe: &Path,
        verify: &Path,
        changed: &[String],
        options: Options,
    ) -> Outcome {
        let name = "no new invariant failure in the changed areas";
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
            usd: 0.0,
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
            &verify.join(super::explore::STARTS[0]),
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
                name,
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
                name,
                "passed",
                format!(
                    "{} steps by the rules in {}; no invariant failed that held before",
                    run.actions,
                    run.areas.iter().cloned().collect::<Vec<_>>().join(", ")
                ),
            )
        } else {
            Outcome::new(name, "failed", new.join("\n"))
        }
    }
}

/// The steps of the rules' exploration of a change's areas.
const CHANGED_STEPS: u32 = 10;

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
    /// compiled, ran a test and a test failed.
    #[test]
    fn a_run_on_the_base_is_evidence_only_when_a_test_ran_and_failed() {
        let failed = on_base_verdict(finished(
            false,
            "running 2 tests\ntest a ... FAILED\ntest result: FAILED. 1 passed; 1 failed; 0 ignored",
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
        let node = on_base_verdict(finished(false, "# tests 2\n# pass 1\n# fail 1"));
        assert_eq!(node.0, "failed");
        let python = on_base_verdict(finished(
            false,
            "Ran 3 tests in 0.1s\n\nFAILED (failures=1)",
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
    }
}
