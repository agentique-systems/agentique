//! A failure of the repository's checks that is not the change's (W13.7,
//! ROADMAP §4.16): which tests failed, read from the failed jobs' logs;
//! whether the change can have caused them, from the crates it touches and
//! the crates that depend on them (a test that reads files outside its own
//! crate counts as affected by every change); a check that never reached a
//! verdict (cancelled, or failed in setting up) as the checks' own
//! machinery; and, once a repair of a failure elsewhere is merged and is
//! shown to touch what failed, the reviewed change carried onto it, only if
//! its patch is unchanged. The repository's checks stay authoritative: a
//! change carried over merges only when they pass on it, and nothing here
//! reruns a failure until it happens to pass.

use crate::forge;
use agq_execution::process::Program;
use agq_execution::{Executor, Scope};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Duration;

/// A test that failed in the repository's checks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FailingTest {
    /// The crate (package) it belongs to, when the log names it.
    pub package: String,
    /// The test target: `lib`, or the name of a test or binary target.
    pub target: String,
    pub name: String,
}

impl FailingTest {
    pub fn line(&self) -> String {
        format!("{} ({} {})", self.name, self.package, self.target)
    }
}

/// What a failed check's log says: the steps that failed, the tests among
/// them, and the lines that show why.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CiFailure {
    /// The check (the CI job) as the host names it.
    pub check: String,
    pub steps: Vec<String>,
    pub tests: Vec<FailingTest>,
    pub excerpt: String,
    /// The host cancelled it before it reached a verdict.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub cancelled: bool,
}

impl CiFailure {
    /// Several failed checks as one: their steps, tests and excerpts.
    pub fn joined(failures: Vec<CiFailure>) -> CiFailure {
        let mut all = CiFailure::default();
        for failure in failures {
            all.check = if all.check.is_empty() {
                failure.check
            } else {
                format!("{}, {}", all.check, failure.check)
            };
            all.steps.extend(failure.steps);
            all.tests.extend(failure.tests);
            if !failure.excerpt.is_empty() {
                all.excerpt = format!("{}\n{}", all.excerpt, failure.excerpt)
                    .trim_start()
                    .to_string();
            }
            all.cancelled |= failure.cancelled;
        }
        all
    }
}

/// Whether a CI step only sets the job up (or tears it down): a failure
/// there is the checks' own machinery, never a verdict on the change.
pub fn is_setup_step(step: &str) -> bool {
    let step = step.trim().to_lowercase();
    ["set up", "run actions/", "post ", "complete job"]
        .iter()
        .any(|p| step.starts_with(p))
        || ["rustup", "apt-get", "cache"]
            .iter()
            .any(|w| step.contains(w))
}

/// Reads a failed job's log (`gh run view --log-failed`: job, step and a
/// timestamped line, tab-separated) into the failing steps and tests. A
/// test binary that fails ends with cargo's "to rerun pass `-p <package>
/// --test <target>`" (or `--lib`, `--bin <name>`), which names the package
/// and target of the failures listed before it.
pub fn parse_failed_log(check: &str, log: &str) -> CiFailure {
    let mut failure = CiFailure {
        check: check.to_string(),
        ..CiFailure::default()
    };
    let mut pending: Vec<String> = Vec::new();
    let mut shown: Vec<String> = Vec::new();
    for line in log.lines() {
        let mut parts = line.splitn(3, '\t');
        let (_, step, rest) = (parts.next(), parts.next(), parts.next());
        let (Some(step), Some(rest)) = (step, rest) else {
            continue;
        };
        // The timestamp first, when there is one.
        let text = match rest.split_once(' ') {
            Some((stamp, text)) if stamp.starts_with(|c: char| c.is_ascii_digit()) => text,
            _ => rest,
        }
        .trim_end();
        let step = step.trim().to_string();
        if !failure.steps.contains(&step) {
            failure.steps.push(step);
        }
        if let Some(name) = text
            .strip_prefix("test ")
            .and_then(|t| t.strip_suffix(" ... FAILED"))
        {
            let name = name.trim().to_string();
            if !pending.contains(&name) {
                pending.push(name);
            }
        }
        if let Some((package, target)) = rerun_target(text) {
            // A target that ended without naming a failing test (it crashed
            // or aborted) failed whole.
            if pending.is_empty() {
                pending.push(WHOLE_TARGET.to_string());
            }
            for name in pending.drain(..) {
                let test = FailingTest {
                    package: package.clone(),
                    target: target.clone(),
                    name,
                };
                if !failure.tests.contains(&test) {
                    failure.tests.push(test);
                }
            }
        }
        if text.contains("panicked at")
            || text.contains("FAILED")
            || text.starts_with("error")
            || text.contains("left:")
            || text.contains("right:")
            || text.contains("Err` value")
        {
            shown.push(text.to_string());
        }
    }
    // Failures no rerun line placed: their package is unknown.
    for name in pending {
        failure.tests.push(FailingTest {
            package: String::new(),
            target: String::new(),
            name,
        });
    }
    shown.dedup();
    failure.excerpt = shown
        .iter()
        .rev()
        .take(14)
        .rev()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    failure
}

