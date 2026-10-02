//! Implementation tasks (ROADMAP §4.15, W8.4): what a task is about, taken
//! from the model and never from names, and the verification of a working
//! copy that both the worker and the Studio run. The Studio trusts the
//! verification it runs itself, never the worker's account of it.

use crate::checks::{contract_shapes, linked_tests, module_boundaries};
use crate::links::Links;
use crate::{CheckKind, ImplementationCheck, run_implementation};
use agq_execution::Executor;
use agq_execution::process::{Program, last_lines};
use agq_language::{ElementId, ElementKind, Semantics, Tree, print_element};
use agq_simulation::digest::{closure, model_digest};
use agq_simulation::{Mode, Request, RunStatus, Verdict};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// What the worker is given: the element to implement and the parts of the
/// model that say what it must do.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Brief {
    pub element: u64,
    pub title: String,
    /// The brief as the worker reads it.
    pub text: String,
    /// The scenarios that exercise it: its acceptance behaviour.
    pub scenarios: Vec<u64>,
    /// Their names, in the same order.
    #[serde(default)]
    pub scenario_names: Vec<String>,
    /// Paths the worker may not write.
    pub protected: Vec<String>,
    /// The checks the task must pass, fixed when the Operator approves it
    /// ([`required_checks`]).
    #[serde(default)]
    pub required: Vec<RequiredCheck>,
}

/// Paths no task writes, whatever the links say: git's own folder, build
/// output, secrets, and the build's configuration (Cargo's configuration
/// and the toolchain file change what a build runs).
pub const ALWAYS_PROTECTED: [&str; 6] = [
    ".git",
    "target",
    ".env",
    ".cargo",
    "rust-toolchain",
    "rust-toolchain.toml",
];

