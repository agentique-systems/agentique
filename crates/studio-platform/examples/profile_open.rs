//! Real, existing-repository read profiling. No fixture or publication rebuild.
use agq_modeling_view::ViewDefinition;
use agq_studio_platform::{NativeConfig, RevisionBinding};
use std::{path::PathBuf, time::Instant};

fn measured<T>(phase: &str, operation: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let result = operation();
    println!(
        "{}",
        serde_json::json!({"phase": phase, "elapsed_ms": start.elapsed().as_secs_f64() * 1000.0})
    );
    result
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let root = PathBuf::from(
        args.next()
            .ok_or("expected root, runtime directory, existing database")?,
    );
    let runtime = PathBuf::from(args.next().ok_or("expected runtime directory")?);
    let database = PathBuf::from(args.next().ok_or("expected existing database")?);
    if !database.is_absolute() || !database.is_file() || args.next().is_some() {
        return Err("supply an absolute existing database (prefer a SQLite backup)".into());
    }
    let mut config = NativeConfig::for_root(root, Some(runtime))?;
    config.database = database;
    let mut last = Instant::now();
    let mut previous = String::from("before_open");
    let platform = measured("warm_project_open", || {
        agq_studio_platform::open(&config, |phase| {
            println!(
                "{}",
                serde_json::json!({"bootstrap_phase": previous, "elapsed_ms": last.elapsed().as_secs_f64() * 1000.0})
            );
            previous = format!("{phase:?}");
            last = Instant::now();
        })
    })?;
    let project = platform
        .projects()?
        .into_iter()
        .find(|p| p.name == "Agentique")
        .ok_or("Agentique project missing")?;
    let history = platform.history(project.id)?;
    let binding = RevisionBinding {
        project: project.id,
        revision: history
            .branches
            .iter()
            .find(|b| b.id == project.default_branch)
            .ok_or("default branch missing")?
            .head,
    };
    let initial = measured("first_system_projection", || {
        platform.project(binding, &ViewDefinition::architecture())
    })?;
    let graph = measured("graph_overview", || {
        platform.project(binding, &ViewDefinition::semantic_graph())
    })?;
    let owner = graph
        .nodes
        .iter()
        .find(|n| n.name == "ModelingPlatform" && n.semantic_kind == "PartDefinition")
        .ok_or("ModelingPlatform missing")?
        .id;
    let repository = graph
        .nodes
        .iter()
        .find(|n| n.name == "ModelRepository")
        .ok_or("ModelRepository missing")?
        .id;
    let reader = platform.revision_reader(binding)?;
    for (name, definition) in [
        (
            "focused_system",
            ViewDefinition {
                focus: Some(owner),
                ..ViewDefinition::architecture()
            },
        ),
        (
            "focused_graph",
            ViewDefinition {
                focus: Some(owner),
                ..ViewDefinition::semantic_graph()
            },
        ),
        ("requirements", ViewDefinition::requirements()),
    ] {
        let view = measured(name, || platform.project(binding, &definition))?;
        println!(
            "{}",
            serde_json::json!({"view": name, "nodes": view.nodes.len(), "edges": view.edges.len(), "revision": view.revision_id})
        );
        let repeated = measured(&format!("{name}_repeat"), || {
            platform.project(binding, &definition)
        })?;
        assert_eq!(view, repeated);
    }
    let inspection = measured("inspector_first", || reader.inspect(repository))?;
    let repeated = measured("inspector_repeat", || reader.inspect(repository))?;
    assert_eq!(inspection, repeated);
    println!(
        "{}",
        serde_json::json!({"outcome": "passed", "project": binding.project, "revision": binding.revision, "first_nodes": initial.nodes.len(), "contract": "Ordinary runtime authentication, repository restoration and exact revision reads. No UI/layout/GPU/display timing is included; see native journey metrics."})
    );
    Ok(())
}
