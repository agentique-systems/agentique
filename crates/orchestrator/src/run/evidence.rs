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

use super::{Driver, tests_ran};
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

/// The failing tests of a run, each judged by its own output: those that
/// failed an assertion, and those that ended with another error.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Failures {
    pub assertions: usize,
    pub errors: usize,
}

/// Panics that are errors, not assertions: `unwrap` and `unwrap_err` on
/// nothing or an error (their messages say so).
const UNWRAPPED: [&str; 3] = [
    "called `Option::unwrap()` on a `None` value",
    "called `Result::unwrap()` on an `Err` value",
    "called `Result::unwrap_err()` on an `Ok` value",
];

/// Cargo's failing tests, each from its own `---- <name> stdout ----`
/// section: a panic is an assertion (`assert!` with or without a message,
/// `assert_eq!`, `panic!`), except an `unwrap` on nothing or an error (by
/// its message) or an `expect` (by the line it points to, read in `folder`,
/// the checkout the run was made in); no panic (a test returning `Err`) is
/// an error.
fn cargo_failures(output: &str, folder: Option<&Path>) -> Failures {
    let mut failures = Failures::default();
    let lines: Vec<&str> = output.lines().collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        if !(line.starts_with("---- ") && line.ends_with(" stdout ----")) {
            i += 1;
            continue;
        }
        let mut section = Vec::new();
        i += 1;
        while i < lines.len() {
            let next = lines[i].trim();
            if next.starts_with("---- ") || next == "failures:" || next.starts_with("test result:")
            {
                break;
            }
            section.push(lines[i]);
            i += 1;
        }
        if cargo_asserted(&section, folder) {
            failures.assertions += 1;
        } else {
            failures.errors += 1;
        }
    }
    failures
}

/// Whether a cargo test's section shows an assertion that failed.
fn cargo_asserted(section: &[&str], folder: Option<&Path>) -> bool {
    let Some(at) = section.iter().position(|l| l.contains("panicked at ")) else {
        return false;
    };
    let message = section[at..].join("\n");
    if UNWRAPPED.iter().any(|m| message.contains(m)) {
        return false;
    }
    // `thread 'x' panicked at crates/a/tests/b.rs:12:5:` points to the line
    // that panicked: an `expect` there is an error, whatever it says.
    let place = section[at]
        .split("panicked at ")
        .nth(1)
        .unwrap_or_default()
        .trim_end_matches(':');
    let mut parts = place.rsplitn(3, ':');
    let (_column, line, file) = (parts.next(), parts.next(), parts.next());
    if let (Some(folder), Some(file), Some(line)) =
        (folder, file, line.and_then(|l| l.parse::<usize>().ok()))
        && let Ok(text) = std::fs::read_to_string(folder.join(file))
        && let Some(code) = text.lines().nth(line.saturating_sub(1))
        && (code.contains(".expect(") || code.contains(".unwrap()"))
        && !code.contains("assert")
    {
        return false;
    }
    true
}

/// Node's failing tests (TAP): each `not ok` block, an `AssertionError`
/// (`ERR_ASSERTION`) being an assertion and any other error not; a parent
/// whose subtests failed is counted by them.
fn node_failures(output: &str) -> Failures {
    let mut failures = Failures::default();
    let lines: Vec<&str> = output.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        if !line.trim_start().starts_with("not ok ") {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        let block: Vec<&str> = lines[i + 1..]
            .iter()
            .take_while(|l| {
                let own = l.len() - l.trim_start().len();
                !(own <= indent
                    && (l.trim_start().starts_with("ok ") || l.trim_start().starts_with("not ok ")))
                    && !(own == indent + 2 && l.trim() == "...")
            })
            .copied()
            .collect();
        let block = block.join("\n");
        if block.contains("failureType: 'subtestsFailed'") {
            continue;
        }
        if block.contains("name: 'AssertionError'") || block.contains("code: 'ERR_ASSERTION'") {
            failures.assertions += 1;
        } else {
            failures.errors += 1;
        }
    }
    failures
}

/// Python's unittest: `FAIL:` (an assertion) and `ERROR:` (another
/// exception) sections.
fn python_failures(output: &str) -> Failures {
    Failures {
        assertions: output.lines().filter(|l| l.starts_with("FAIL: ")).count(),
        errors: output.lines().filter(|l| l.starts_with("ERROR: ")).count(),
    }
}