/// The brief for implementing `element` (a part def or a part).
pub fn brief(
    tree: &Tree,
    links: &Links,
    element: ElementId,
    instructions: &str,
) -> Result<Brief, String> {
    let semantics = Semantics::new(tree);
    let e = tree
        .get(element)
        .ok_or("the element is no longer in the model")?;
    let definition = match e.kind {
        ElementKind::PartDef => element,
        ElementKind::Part => semantics
            .types_of(element)
            .first()
            .map(|(t, _)| *t)
            .filter(|t| tree.contains(*t))
            .ok_or("the part has no definition in the model to implement")?,
        other => {
            return Err(format!(
                "a {} is not implemented on its own; implement the part def it belongs to",
                other.keyword()
            ));
        }
    };
    let name = tree.qualified_name(definition);
    let text_of = |id: ElementId| print_element(tree, id).unwrap_or_default();
    let mut sections = vec![format!(
        "# Task\n\nImplement `{name}` in {} in this repository, as the model below says.{}",
        if links.language.is_empty() {
            "Rust"
        } else {
            links.language.as_str()
        },
        if instructions.trim().is_empty() {
            String::new()
        } else {
            format!("\n\nThe Operator adds: {}", instructions.trim())
        }
    )];
    sections.push(format!(
        "# {name}, as modelled\n\n```sysml\n{}\n```",
        text_of(definition)
    ));
    // The contracts: what its ports carry, and the definitions those use.
    let mut contracts = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut pending: Vec<ElementId> = semantics
        .features(definition)
        .into_iter()
        .filter(|f| {
            semantics
                .element(*f)
                .is_some_and(|e| e.kind == ElementKind::Port)
        })
        .flat_map(|p| semantics.types_of(p).into_iter().map(|(t, _)| t))
        .collect();
    while let Some(id) = pending.pop() {
        if !tree.contains(id) || !seen.insert(id) || seen.len() > 40 {
            continue;
        }
        contracts.push(text_of(id));
        for feature in semantics.features(id) {
            pending.extend(semantics.types_of(feature).into_iter().map(|(t, _)| t));
        }
        for general in semantics.generals(id) {
            pending.push(general);
        }
    }
    if !contracts.is_empty() {
        sections.push(format!(
            "# The contracts it keeps (ports and what they carry)\n\n```sysml\n{}\n```",
            contracts.join("\n\n")
        ));
    }
    // Its parts' definitions, one level down.
    let inner: Vec<String> = semantics
        .features(definition)
        .into_iter()
        .filter(|f| {
            semantics
                .element(*f)
                .is_some_and(|e| e.kind == ElementKind::Part)
        })
        .filter_map(|p| semantics.types_of(p).first().map(|(t, _)| *t))
        .filter(|t| tree.contains(*t) && *t != definition)
        .map(text_of)
        .collect();
    if !inner.is_empty() {
        sections.push(format!(
            "# The parts inside it\n\n```sysml\n{}\n```",
            inner.join("\n\n")
        ));
    }
    // The scenarios that exercise it are its acceptance behaviour.
    let scenarios: Vec<ElementId> = tree
        .walk()
        .into_iter()
        .filter(|id| tree[*id].kind == ElementKind::VerificationDef)
        .filter(|id| closure(tree, *id).contains(&definition))
        .collect();
    if !scenarios.is_empty() {
        sections.push(format!(
            "# The scenarios it must pass\n\nEach runs against the code through the harness: the stand-ins replace the parts they name, the steps put items in and wait for what comes out, and the checks must hold.\n\n```sysml\n{}\n```",
            scenarios.iter().map(|s| text_of(*s)).collect::<Vec<_>>().join("\n\n")
        ));
    }
    // What is already linked.
    let mut linked: Vec<String> = links
        .links
        .iter()
        .filter(|l| {
            let id = l.element();
            id == definition
                || seen.contains(&id)
                || tree.get(id).and_then(|e| e.owner()) == Some(definition)
        })
        .map(|l| {
            format!(
                "- {} {}: {}{}",
                l.name,
                l.kind.label(),
                l.path,
                l.symbol
                    .as_ref()
                    .map(|s| format!("#{s}"))
                    .unwrap_or_default()
            )
        })
        .collect();
    linked.sort();
    linked.dedup();
    if !linked.is_empty() {
        sections.push(format!(
            "# Code already linked to these elements\n\n{}",
            linked.join("\n")
        ));
    }
    if !links.harness.is_empty() {
        sections.push(format!(
            "# The harness\n\nScenarios run through `{}`, which speaks this protocol on standard input and output:\n\n{}",
            links.harness.join(" "),
            crate::harness::PROTOCOL
        ));
    }
    let mut protected: Vec<String> = ALWAYS_PROTECTED.iter().map(|p| p.to_string()).collect();
    protected.extend(links.protected.iter().cloned());
    sections.push(format!(
        "# Rules\n\n- Work only in this repository. These paths are protected and cannot be written: {}.\n- The model is the contract. If it is wrong or not enough to implement, call request_contract_change and stop; do not work around it in code.\n- Link what you write to the model with link_code: the module or file that implements a part, the type that implements an item or enum def, the tests that check them.\n- Call run_checks, fix what fails, and call finish_implementation with an honest summary, including what still fails.",
        protected.join(", ")
    ));
    Ok(Brief {
        element: definition.raw(),
        title: format!(
            "Implement {}",
            tree.effective_name(definition).unwrap_or("?")
        ),
        text: sections.join("\n\n"),
        scenario_names: scenarios
            .iter()
            .map(|s| tree.effective_name(*s).unwrap_or("?").to_string())
            .collect(),
        scenarios: scenarios.into_iter().map(ElementId::raw).collect(),
        protected,
        required: Vec::new(),
    })
}

/// A check a task must pass, fixed when the Operator approves the task
/// (ROADMAP §4.15): the worker can add checks by proposing links, never
/// remove one of these.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum RequiredCheck {
    /// The working copy builds (`cargo build --all-targets`).
    Build,
    /// The linked modules' dependencies against the model's.
    DependencyBoundaries,
    /// One linked type against the item or enum def it implements.
    ContractShape { element: u64, location: String },
    /// One linked test.
    LinkedTest { element: u64, location: String },
    /// One scenario of the brief, run against the code through the harness.
    Scenario { element: u64, name: String },
    /// One of the project's own commands (its required checks).
    Command {
        id: String,
        label: String,
        program: Vec<String>,
    },
}

impl RequiredCheck {
    /// The identity a recorded check is matched by.
    pub fn key(&self) -> String {
        match self {
            RequiredCheck::Build => "build".into(),
            RequiredCheck::DependencyBoundaries => "boundaries".into(),
            RequiredCheck::ContractShape { location, .. } => format!("shape:{location}"),
            RequiredCheck::LinkedTest { location, .. } => format!("test:{location}"),
            RequiredCheck::Scenario { element, .. } => format!("scenario:{element}"),
            RequiredCheck::Command { id, .. } => format!("command:{id}"),
        }
    }

