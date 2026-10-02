//! Dogfood (Scenario I, C-20; Scenario C, C-51): Agentique's own
//! architecture (`model/`, the repository's own project model), read by its
//! own language core, checked against its own crate graph by its own
//! implementation check (the rule `tools/check_architecture.py` has enforced
//! in CI since Stage 0, R-15), and its workflows run in its own model
//! execution.
use agq_execution::{Executor, Program, Scope};
use agq_implementation::checks::crate_boundaries;
use agq_language::{Source, parse, validate};
use agq_simulation::Verdict;
use std::path::Path;
use std::time::Duration;

fn self_model() -> agq_language::Tree {
    let folder = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../model");
    let mut sources = Vec::new();
    for entry in std::fs::read_dir(&folder).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "sysml") {
            let text = std::fs::read_to_string(&path).unwrap();
            sources.push(Source::new(
                path.file_name().unwrap().to_string_lossy(),
                text,
            ));
        }
    }
    parse(&sources)
}

#[test]
fn agentiques_own_model_is_valid_under_its_own_core() {
    let tree = self_model();
    let problems: Vec<String> = validate(&tree)
        .into_iter()
        .map(|d| {
            format!(
                "{}: {} ({})",
                tree.qualified_name(d.element),
                d.message,
                d.code
            )
        })
        .collect();
    assert_eq!(problems, [] as [String; 0]);
}

#[test]
fn agentiques_crates_follow_its_self_model() {
    let tree = self_model();
    let crate_def = tree.find("AgentiqueArchitecture::Crate").unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let executor = Executor::new(Scope::read_only(&root).unwrap()).trusted(true);
    let metadata = executor
        .run(
            &Program::cargo(&[
                "metadata",
                "--format-version",
                "1",
                "--no-deps",
                "--offline",
            ]),
            "",
            Duration::from_secs(120),
        )
        .unwrap();
    assert!(metadata.success, "{}", metadata.stderr);
    let metadata: serde_json::Value = serde_json::from_str(&metadata.stdout).unwrap();
    let check = crate_boundaries(&tree, crate_def, &metadata);
    assert_eq!(check.verdict, Verdict::Passed, "{:#?}", check.details);
    assert!(check.message.contains("crates in"), "{}", check.message);
    // A deliberate contradiction: Simulation using the System State.
    let mut broken = metadata.clone();
    for package in broken["packages"].as_array_mut().unwrap() {
        if package["name"] == "agq-simulation" {
            package["dependencies"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::json!({"name": "agq-system-state", "kind": null}));
        }
    }
    let check = crate_boundaries(&tree, crate_def, &broken);
    assert_eq!(check.verdict, Verdict::Failed);
    assert!(
        check.details[0]
            .contains("`agq-simulation` (Simulation) depends on `agq-system-state` (SystemState)"),
        "{:#?}",
        check.details
    );
    let simulation = tree.find("AgentiqueArchitecture::Simulation").unwrap();
    assert_eq!(check.elements, [simulation.raw()]);
}

/// The workflows of ROADMAP §2.8 C2 (an ordinary edit, an Assistant action,
/// a development task) and their failure paths run in model execution, and
/// every check holds: the model's account of the workflows is consistent.
/// (The code is checked by the tests linked to each step.)
#[test]
fn agentiques_workflows_run_in_its_own_model_execution() {
    use agq_simulation::{Answers, Mode, Request, RunStatus, compile, digest::model_digest, run};
    use std::sync::Arc;
    use std::sync::atomic::AtomicBool;
    let tree = self_model();
    let scenarios: Vec<_> = tree
        .walk()
        .into_iter()
        .filter(|id| tree[*id].kind == agq_language::ElementKind::VerificationDef)
        .collect();
    assert!(scenarios.len() >= 8, "{} scenarios", scenarios.len());
    let mut failures = Vec::new();
    for scenario in scenarios {
        let name = tree.qualified_name(scenario);
        let program = match compile(&tree, scenario) {
            Ok(program) => program,
            Err(blockers) => {
                failures.push(format!("{name} cannot start: {blockers:?}"));
                continue;
            }
        };
        let result = run(
            &program,
            model_digest(&tree, scenario),
            &Request::new(Mode::Model),
            Answers::StandIns,
            Arc::new(AtomicBool::new(false)),
        );
        if result.status != RunStatus::Completed || !result.all_passed() {
            failures.push(result.describe(None, 30));
        }
    }
    assert!(
        failures.is_empty(),
        "{}",
        failures.join(
            "

"
        )
    );
}