/// Which test runner a criterion's command is, so its output is read only
/// by that runner's rules (another language's markers printed by a test
/// never count).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Runner {
    Cargo,
    Node,
    Python,
}

impl Runner {
    /// The runner of a test command (`roles::test_command` allows only
    /// these three).
    pub fn of(program: &[String]) -> Option<Runner> {
        let first = program.first()?.to_ascii_lowercase();
        let name = first
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(&first)
            .trim_end_matches(".exe");
        match name {
            "cargo" => Some(Runner::Cargo),
            "node" => Some(Runner::Node),
            "python" | "python3" | "py" => Some(Runner::Python),
            _ => None,
        }
    }
}

/// The failing tests of a run's output, each judged by its own block, by
/// the rules of `runner` (all three when it is not known).
pub fn failures(output: &str, folder: Option<&Path>, runner: Option<Runner>) -> Failures {
    let none = Failures {
        assertions: 0,
        errors: 0,
    };
    let only = |wanted: Runner| runner.is_none_or(|r| r == wanted);
    let cargo = if only(Runner::Cargo) {
        cargo_failures(output, folder)
    } else {
        none
    };
    let node = if only(Runner::Node) {
        node_failures(output)
    } else {
        none
    };
    let python = if only(Runner::Python) {
        python_failures(output)
    } else {
        none
    };
    Failures {
        assertions: cargo.assertions + node.assertions + python.assertions,
        errors: cargo.errors + node.errors + python.errors,
    }
}