    pub fn label(&self) -> String {
        match self {
            RequiredCheck::Build => "Build".into(),
            RequiredCheck::DependencyBoundaries => CheckKind::DependencyBoundaries.label().into(),
            RequiredCheck::ContractShape { location, .. } => format!("Contract shape {location}"),
            RequiredCheck::LinkedTest { location, .. } => format!("Test {location}"),
            RequiredCheck::Scenario { name, .. } => format!("Scenario {name}"),
            RequiredCheck::Command { label, .. } => label.clone(),
        }
    }
}

/// One of a project's own check commands, kept by the Operator in the
/// project's app data (`checks.json`), never in the repository a worker
/// writes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectCommand {
    /// Stable and short: `lints`, `architecture`.
    pub id: String,
    /// What the Operator reads: "Lints, warnings denied".
    pub label: String,
    /// The program and its arguments, run in the repository's root.
    pub program: Vec<String>,
}

/// A project's required check commands (`checks.json`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectChecks {
    pub format: u32,
    #[serde(default)]
    pub commands: Vec<ProjectCommand>,
}

impl ProjectChecks {
    pub const FORMAT: u32 = 1;

    /// The checks Agentique's own repository requires (`AGENTS.md`): the
    /// format, lints with warnings denied, the workspace's tests, the
    /// architecture check, and the Claude Agent companion's tests.
    pub fn agentique() -> ProjectChecks {
        let command = |id: &str, label: &str, program: &[&str]| ProjectCommand {
            id: id.into(),
            label: label.into(),
            program: program.iter().map(|a| a.to_string()).collect(),
        };
        ProjectChecks {
            format: Self::FORMAT,
            commands: vec![
                command(
                    "format",
                    "Formatting (cargo fmt --check)",
                    &["cargo", "fmt", "--all", "--", "--check"],
                ),
                command(
                    "lints",
                    "Lints, warnings denied (cargo clippy)",
                    &[
                        "cargo",
                        "clippy",
                        "--workspace",
                        "--all-targets",
                        "--offline",
                        "--",
                        "-D",
                        "warnings",
                    ],
                ),
                command(
                    "tests",
                    "The workspace's tests (cargo test)",
                    &["cargo", "test", "--workspace", "--offline"],
                ),
                command(
                    "architecture",
                    "The architecture check (tools/check_architecture.py)",
                    &["python", "tools/check_architecture.py"],
                ),
                command(
                    "companion",
                    "The Claude Agent companion's tests",
                    &[
                        "node",
                        "--experimental-strip-types",
                        "--no-warnings",
                        "--test",
                        "claude-agent/test/*.test.ts",
                    ],
                ),
            ],
        }
    }
}

/// The checks a task on `brief` must pass: the build, every check its links
/// configure (dependency boundaries for linked modules, a contract shape per
/// linked type, each linked test), every scenario of the brief when a
/// harness can run it against the code, and the project's commands. What
/// cannot be checked because it is not configured is said by
/// [`not_configured`] before the Operator approves, and stays visible as not
/// run in the verification: never left out silently.
pub fn required_checks(
    links: &Links,
    brief: &Brief,
    commands: &[ProjectCommand],
) -> Vec<RequiredCheck> {
    use crate::links::LinkKind;
    let mut required = vec![RequiredCheck::Build];
    if links.links.iter().any(|l| l.kind == LinkKind::Module) {
        required.push(RequiredCheck::DependencyBoundaries);
    }
    for link in &links.links {
        if link.symbol.is_none() {
            continue;
        }
        match link.kind {
            LinkKind::Type => required.push(RequiredCheck::ContractShape {
                element: link.element,
                location: link.location(),
            }),
            LinkKind::Test => required.push(RequiredCheck::LinkedTest {
                element: link.element,
                location: link.location(),
            }),
            _ => {}
        }
    }
    if !links.harness.is_empty() {
        for (element, name) in brief.scenarios.iter().zip(&brief.scenario_names) {
            required.push(RequiredCheck::Scenario {
                element: *element,
                name: name.clone(),
            });
        }
    }
    for command in commands {
        required.push(RequiredCheck::Command {
            id: command.id.clone(),
            label: command.label.clone(),
            program: command.program.clone(),
        });
    }
    let mut seen = std::collections::HashSet::new();
    required.retain(|check| seen.insert(check.key()));
    required
}

