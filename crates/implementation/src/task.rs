//! Implementation tasks (ROADMAP §4.15, W8.4): what a task is about, taken
//! from the model and never from names, and the verification of a working
//! copy that both the worker and the Studio run. The Studio trusts the
//! verification it runs itself, never the worker's account of it.

use crate::checks::{contract_shapes, linked_tests, module_boundaries};
use crate::links::Links;
use crate::{ImplementationCheck, run_implementation};
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
    /// Paths the worker may not write.
    pub protected: Vec<String>,
}

/// Paths no task writes, whatever the links say.
pub const ALWAYS_PROTECTED: [&str; 3] = [".git", "target", ".env"];

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
        scenarios: scenarios.into_iter().map(ElementId::raw).collect(),
        protected,
    })
}

/// What verifying a working copy found.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Verification {
    /// `cargo build --all-targets`: whether it built, and the errors.
    pub built: bool,
    #[serde(default)]
    pub build_errors: String,
    pub checks: Vec<ImplementationCheck>,
    /// Scenarios run against the code: (name, verdict, what happened).
    pub scenarios: Vec<(String, Verdict, String)>,
}

impl Verification {
    /// What failed or could not be verified.
    pub fn failures(&self) -> usize {
        usize::from(!self.built)
            + self
                .checks
                .iter()
                .filter(|c| c.verdict == Verdict::Failed)
                .count()
            + self
                .scenarios
                .iter()
                .filter(|(_, v, _)| *v != Verdict::Passed)
                .count()
    }

    /// In plain words.
    pub fn describe(&self) -> String {
        let mut lines = Vec::new();
        if self.built {
            lines.push("Build: it builds.".to_string());
        } else {
            lines.push(format!("Build: it does not build.\n{}", self.build_errors));
        }
        for check in &self.checks {
            lines.push(format!(
                "- {}: {}. {}",
                check.name,
                check.verdict.label(),
                check.message
            ));
            for detail in check.details.iter().take(4) {
                lines.push(format!("    {detail}"));
            }
        }
        for (name, verdict, what) in &self.scenarios {
            lines.push(format!("- Scenario {name}: {}. {what}", verdict.label()));
        }
        lines.push(format!("{} failing or not verified.", self.failures()));
        lines.join("\n")
    }
}

/// Builds the working copy, runs the implementation checks and the brief's
/// scenarios against it. Nothing runs without trusted-local execution: the
/// executor refuses, and the verification says so.
pub fn verify(
    tree: &Tree,
    links: &Links,
    brief: &Brief,
    executor: &Executor,
    cancel: Arc<AtomicBool>,
) -> Verification {
    let mut verification = Verification::default();
    let root = executor.scope().root().to_path_buf();
    match executor.run(
        &Program::cargo(&["build", "--offline", "--all-targets"]),
        "",
        Duration::from_secs(900),
    ) {
        Ok(finished) if finished.success => verification.built = true,
        Ok(finished) => {
            verification.build_errors = last_lines(&finished.stderr, 40);
        }
        Err(refusal) => {
            verification.build_errors = refusal.to_string();
        }
    }
    let read = |path: &str| std::fs::read_to_string(root.join(path)).ok();
    verification
        .checks
        .push(module_boundaries(tree, links, &read));
    verification
        .checks
        .extend(contract_shapes(tree, links, &read));
    if verification.built && !cancel.load(Ordering::SeqCst) {
        verification
            .checks
            .extend(linked_tests(links, executor, "", Duration::from_secs(900)));
    }
    if verification.built && !links.harness.is_empty() {
        for scenario in &brief.scenarios {
            if cancel.load(Ordering::SeqCst) {
                break;
            }
            let scenario = ElementId::from_raw(*scenario);
            let name = tree.effective_name(scenario).unwrap_or("?").to_string();
            let Ok(program) = agq_simulation::compile(tree, scenario) else {
                verification.scenarios.push((
                    name,
                    Verdict::Blocked,
                    "the scenario cannot start in the model".into(),
                ));
                continue;
            };
            let result = run_implementation(
                &program,
                model_digest(tree, scenario),
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
            verification.scenarios.push((name, verdict, what));
        }
    }
    verification
}