/// A command criterion's run on the base, judged as evidence: `failed`
/// only when it compiled, ran at least one test and a failing test's own
/// output shows an assertion that failed; `passed` when it passed there
/// (the gate fails); otherwise `no evidence` with the reason. `folder` is
/// the checkout it ran in (where an `expect` that panicked is read);
/// `program` the criterion's command, whose runner decides how its output
/// is read.
pub fn on_base_verdict(
    run: Result<Finished, String>,
    folder: Option<&Path>,
    program: Option<&[String]>,
) -> (&'static str, String) {
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
    let failed = failures(&output, folder, program.and_then(Runner::of));
    let ran = tests_ran(&output)
        .unwrap_or(0)
        .max(failed.assertions + failed.errors);
    if failed.assertions > 0 {
        return (
            "failed",
            format!(
                "{} of its tests failed an assertion on the base: {}",
                failed.assertions,
                agq_execution::process::last_lines(&output, 12)
            ),
        );
    }
    if failed.errors > 0 {
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
                Ok(()) => on_base_verdict(
                    self.execute(&folder, program, Duration::from_secs(1800)),
                    Some(&folder),
                    Some(program),
                ),
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
        let replay = self.replay_in(&built.exe, &source, finding, Options::default())?;
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
    /// its start state in `source` (the base's checkout), its copy checked
    /// first (the W13.7 repair): one that does not match is the error. Its
    /// project gone from the base cannot be replayed (it diverges).
    fn replay_in(
        &mut self,
        exe: &Path,
        source: &Path,
        finding: &Finding,
        options: Options,
    ) -> Result<Replay, String> {
        let base = self.cycle().base.clone().unwrap_or_default();
        let copy = match self.copy_of(source, &finding.start, &base) {
            Ok(copy) => copy,
            Err(reason) => return Ok(Replay::Diverged { at: 0, reason }),
        };
        let mut instance = self.setup.studios.instance(
            exe,
            &explore::within(source, &finding.start),
            &self.folder("replay"),
            options,
        );
        let controls = self.controls.clone();
        findings::replay(instance.as_mut(), finding, Some(&copy), &mut || {
            controls.stopped()
        })
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
                Ok(Replay::Passed) => Outcome::new(REPLAY, "passed", "it holds on the change"),
                Ok(Replay::Failed { message }) => {
                    Outcome::new(REPLAY, "failed", format!("it still fails: {message}"))
                }
                // Not the project: a failure, never a pass (the W13.7
                // repair).
                Err(why) => Outcome::new(REPLAY, "failed", why),
                Ok(Replay::Diverged { at, reason }) => Outcome::new(
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

    /// A short exploration by the rules of the areas the change touched, on
    /// the objective's target's project: a failed invariant that the
    /// testing knowledge did not hold before is a failure (an expectation
    /// is never one here: the rules state none).
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
        // What the objective explores, at the base (the W13.7 repair); the
        // sample for an objective that explores nothing of its own.
        let targeted = self.objective.target.as_ref().map(|t| t.project.clone());
        let project = targeted
            .clone()
            .unwrap_or_else(|| super::explore::SAMPLE.to_string());
        let base = self.cycle().base.clone().unwrap_or_default();
        let source = match self.copy_of(start, &project, &base) {
            Ok(source) => source,
            // Its project gone from the base is a failure; the sample
            // missing (a repository without it) explores nothing.
            Err(problem) => {
                return Outcome::new(
                    CHANGED_AREAS,
                    if targeted.is_some() {
                        "failed"
                    } else {
                        "not run"
                    },
                    format!("it did not explore: {problem}"),
                );
            }
        };
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
            start: project.clone(),
            begin: Default::default(),
            source: Some(source),
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
            &explore::within(start, &project),
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
        // Another copy than the project's is never explored instead: a
        // failure, not a criterion left unrun.
        if let Some(why) = &run.mismatch {
            return Outcome::new(
                CHANGED_AREAS,
                "failed",
                format!("the test instance did not open {project} at the base: {why}"),
            );
        }
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
    /// compiled, ran a test and a failing test's own output shows an
    /// assertion that failed.
    #[test]
    fn a_run_on_the_base_is_evidence_only_when_a_test_failed_an_assertion() {
        let verdict = |success, out: &str| on_base_verdict(finished(success, out), None, None).0;
        // Cargo: `assert!` with a message only (this repository's style),
        // `assert_eq!`, a plain panic are assertions.
        let cargo = |body: &str| {
            format!(
                "running 2 tests\ntest tests::label_is_set ... FAILED\ntest tests::other ... ok\n\nfailures:\n\n---- tests::label_is_set stdout ----\n\nthread 'tests::label_is_set' panicked at crates/x/src/lib.rs:12:9:\n{body}\nnote: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n\n\nfailures:\n    tests::label_is_set\n\ntest result: FAILED. 1 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n"
            )
        };
        assert_eq!(
            verdict(false, &cargo("the Archive button has no label")),
            "failed"
        );
        assert_eq!(
            verdict(
                false,
                &cargo("assertion `left == right` failed\n  left: 1\n right: 2")
            ),
            "failed"
        );
        assert_eq!(
            verdict(false, &cargo("called `Option::unwrap()` on a `None` value")),
            "no evidence"
        );
        assert_eq!(
            verdict(
                false,
                &cargo("called `Result::unwrap()` on an `Err` value: NotFound")
            ),
            "no evidence"
        );
        // An `expect`, by the line it points to in the checkout.
        let folder = tempfile::tempdir().unwrap();
        let file = folder.path().join("crates/x/src/lib.rs");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        let mut code = "fn x() {}\n".repeat(11);
        code.push_str("        let label = label().expect(\"a label\");\n");
        std::fs::write(&file, code).unwrap();
        let expected = on_base_verdict(
            finished(false, &cargo("a label: NotFound")),
            Some(folder.path()),
            None,
        );
        assert_eq!(expected.0, "no evidence", "{}", expected.1);
        // A passing test named for assertions, or a log line, does not turn
        // an error into evidence.
        let named = "running 2 tests\ntest tests::assertion_holds ... ok\ntest tests::label ... FAILED\nassertion log line\n\nfailures:\n\n---- tests::label stdout ----\nthread 'tests::label' panicked at crates/x/src/lib.rs:3:5:\ncalled `Option::unwrap()` on a `None` value\n\nfailures:\n    tests::label\n\ntest result: FAILED. 1 passed; 1 failed; 0 ignored";
        assert_eq!(verdict(false, named), "no evidence");
        // A test returning an error: no panic, an error.
        let returned = "running 1 test\ntest t ... FAILED\n\nfailures:\n\n---- t stdout ----\nError: NotFound\n\nfailures:\n    t\n\ntest result: FAILED. 0 passed; 1 failed";
        assert_eq!(verdict(false, returned), "no evidence");
        // Node: the failing subtest's own block.
        let node = |name: &str, code: &str| {
            format!(
                "TAP version 13\n# Subtest: assertion helpers stay quiet\nok 1 - assertion helpers stay quiet\n  ---\n  duration_ms: 0.4\n  ...\n# Subtest: the note says it improved\nnot ok 2 - the note says it improved\n  ---\n  duration_ms: 1.2\n  failureType: 'testCodeFailure'\n  error: \"x\"\n  code: '{code}'\n  name: '{name}'\n  ...\n1..2\n# tests 2\n# suites 0\n# pass 1\n# fail 1\n"
            )
        };
        assert_eq!(
            verdict(false, &node("AssertionError", "ERR_ASSERTION")),
            "failed"
        );
        assert_eq!(
            verdict(false, &node("TypeError", "ERR_TEST_FAILURE")),
            "no evidence",
            "a passing test named for assertions does not count"
        );
        // Python: FAIL is an assertion, ERROR is not.
        let python = |kind: &str, error: &str| {
            format!(
                "======================================================================\n{kind}: test_label (tests.test_a.T.test_label)\n----------------------------------------------------------------------\nTraceback (most recent call last):\n  File \"tests/test_a.py\", line 5, in test_label\n{error}\n\n----------------------------------------------------------------------\nRan 3 tests in 0.010s\n\nFAILED ({}=1)\n",
                if kind == "FAIL" { "failures" } else { "errors" }
            )
        };
        assert_eq!(
            verdict(false, &python("FAIL", "AssertionError: 1 != 2")),
            "failed"
        );
        assert_eq!(
            verdict(
                false,
                &python("ERROR", "NameError: name 'fixed' is not defined")
            ),
            "no evidence"
        );
        // What runs no test, does not compile or does not finish.
        assert_eq!(
            verdict(
                false,
                "error[E0425]: cannot find function `fixed` in this scope\nerror: could not compile `agq-x`"
            ),
            "no evidence"
        );
        assert_eq!(
            verdict(
                true,
                "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out"
            ),
            "no evidence"
        );
        assert_eq!(
            verdict(true, "test result: ok. 3 passed; 0 failed"),
            "passed"
        );
        assert_eq!(
            on_base_verdict(Err("timed out after 1800 s".into()), None, None).0,
            "no evidence"
        );
        assert_eq!(
            verdict(
                false,
                "Error: Cannot find module './note.mjs'\n# tests 0\n# fail 0"
            ),
            "no evidence"
        );
        assert_eq!(
            verdict(false, "running 1 test\nerror: test failed, to rerun"),
            "no evidence"
        );
    }

    /// A run's output is read by its own runner's rules: another
    /// language's failure marker printed by a test never counts.
    #[test]
    fn a_runs_output_is_read_by_its_own_runners_rules() {
        let program = |words: &[&str]| words.iter().map(|w| w.to_string()).collect::<Vec<_>>();
        assert_eq!(
            Runner::of(&program(&["cargo", "test"])),
            Some(Runner::Cargo)
        );
        assert_eq!(
            Runner::of(&program(&["C:\\Tools\\node.exe", "--test"])),
            Some(Runner::Node)
        );
        assert_eq!(
            Runner::of(&program(&["python3", "-m", "unittest"])),
            Some(Runner::Python)
        );
        assert_eq!(Runner::of(&program(&["make"])), None);
        // A cargo test that failed on an `unwrap` but printed Python's
        // marker: no evidence when read as cargo, which it is.
        let out = "running 1 test\ntest tests::label ... FAILED\n\nfailures:\n\n---- tests::label stdout ----\nFAIL: looks like an assertion\nthread 'tests::label' panicked at crates/x/src/lib.rs:3:5:\ncalled `Option::unwrap()` on a `None` value\n\nfailures:\n    tests::label\n\ntest result: FAILED. 0 passed; 1 failed; 0 ignored";
        let cargo = program(&["cargo", "test", "-p", "x", "label"]);
        assert_eq!(
            on_base_verdict(finished(false, out), None, Some(&cargo)).0,
            "no evidence"
        );
        assert_eq!(failures(out, None, Some(Runner::Python)).assertions, 1);
        assert_eq!(failures(out, None, Some(Runner::Cargo)).assertions, 0);
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