/// The required checks as the worker reads them in its brief: each must
/// pass for the Operator to integrate the task without saying otherwise.
pub fn checks_section(required: &[RequiredCheck]) -> String {
    let mut lines = vec![
        "# The checks this task must pass

Fixed when the Operator approved the task; `run_checks` runs them all, and the Studio runs them again itself. Each project command can also be run alone with `run_program`, exactly as written here.
".to_string(),
    ];
    for check in required {
        lines.push(match check {
            RequiredCheck::Command { label, program, .. } => {
                format!("- {label}: `{}`", program.join(" "))
            }
            other => format!("- {}", other.label()),
        });
    }
    lines.join(
        "
",
    )
}

/// What a task on `brief` will not be checked for, in plain words, for the
/// Operator to see before approving it.
pub fn not_configured(links: &Links, brief: &Brief) -> Vec<String> {
    use crate::links::LinkKind;
    let mut out = Vec::new();
    if !links.links.iter().any(|l| l.kind == LinkKind::Module) {
        out.push(
            "No modules are linked: dependency boundaries between modules are not checked.".into(),
        );
    }
    if !links.links.iter().any(|l| l.kind == LinkKind::Test) {
        out.push("No tests are linked to the model.".into());
    }
    if !brief.scenarios.is_empty() && links.harness.is_empty() {
        out.push(format!(
            "No harness is linked: the {} scenario(s) cannot run against the code and will stay not run.",
            brief.scenarios.len()
        ));
    }
    out
}

/// What a verification checked: the task commit and the model.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Checked {
    /// The task commit verified.
    pub commit: String,
    /// The digest of the model the brief was taken from ([`brief_digest`]).
    pub model_digest: String,
}

/// One required check with its outcome.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub check: RequiredCheck,
    /// What it is called: the recorded check's own name when it ran.
    pub name: String,
    pub verdict: Verdict,
    pub message: String,
}

/// What verifying a working copy found: every check it ran, in one list,
/// and the required checks fixed at approval. A verification passes only
/// when every required check passed and nothing else failed (§4.15).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", from = "VerificationFile")]
pub struct Verification {
    /// Whether the working copy built.
    pub built: bool,
    #[serde(default)]
    pub build_errors: String,
    /// Every check recorded: boundaries, contract shapes, linked tests,
    /// scenarios against the code and the project's commands.
    pub checks: Vec<ImplementationCheck>,
    /// The required checks fixed at approval. Empty for a verification
    /// made without a task definition: then every recorded check counts as
    /// required.
    #[serde(default)]
    pub required: Vec<RequiredCheck>,
    #[serde(default)]
    pub checked: Option<Checked>,
}

/// The stored form, which also reads verifications written before the
/// required checks existed (their scenarios were kept apart).
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerificationFile {
    #[serde(default)]
    built: bool,
    #[serde(default)]
    build_errors: String,
    #[serde(default)]
    checks: Vec<ImplementationCheck>,
    #[serde(default)]
    scenarios: Vec<(String, Verdict, String)>,
    #[serde(default)]
    required: Vec<RequiredCheck>,
    #[serde(default)]
    checked: Option<Checked>,
}

impl From<VerificationFile> for Verification {
    fn from(file: VerificationFile) -> Verification {
        let mut checks = file.checks;
        for (name, verdict, message) in file.scenarios {
            checks.push(ImplementationCheck {
                kind: CheckKind::Scenario,
                name: format!("Scenario {name}"),
                elements: Vec::new(),
                verdict,
                message,
                details: Vec::new(),
                location: None,
            });
        }
        Verification {
            built: file.built,
            build_errors: file.build_errors,
            checks,
            required: file.required,
            checked: file.checked,
        }
    }
}

impl Verification {
    /// The build's outcome.
    fn build(&self) -> Outcome {
        let refused = self
            .checks
            .iter()
            .find(|c| c.kind == CheckKind::Build)
            .map(|c| c.message.clone());
        let (verdict, message) = if self.built {
            (Verdict::Passed, "it builds".to_string())
        } else if let Some(why) = refused {
            (Verdict::NotRun, why)
        } else if self.build_errors.is_empty() {
            (Verdict::NotRun, "the build did not run".to_string())
        } else {
            (
                Verdict::Failed,
                self.build_errors.lines().next().unwrap_or("").to_string(),
            )
        };
        Outcome {
            check: RequiredCheck::Build,
            name: "Build".into(),
            verdict,
            message,
        }
    }