/// The name a failing target is listed under when it ended without naming
/// a failing test.
pub const WHOLE_TARGET: &str = "the whole target (it ended without naming a failing test)";

/// `(package, target)` from cargo's "error: test failed, to rerun pass
/// `-p <package> --test <target>`" (`--lib` is `lib`, `--bin <b>` is `b`).
fn rerun_target(text: &str) -> Option<(String, String)> {
    let at = text.find("to rerun pass `")?;
    let args = text[at + "to rerun pass `".len()..].split('`').next()?;
    let words: Vec<&str> = args.split_whitespace().collect();
    let package = words
        .iter()
        .position(|w| *w == "-p")
        .and_then(|i| words.get(i + 1))?
        .to_string();
    let target = if words.contains(&"--lib") {
        "lib".to_string()
    } else if let Some(i) = words.iter().position(|w| *w == "--test" || *w == "--bin") {
        words.get(i + 1)?.to_string()
    } else {
        "lib".to_string()
    };
    Some((package, target))
}

/// The workspace's crates: each one's folder (repository-relative, with
/// `/`) and the workspace crates it depends on.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Workspace {
    crates: BTreeMap<String, (String, BTreeSet<String>)>,
}

impl Workspace {
    /// From `cargo metadata --no-deps --format-version 1`, its paths made
    /// relative to the workspace's root as cargo states it (`root` when it
    /// does not).
    pub fn from_metadata(metadata: &Value, root: &Path) -> Workspace {
        let root = metadata["workspace_root"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| root.to_string_lossy().into_owned())
            .replace('\\', "/");
        let root = root.trim_end_matches('/');
        let packages = metadata["packages"].as_array().cloned().unwrap_or_default();
        let names: BTreeSet<String> = packages
            .iter()
            .filter_map(|p| p["name"].as_str().map(str::to_string))
            .collect();
        let mut crates = BTreeMap::new();
        for package in &packages {
            let Some(name) = package["name"].as_str() else {
                continue;
            };
            let manifest = package["manifest_path"]
                .as_str()
                .unwrap_or_default()
                .replace('\\', "/");
            let dir = manifest
                .strip_suffix("/Cargo.toml")
                .unwrap_or(&manifest)
                .strip_prefix(root)
                .unwrap_or(&manifest)
                .trim_start_matches('/')
                .to_string();
            let deps: BTreeSet<String> = package["dependencies"]
                .as_array()
                .into_iter()
                .flatten()
                // Dev-dependencies too: a crate's tests build with them.
                .filter_map(|d| d["name"].as_str())
                .filter(|d| names.contains(*d))
                .map(str::to_string)
                .collect();
            crates.insert(name.to_string(), (dir, deps));
        }
        Workspace { crates }
    }