    /// Each required check with its outcome, in the order fixed at
    /// approval; a required check with no recorded result is not run, with
    /// that reason. Without required checks: the build and every recorded
    /// check.
    pub fn outcomes(&self) -> Vec<Outcome> {
        if self.required.is_empty() {
            let mut out = vec![self.build()];
            out.extend(
                self.checks
                    .iter()
                    .filter(|c| c.kind != CheckKind::Build)
                    .map(|c| Outcome {
                        check: c.required_check(),
                        name: c.name.clone(),
                        verdict: c.verdict,
                        message: c.message.clone(),
                    }),
            );
            return out;
        }
        self.required
            .iter()
            .map(|required| {
                if *required == RequiredCheck::Build {
                    return self.build();
                }
                let key = required.key();
                match self.checks.iter().find(|c| c.required_check().key() == key) {
                    Some(c) => Outcome {
                        check: required.clone(),
                        name: c.name.clone(),
                        verdict: c.verdict,
                        message: c.message.clone(),
                    },
                    None => Outcome {
                        check: required.clone(),
                        name: required.label(),
                        verdict: Verdict::NotRun,
                        message: "no result was recorded for this required check".into(),
                    },
                }
            })
            .collect()
    }

    /// The recorded checks that were not required (such as those the
    /// worker's proposed links add).
    pub fn extra(&self) -> Vec<&ImplementationCheck> {
        if self.required.is_empty() {
            return Vec::new();
        }
        let keys: std::collections::HashSet<String> =
            self.required.iter().map(RequiredCheck::key).collect();
        self.checks
            .iter()
            .filter(|c| c.kind != CheckKind::Build && !keys.contains(&c.required_check().key()))
            .collect()
    }

    /// What failed or could not be verified: every required check that did
    /// not pass, and every other check that failed.
    pub fn failures(&self) -> usize {
        self.outcomes()
            .iter()
            .filter(|o| o.verdict != Verdict::Passed)
            .count()
            + self
                .extra()
                .iter()
                .filter(|c| c.verdict == Verdict::Failed)
                .count()
    }

    /// The one summary (§4.14): passed only when every required check
    /// passed and nothing else failed.
    pub fn verdict(&self) -> Verdict {
        let mut verdicts: Vec<Verdict> = self.outcomes().iter().map(|o| o.verdict).collect();
        verdicts.extend(
            self.extra()
                .iter()
                .filter(|c| c.verdict == Verdict::Failed)
                .map(|c| c.verdict),
        );
        agq_simulation::summary(verdicts)
    }

    pub fn passed(&self) -> bool {
        self.verdict() == Verdict::Passed
    }

    /// Whether the verification still describes the task: the same commit
    /// with nothing uncommitted, and the same model.
    pub fn freshness(
        &self,
        commit: &str,
        uncommitted: bool,
        model_digest: &str,
    ) -> agq_simulation::Freshness {
        let Some(checked) = &self.checked else {
            return agq_simulation::Freshness::Outdated(
                "it does not say which commit it checked".into(),
            );
        };
        if checked.commit != commit || uncommitted {
            return agq_simulation::Freshness::Outdated(
                "the working copy changed after it was checked".into(),
            );
        }
        if checked.model_digest != model_digest {
            return agq_simulation::Freshness::Outdated(
                "the model changed after it was checked".into(),
            );
        }
        agq_simulation::Freshness::Current
    }

    /// In plain words: each required check with its outcome, then the
    /// other checks, then the summary.
    pub fn describe(&self) -> String {
        let mut lines = Vec::new();
        let outcomes = self.outcomes();
        let passed = outcomes
            .iter()
            .filter(|o| o.verdict == Verdict::Passed)
            .count();
        for outcome in &outcomes {
            lines.push(format!(
                "- {}: {}. {}",
                outcome.name,
                outcome.verdict.label(),
                outcome.message
            ));
            if outcome.check == RequiredCheck::Build {
                if !self.built && !self.build_errors.is_empty() {
                    for line in last_lines(&self.build_errors, 12).lines() {
                        lines.push(format!("    {line}"));
                    }
                }
            } else if let Some(check) = self
                .checks
                .iter()
                .find(|c| c.required_check().key() == outcome.check.key())
            {
                for detail in check.details.iter().take(4) {
                    lines.push(format!("    {detail}"));
                }
            }
        }
        let extra = self.extra();
        if !extra.is_empty() {
            lines.push("Also checked (not required):".into());
            for check in extra {
                lines.push(format!(
                    "- {}: {}. {}",
                    check.name,
                    check.verdict.label(),
                    check.message
                ));
            }
        }
        lines.push(match self.verdict() {
            Verdict::Passed => format!(
                "Passed: every one of the {} required check(s) passed.",
                outcomes.len()
            ),
            verdict => format!(
                "Not passed ({}): {passed} of {} required check(s) passed; {} failing or not verified.",
                verdict.label(),
                outcomes.len(),
                self.failures()
            ),
        });
        lines.join("\n")
    }
}

/// Builds the working copy and runs every check: the implementation checks,
/// the brief's scenarios against the code, and the project's commands the
/// brief requires. Nothing runs without trusted-local execution: the
/// executor refuses, and each check says so. Every check ends with an
/// outcome; none is left out because something before it failed.
pub fn verify(
    tree: &Tree,
    links: &Links,
    brief: &Brief,
    executor: &Executor,
    cancel: Arc<AtomicBool>,
) -> Verification {
    let mut verification = Verification {
        required: brief.required.clone(),
        ..Verification::default()
    };
    let root = executor.scope().root().to_path_buf();
    let not_run =
        |kind: CheckKind, name: String, elements: Vec<u64>, why: &str| ImplementationCheck {
            kind,
            name,
            elements,
            verdict: Verdict::NotRun,
            message: why.to_string(),
            details: Vec::new(),
            location: None,
        };
    match executor.run(
        &Program::cargo(&["build", "--offline", "--all-targets"]),
        "",
        Duration::from_secs(900),
    ) {
        Ok(finished) if finished.success => verification.built = true,
        Ok(finished) if finished.cancelled => verification.checks.push(not_run(
            CheckKind::Build,
            "Build".into(),
            Vec::new(),
            "cancelled by the Operator",
        )),
        Ok(finished) => {
            verification.build_errors = format!(
                "{}\n{}",
                finished.summary(),
                last_lines(&finished.stderr, 40)
            );
        }
        Err(refusal) => verification.checks.push(not_run(
            CheckKind::Build,
            "Build".into(),
            Vec::new(),
            &refusal.to_string(),
        )),
    }
    let read = |path: &str| std::fs::read_to_string(root.join(path)).ok();
    verification
        .checks
        .push(module_boundaries(tree, links, &read));
    verification
        .checks
        .extend(contract_shapes(tree, links, &read));
    let built = verification.built;
    let why_not = || -> Option<&'static str> {
        if cancel.load(Ordering::SeqCst) {
            Some("cancelled by the Operator")
        } else if !built {
            Some("the working copy did not build")
        } else {
            None
        }
    };
    if let Some(why) = why_not() {
        for link in links
            .links
            .iter()
            .filter(|l| l.kind == crate::links::LinkKind::Test)
        {
            if let Some(symbol) = &link.symbol {
                let mut check = not_run(
                    CheckKind::LinkedTest,
                    format!("Test {symbol}"),
                    vec![link.element],
                    why,
                );
                check.location = Some(link.location());
                verification.checks.push(check);
            }
        }
    } else {
        verification
            .checks
            .extend(linked_tests(links, executor, "", Duration::from_secs(900)));
    }
    for (scenario, name) in brief.scenarios.iter().zip(&brief.scenario_names) {
        let elements = vec![*scenario];
        let label = format!("Scenario {name}");
        if let Some(why) = why_not() {
            verification
                .checks
                .push(not_run(CheckKind::Scenario, label, elements, why));
            continue;
        }
        if links.harness.is_empty() {
            verification.checks.push(not_run(
                CheckKind::Scenario,
                label,
                elements,
                "no harness is linked, so it cannot run against the code",
            ));
            continue;
        }
        let id = ElementId::from_raw(*scenario);
        let Ok(program) = agq_simulation::compile(tree, id) else {
            let mut check = not_run(
                CheckKind::Scenario,
                label,
                elements,
                "the scenario cannot start in the model",
            );
            check.verdict = Verdict::Blocked;
            verification.checks.push(check);
            continue;
        };
        let result = run_implementation(
            &program,
            model_digest(tree, id),
            &Request::new(Mode::Implementation),
            links,
            &root,
            executor,
            cancel.clone(),
        );
        let verdict = match result.status {
            RunStatus::Completed if result.all_passed() => Verdict::Passed,
            RunStatus::Completed => Verdict::Failed,
            RunStatus::Blocked => Verdict::Blocked,
            RunStatus::Cancelled => Verdict::NotRun,
            _ => Verdict::Failed,
        };
        let what = match (
            &result.stop,
            result.checks.iter().find(|c| c.verdict != Verdict::Passed),
        ) {
            (Some(stop), _) => format!("stopped ({}): {}", stop.reason.code(), stop.message),
            (None, Some(check)) => format!(
                "check {} {}: {}",
                check.name,
                check.verdict.label(),
                check.message
            ),
            (None, None) => "every check passed".into(),
        };
        verification.checks.push(ImplementationCheck {
            kind: CheckKind::Scenario,
            name: label,
            elements,
            verdict,
            message: what,
            details: Vec::new(),
            location: None,
        });
    }
    for required in &brief.required {
        let RequiredCheck::Command { id, label, program } = required else {
            continue;
        };
        let mut check = not_run(CheckKind::Command, label.clone(), Vec::new(), "");
        check.location = Some(id.clone());
        let Some(program) = Program::from_list(program) else {
            check.message = "the command is empty".into();
            verification.checks.push(check);
            continue;
        };
        if cancel.load(Ordering::SeqCst) {
            check.message = "cancelled by the Operator".into();
            verification.checks.push(check);
            continue;
        }
        match executor.run(&program, "", Duration::from_secs(1800)) {
            Err(refusal) => check.message = refusal.to_string(),
            Ok(finished) if finished.cancelled => {
                check.message = "cancelled by the Operator".into();
            }
            Ok(finished) if finished.success => {
                check.verdict = Verdict::Passed;
                check.message = format!("`{}` {}", program.display(), finished.summary());
            }
            Ok(finished) => {
                check.verdict = Verdict::Failed;
                check.message = format!("`{}` {}", program.display(), finished.summary());
                let output = format!("{}\n{}", finished.stdout, finished.stderr);
                check.details = last_lines(output.trim(), 12)
                    .lines()
                    .map(str::to_string)
                    .collect();
            }
        }
        verification.checks.push(check);
    }
    verification
}