    /// Reads the workspace in `folder` (`cargo metadata`, offline).
    pub fn read(folder: &Path) -> Result<Workspace, String> {
        let program = Program::new(
            "cargo",
            &[
                "metadata",
                "--no-deps",
                "--format-version",
                "1",
                "--offline",
            ],
        );
        let executor = Executor::new(Scope::read_only(folder).map_err(|e| e.to_string())?)
            .trusted(true)
            .allow(vec![program.clone()]);
        let finished = executor
            .run(&program, "", Duration::from_secs(120))
            .map_err(|e| format!("{}: {e}", program.display()))?;
        if !finished.success {
            return Err(format!("cargo metadata failed: {}", finished.stderr.trim()));
        }
        let metadata: Value =
            serde_json::from_str(&finished.stdout).map_err(|e| format!("cargo metadata: {e}"))?;
        Ok(Workspace::from_metadata(&metadata, folder))
    }

    /// Whether `test`'s target reads files outside its own crate (a path
    /// leaving it, `"../…"`, or one built from `CARGO_MANIFEST_DIR`), as read
    /// in the checkout `folder`: then any change may affect it. A target
    /// whose source cannot be read counts as reading outside.
    pub fn reads_beyond(&self, folder: &Path, test: &FailingTest) -> bool {
        let Some((dir, _)) = self.crates.get(&test.package) else {
            return true;
        };
        let crate_folder = folder.join(dir);
        let files: Vec<std::path::PathBuf> = if test.target == "lib" {
            rust_files(&crate_folder.join("src"))
        } else {
            [
                crate_folder
                    .join("tests")
                    .join(format!("{}.rs", test.target)),
                crate_folder
                    .join("tests")
                    .join(&test.target)
                    .join("main.rs"),
                crate_folder
                    .join("src")
                    .join("bin")
                    .join(format!("{}.rs", test.target)),
            ]
            .into_iter()
            .filter(|f| f.is_file())
            .collect()
        };
        files.is_empty()
            || files.iter().any(|file| {
                std::fs::read_to_string(file).map_or(true, |text| {
                    text.contains("\"../") || text.contains("CARGO_MANIFEST_DIR")
                })
            })
    }

    /// The crate whose folder holds `path`, if any.
    fn crate_of(&self, path: &str) -> Option<&str> {
        self.crates
            .iter()
            .filter(|(_, (dir, _))| !dir.is_empty() && path.starts_with(&format!("{dir}/")))
            .max_by_key(|(_, (dir, _))| dir.len())
            .map(|(name, _)| name.as_str())
    }

    /// The crates `paths` can affect: those that hold them and those that
    /// depend on these, transitively. `Err` names a path outside every
    /// crate that may affect any of them (the workspace's manifests, the
    /// model and examples the tests read, tools, CI): then every crate is.
    /// Prose (`.md`) outside the crates affects none.
    pub fn affected(&self, paths: &[String]) -> Result<BTreeSet<String>, String> {
        let mut touched = BTreeSet::new();
        for path in paths {
            match self.crate_of(path) {
                Some(name) => {
                    touched.insert(name.to_string());
                }
                None if path.ends_with(".md") => {}
                None => return Err(path.clone()),
            }
        }
        let mut affected = touched.clone();
        loop {
            let more: Vec<String> = self
                .crates
                .iter()
                .filter(|(name, (_, deps))| {
                    !affected.contains(*name) && deps.iter().any(|d| affected.contains(d))
                })
                .map(|(name, _)| name.clone())
                .collect();
            if more.is_empty() {
                return Ok(affected);
            }
            affected.extend(more);
        }
    }
}

/// The `.rs` files under `folder`.
fn rust_files(folder: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(folder).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            files.push(path);
        }
    }
    files
}

/// Whose failure it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Attribution {
    /// The change may have caused it: it goes back to the implementer.
    Change(String),
    /// The change cannot have caused it: a defect already on the base, which
    /// a repair cycle may repair.
    Elsewhere(String),
    /// The checks never reached a verdict (cancelled, or failed setting the
    /// job up): their own machinery. Nothing is repaired and nothing reruns
    /// them here; the change waits.
    Infrastructure(String),
}