/// The digest of what a task's brief was taken from (its element and its
/// scenarios), so a verification is outdated when they change.
pub fn brief_digest(tree: &Tree, brief: &Brief) -> String {
    let mut text = String::new();
    for id in std::iter::once(brief.element).chain(brief.scenarios.iter().copied()) {
        let id = ElementId::from_raw(id);
        if tree.contains(id) {
            text.push_str(&model_digest(tree, id));
        }
    }
    agq_simulation::digest::text_digest(&text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_execution::Scope;

    fn check(kind: CheckKind, verdict: Verdict, message: &str) -> ImplementationCheck {
        ImplementationCheck {
            kind,
            name: format!("{} ({})", kind.label(), verdict.label()),
            elements: Vec::new(),
            verdict,
            message: message.into(),
            details: Vec::new(),
            location: None,
        }
    }

    /// Regression (ROADMAP §5.6 item 1): a verification counted a check
    /// only when it failed, so checks that did not run, could not be
    /// evaluated or could not decide read as success.
    #[test]
    fn a_check_that_did_not_pass_is_never_counted_as_a_pass() {
        let verification = Verification {
            built: true,
            checks: vec![
                check(
                    CheckKind::DependencyBoundaries,
                    Verdict::NotRun,
                    "no modules are linked",
                ),
                check(CheckKind::ContractShape, Verdict::Unsupported, "no symbol"),
                check(CheckKind::LinkedTest, Verdict::Inconclusive, "unreadable"),
                check(CheckKind::LinkedTest, Verdict::Passed, "the test passed"),
            ],
            ..Default::default()
        };
        assert_eq!(verification.failures(), 3, "{}", verification.describe());
    }

    /// The required set is fixed: a required check with no recorded result
    /// is not run, a passing set passes, a failing extra check fails it, and
    /// a verification describes only the commit and model it checked.
    #[test]
    fn required_checks_are_accounted_for_one_by_one() {
        let test = RequiredCheck::LinkedTest {
            element: 7,
            location: "src/lib.rs#works".into(),
        };
        let command = RequiredCheck::Command {
            id: "lints".into(),
            label: "Lints".into(),
            program: vec!["cargo".into(), "clippy".into()],
        };
        let mut passing_test = check(CheckKind::LinkedTest, Verdict::Passed, "the test passed");
        passing_test.elements = vec![7];
        passing_test.location = Some("src/lib.rs#works".into());
        let mut verification = Verification {
            built: true,
            checks: vec![passing_test],
            required: vec![RequiredCheck::Build, test.clone(), command.clone()],
            checked: Some(Checked {
                commit: "abc".into(),
                model_digest: "m1".into(),
            }),
            ..Default::default()
        };
        // The command never ran: not run, with the reason.
        let outcomes = verification.outcomes();
        assert_eq!(outcomes.len(), 3);
        assert_eq!(outcomes[2].check, command);
        assert_eq!(outcomes[2].verdict, Verdict::NotRun);
        assert!(outcomes[2].message.contains("no result was recorded"));
        assert_eq!(verification.verdict(), Verdict::NotRun);
        assert!(!verification.passed());
        assert!(
            verification
                .describe()
                .contains("Not passed (not run): 2 of 3")
        );
        // It ran and passed: the verification passes.
        let mut lints = check(CheckKind::Command, Verdict::Passed, "succeeded");
        lints.location = Some("lints".into());
        verification.checks.push(lints);
        assert!(verification.passed(), "{}", verification.describe());
        // A check the worker's links added fails: not passed.
        verification.checks.push(check(
            CheckKind::ContractShape,
            Verdict::Failed,
            "a field is missing",
        ));
        assert_eq!(verification.verdict(), Verdict::Failed);
        assert!(
            verification
                .describe()
                .contains("Also checked (not required)")
        );
        verification.checks.pop();
        // Freshness: another commit, uncommitted work, or another model.
        use agq_simulation::Freshness;
        assert_eq!(
            verification.freshness("abc", false, "m1"),
            Freshness::Current
        );
        assert!(matches!(
            verification.freshness("abd", false, "m1"),
            Freshness::Outdated(_)
        ));
        assert!(matches!(
            verification.freshness("abc", true, "m1"),
            Freshness::Outdated(_)
        ));
        assert!(matches!(
            verification.freshness("abc", false, "m2"),
            Freshness::Outdated(_)
        ));
        // The build did not run (refused): not a pass, and said so.
        let refused = Verification {
            checks: vec![check(
                CheckKind::Build,
                Verdict::NotRun,
                "needs trusted-local execution",
            )],
            required: vec![RequiredCheck::Build],
            ..Default::default()
        };
        assert_eq!(refused.verdict(), Verdict::NotRun);
        assert!(refused.describe().contains("needs trusted-local execution"));
    }

    /// A verification stored before required checks existed still reads,
    /// its scenarios among the checks, and a scenario that was not run is
    /// not a pass.
    #[test]
    fn an_older_verification_reads_and_is_judged_by_the_same_rule() {
        let stored = serde_json::json!({
            "built": true,
            "buildErrors": "",
            "checks": [],
            "scenarios": [["HeldLink", "notRun", "no harness"]]
        });
        let verification: Verification = serde_json::from_value(stored).unwrap();
        assert_eq!(verification.checks.len(), 1);
        assert_eq!(verification.checks[0].kind, CheckKind::Scenario);
        assert_eq!(verification.failures(), 1);
        // And it writes back in the one shape.
        let written = serde_json::to_value(&verification).unwrap();
        assert!(written.get("scenarios").is_none(), "{written}");
    }

    /// Regression (§5.6 item 1): a scenario of the brief that could not run
    /// against the code (no harness, or no build) was left out of the
    /// verification instead of being reported as not run.
    #[test]
    fn every_scenario_of_the_brief_has_an_explicit_outcome() {
        let dir = tempfile::tempdir().unwrap();
        let tree = agq_language::parse(&[agq_language::Source::new(
            "M.sysml",
            "package M { part def A; verification def S { subject a : A; } }",
        )]);
        let scenario = tree.find("M::S").unwrap();
        let brief = Brief {
            element: tree.find("M::A").unwrap().raw(),
            title: "Implement A".into(),
            text: String::new(),
            scenarios: vec![scenario.raw()],
            scenario_names: vec!["S".into()],
            protected: Vec::new(),
            required: Vec::new(),
        };
        // No harness and no trusted-local execution: nothing can run.
        let executor = Executor::new(Scope::read_only(dir.path()).unwrap());
        let verification = verify(
            &tree,
            &Links::default(),
            &brief,
            &executor,
            Arc::new(AtomicBool::new(false)),
        );
        let text = verification.describe();
        assert!(text.contains("Scenario S"), "{text}");
        assert!(verification.failures() >= 2, "{text}");
    }
}