/// Several checks' attributions as one: the change's if any is; the
/// machinery's if all are; otherwise elsewhere.
pub fn combined(all: Vec<Attribution>) -> Attribution {
    let words = |a: &Attribution| match a {
        Attribution::Change(w) | Attribution::Elsewhere(w) | Attribution::Infrastructure(w) => {
            w.clone()
        }
    };
    if let Some(change) = all.iter().find(|a| matches!(a, Attribution::Change(_))) {
        return change.clone();
    }
    let text = all.iter().map(words).collect::<Vec<_>>().join("; ");
    if all
        .iter()
        .all(|a| matches!(a, Attribution::Infrastructure(_)))
    {
        Attribution::Infrastructure(text)
    } else {
        Attribution::Elsewhere(text)
    }
}

/// Whether a repair touching `paths` can have repaired every failing test
/// of `tests`: it touches what every crate depends on, or a crate that is
/// or affects each failing test's crate. Otherwise carrying a blocked change
/// onto it would only run the failing checks again; why not.
pub fn repairs_what_failed(
    tests: &[FailingTest],
    paths: &[String],
    workspace: &Workspace,
) -> Result<(), String> {
    let affected = match workspace.affected(paths) {
        Ok(affected) => affected,
        Err(_) => return Ok(()),
    };
    let missed: Vec<String> = tests
        .iter()
        .filter(|t| !affected.contains(&t.package))
        .map(FailingTest::line)
        .collect();
    if missed.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "the repair touches nothing that {} can depend on",
            missed.join(", ")
        ))
    }
}

/// Whether `failure` can be the change's, which touches `paths`. Only a
/// failure whose every failing test is in a crate the change neither
/// touches nor can affect is elsewhere; a failing step that is not a test
/// (formatting, lints, the build itself) or a test whose crate the log does
/// not name is taken to be the change's, as is everything when the change
/// touches what every crate depends on.
pub fn attribute(
    failure: &CiFailure,
    paths: &[String],
    workspace: &Workspace,
    folder: &Path,
) -> Attribution {
    if failure.tests.is_empty() {
        if failure.cancelled {
            return Attribution::Infrastructure(format!(
                "{} was cancelled before it reached a verdict",
                failure.check
            ));
        }
        if !failure.steps.is_empty() && failure.steps.iter().all(|s| is_setup_step(s)) {
            return Attribution::Infrastructure(format!(
                "{} failed setting the job up ({}), before any check ran",
                failure.check,
                failure.steps.join("; ")
            ));
        }
        return Attribution::Change(format!(
            "the failing step is not a test that can be placed in a crate ({})",
            failure.steps.join("; ")
        ));
    }
    let affected = match workspace.affected(paths) {
        Ok(affected) => affected,
        Err(path) => {
            return Attribution::Change(format!(
                "the change touches {path}, which every crate may depend on"
            ));
        }
    };
    if let Some(test) = failure
        .tests
        .iter()
        .find(|t| t.package.is_empty() || affected.contains(&t.package))
    {
        return Attribution::Change(if test.package.is_empty() {
            format!("the log does not say which crate {} belongs to", test.name)
        } else {
            format!(
                "{} is in {}, which the change touches or affects",
                test.name, test.package
            )
        });
    }
    if let Some(test) = failure
        .tests
        .iter()
        .find(|t| workspace.reads_beyond(folder, t))
    {
        return Attribution::Change(format!(
            "{} reads files outside its crate, which any change may affect",
            test.line()
        ));
    }
    let packages: BTreeSet<&str> = failure.tests.iter().map(|t| t.package.as_str()).collect();
    Attribution::Elsewhere(format!(
        "{} fail{} in {}, which the change neither touches nor can affect by the workspace's dependencies (it affects only {}); test targets after the first failing one did not run",
        failure
            .tests
            .iter()
            .map(FailingTest::line)
            .collect::<Vec<_>>()
            .join(", "),
        if failure.tests.len() == 1 { "s" } else { "" },
        packages.into_iter().collect::<Vec<_>>().join(", "),
        if affected.is_empty() {
            "prose".to_string()
        } else {
            affected.into_iter().collect::<Vec<_>>().join(", ")
        }
    ))
}

/// The reviewed change (`reviewed`, made on `base`) carried onto
/// `new_base`: the tree of the three-way merge, when it is clean and the
/// change's patch onto `new_base` reads as the reviewed patch (their diffs
/// agree but for index lines and hunk positions); otherwise why not, and
/// the change needs its own new review.
pub fn carry_over(
    repository: &Path,
    base: &str,
    reviewed: &str,
    new_base: &str,
) -> Result<String, String> {
    let merged = forge::run(
        repository,
        &[
            "git",
            "merge-tree",
            "--write-tree",
            "--merge-base",
            base,
            new_base,
            reviewed,
        ],
        Duration::from_secs(120),
    )
    .map_err(|e| format!("the change does not merge cleanly onto the new base: {e}"))?;
    let tree = merged
        .stdout
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    if tree.len() < 40 || !tree.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("git merge-tree did not say the tree: {tree}"));
    }
    let reviewed_patch = patch_id(repository, base, reviewed)?;
    let carried_patch = patch_id(repository, new_base, &tree)?;
    if reviewed_patch != carried_patch {
        return Err(
            "the change's patch onto the new base differs from the reviewed one".to_string(),
        );
    }
    Ok(tree)
}

/// The difference from `from` to `to` as its patch reads, wherever it
/// applies: the index lines and the hunks' positions left out, as git's
/// patch ids leave them out.
fn patch_id(repository: &Path, from: &str, to: &str) -> Result<String, String> {
    let diff = forge::run(
        repository,
        &["git", "diff", "--no-color", "--no-ext-diff", from, to],
        Duration::from_secs(120),
    )?;
    Ok(diff
        .stdout
        .lines()
        .filter(|line| !line.starts_with("index "))
        .map(|line| match line.strip_prefix("@@") {
            // `@@ -a,b +c,d @@ context`: only what follows the position.
            Some(rest) => format!("@@{}", rest.split_once("@@").map_or("", |(_, c)| c)),
            None => line.to_string(),
        })
        .collect::<Vec<_>>()
        .join(
            "
",
        ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The log of #125's failed check, as `gh run view --log-failed` gave it
    /// (shortened).
    const LOG: &str = "workspace\tRun cargo test --locked --workspace\t2026-10-10T11:45:53.8501601Z test an_undeclared_change_is_listed_as_changed_but_not_named ... FAILED
workspace\tRun cargo test --locked --workspace\t2026-10-10T11:45:54.4068797Z thread 'an_undeclared_change_is_listed_as_changed_but_not_named' (10678) panicked at crates/orchestrator/tests/traceability.rs:58:10:
workspace\tRun cargo test --locked --workspace\t2026-10-10T11:45:54.4068900Z called `Result::unwrap()` on an `Err` value: \"the project is already open in another Agentique window\"
workspace\tRun cargo test --locked --workspace\t2026-10-10T11:45:54.4072646Z test result: FAILED. 4 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.59s
workspace\tRun cargo test --locked --workspace\t2026-10-10T11:45:54.4073112Z error: test failed, to rerun pass `-p agq-orchestrator --test traceability`";

    fn workspace() -> Workspace {
        let package = |name: &str, dir: &str, deps: &[&str]| {
            json!({
                "name": name,
                "manifest_path": format!("C:\\repo\\{dir}\\Cargo.toml"),
                "dependencies": deps.iter().map(|d| json!({ "name": d })).collect::<Vec<_>>(),
            })
        };
        Workspace::from_metadata(
            &json!({ "packages": [
                package("agq-history", "crates\\history", &["git2"]),
                package("agq-orchestrator", "crates\\orchestrator", &["agq-history", "serde"]),
                package("agq-studio-native", "crates\\studio-native", &["agq-orchestrator"]),
            ] }),
            Path::new("C:\\repo"),
        )
    }

    #[test]
    fn a_failed_log_names_its_tests_their_crate_and_why() {
        let failure = parse_failed_log("workspace", LOG);
        assert_eq!(failure.steps, ["Run cargo test --locked --workspace"]);
        assert_eq!(
            failure.tests,
            [FailingTest {
                package: "agq-orchestrator".into(),
                target: "traceability".into(),
                name: "an_undeclared_change_is_listed_as_changed_but_not_named".into(),
            }]
        );
        assert!(
            failure
                .excerpt
                .contains("already open in another Agentique window")
        );
        assert_eq!(
            forge::run_and_job(
                "workspace fail (https://github.com/o/r/actions/runs/38049431872/job/114205402717)"
            ),
            Some(("38049431872".into(), Some("114205402717".into())))
        );
        assert_eq!(forge::run_and_job("workspace fail ()"), None);
        assert_eq!(
            rerun_target("error: test failed, to rerun pass `-p agq-history --lib`"),
            Some(("agq-history".into(), "lib".into()))
        );
    }

    /// A checkout holding the failing test's source, reading only its own
    /// crate unless `beyond`.
    fn checkout(beyond: bool) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let tests = dir.path().join("crates/orchestrator/tests");
        std::fs::create_dir_all(&tests).unwrap();
        std::fs::write(
            tests.join("traceability.rs"),
            if beyond {
                "const MODEL: &str = include_str!(\"../../../model/Agentique.sysml\");\n"
            } else {
                "fn repository(dir: &Path) {}\n"
            },
        )
        .unwrap();
        dir
    }

    /// #125 changed only the Studio; the failing test is the Orchestrator's,
    /// which does not depend on the Studio and reads only its own crate:
    /// elsewhere. A change to the history crate, which the Orchestrator
    /// depends on, may have caused it; so may any change when the test reads
    /// files outside its crate.
    #[test]
    fn a_failure_in_a_crate_the_change_cannot_affect_is_elsewhere() {
        let failure = parse_failed_log("workspace", LOG);
        let own = checkout(false);
        let folder = own.path();
        let studio = vec!["crates/studio-native/src/panels/requirements.rs".to_string()];
        assert!(matches!(
            attribute(&failure, &studio, &workspace(), folder),
            Attribution::Elsewhere(why) if why.contains("agq-orchestrator") && why.contains("agq-studio-native") && why.contains("did not run")
        ));
        let beyond = checkout(true);
        assert!(matches!(
            attribute(&failure, &studio, &workspace(), beyond.path()),
            Attribution::Change(why) if why.contains("outside its crate")
        ));
        let history = vec!["crates/history/src/lib.rs".to_string()];
        assert!(matches!(
            attribute(&failure, &history, &workspace(), folder),
            Attribution::Change(why) if why.contains("touches or affects")
        ));
        // What every crate may read: the change's.
        let model = vec!["model/Agentique.sysml".to_string()];
        assert!(matches!(
            attribute(&failure, &model, &workspace(), folder),
            Attribution::Change(_)
        ));
        // Prose affects none.
        let docs = vec!["docs/stages.md".to_string(), studio[0].clone()];
        assert!(matches!(
            attribute(&failure, &docs, &workspace(), folder),
            Attribution::Elsewhere(_)
        ));
        // A failing step that is not a test: the change's.
        let lints = parse_failed_log(
            "workspace",
            "workspace\tRun cargo clippy\t2026-10-10T11:00:00Z error: unused variable",
        );
        assert!(matches!(
            attribute(&lints, &studio, &workspace(), folder),
            Attribution::Change(_)
        ));
        // A test target that crashed without naming a test: failed whole.
        let crashed = parse_failed_log(
            "workspace",
            "workspace\tRun cargo test\t2026-10-10T11:00:00Z error: test failed, to rerun pass `-p agq-studio-native --lib`",
        );
        assert_eq!(crashed.tests[0].name, WHOLE_TARGET);
        assert!(matches!(
            attribute(&crashed, &studio, &workspace(), folder),
            Attribution::Change(_)
        ));
    }

    /// A check that never reached a verdict is the machinery's; several
    /// checks are the change's if any is; a repair is carried over only if
    /// it touches what failed.
    #[test]
    fn the_machinery_several_checks_and_a_repair_that_touches_what_failed() {
        let folder = checkout(false);
        let studio = vec!["crates/studio-native/src/panels/requirements.rs".to_string()];
        let setup = parse_failed_log(
            "workspace",
            "workspace\tRun actions/checkout@11d5960a\t2026-10-10T11:00:00Z error: fetch failed",
        );
        assert!(matches!(
            attribute(&setup, &studio, &workspace(), folder.path()),
            Attribution::Infrastructure(why) if why.contains("setting the job up")
        ));
        let cancelled = CiFailure {
            check: "workspace".into(),
            cancelled: true,
            ..CiFailure::default()
        };
        assert!(matches!(
            attribute(&cancelled, &studio, &workspace(), folder.path()),
            Attribution::Infrastructure(_)
        ));
        let elsewhere = attribute(
            &parse_failed_log("workspace", LOG),
            &studio,
            &workspace(),
            folder.path(),
        );
        assert!(matches!(
            combined(vec![
                elsewhere.clone(),
                Attribution::Infrastructure("x".into())
            ]),
            Attribution::Elsewhere(_)
        ));
        assert!(matches!(
            combined(vec![elsewhere, Attribution::Change("y".into())]),
            Attribution::Change(why) if why == "y"
        ));
        assert!(matches!(
            combined(vec![Attribution::Infrastructure("x".into())]),
            Attribution::Infrastructure(_)
        ));
        let failing = parse_failed_log("workspace", LOG).tests;
        let in_history = vec!["crates/history/src/lib.rs".to_string()];
        assert!(repairs_what_failed(&failing, &in_history, &workspace()).is_ok());
        let in_studio = vec!["crates/studio-native/src/main.rs".to_string()];
        assert!(
            repairs_what_failed(&failing, &in_studio, &workspace())
                .unwrap_err()
                .contains("an_undeclared_change")
        );
        let prose = vec!["docs/stages.md".to_string()];
        assert!(repairs_what_failed(&failing, &prose, &workspace()).is_err());
    }

    fn git(folder: &Path, args: &[&str]) -> String {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(folder)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    /// The reviewed change carried onto a base that moved elsewhere keeps
    /// its patch; one the new base changed under it does not.
    #[test]
    fn a_reviewed_change_is_carried_onto_a_new_base_only_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        git(repo, &["init", "-q", "-b", "main"]);
        git(repo, &["config", "user.name", "Agentique test"]);
        git(repo, &["config", "user.email", "test@example.invalid"]);
        std::fs::write(repo.join("a.txt"), "one\ntwo\nthree\n").unwrap();
        std::fs::write(repo.join("b.txt"), "base\n").unwrap();
        git(repo, &["add", "-A"]);
        git(repo, &["commit", "-q", "-m", "base"]);
        let base = git(repo, &["rev-parse", "HEAD"]);
        // The reviewed change edits a.txt.
        git(repo, &["checkout", "-q", "-b", "change"]);
        std::fs::write(repo.join("a.txt"), "one\nTWO\nthree\n").unwrap();
        git(repo, &["commit", "-q", "-am", "change"]);
        let reviewed = git(repo, &["rev-parse", "HEAD"]);
        // The repair edits b.txt on main.
        git(repo, &["checkout", "-q", "main"]);
        std::fs::write(repo.join("b.txt"), "repaired\n").unwrap();
        git(repo, &["commit", "-q", "-am", "repair"]);
        let repaired = git(repo, &["rev-parse", "HEAD"]);
        let tree = carry_over(repo, &base, &reviewed, &repaired).unwrap();
        assert_eq!(
            git(repo, &["show", &format!("{tree}:a.txt")]),
            "one\nTWO\nthree"
        );
        assert_eq!(git(repo, &["show", &format!("{tree}:b.txt")]), "repaired");
        // A new base that rewrote the same line: not carried.
        std::fs::write(repo.join("a.txt"), "one\nzwei\nthree\n").unwrap();
        git(repo, &["commit", "-q", "-am", "elsewhere"]);
        let moved = git(repo, &["rev-parse", "HEAD"]);
        assert!(carry_over(repo, &base, &reviewed, &moved).is_err());
    }
}
