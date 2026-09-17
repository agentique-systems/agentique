use agq_application::{Action, Actor, Application, Command, Operation};
use agq_model::*;
use clap::{Parser, Subcommand};
use serde_json::{Value as Json, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
#[derive(Parser)]
#[command(version, about = "Agentique headless modelling and simulation")]
struct Args {
    #[arg(long, default_value = ".workspaces/local.db")]
    workspace: PathBuf,
    #[arg(long, default_value = "models")]
    models: PathBuf,
    #[command(subcommand)]
    command: Cmd,
}
#[derive(Subcommand)]
enum Cmd {
    Validate {
        paths: Vec<PathBuf>,
    },
    Inspect {
        #[arg(long)]
        revision: Option<String>,
        #[arg(long)]
        element: Option<String>,
    },
    Command {
        file: PathBuf,
        #[arg(long)]
        assistant: bool,
    },
    Rename {
        element: String,
        name: String,
    },
    Move {
        element: String,
        owner: String,
    },
    Prepare {
        scenario: PathBuf,
    },
    Control {
        run: String,
        operation: String,
        #[arg(long)]
        version: Option<u64>,
    },
    Trace {
        run: String,
        #[arg(long, default_value_t = 0)]
        from: usize,
    },
    Export {
        output: PathBuf,
        #[arg(long)]
        revision: Option<String>,
        /// Export a new directory of standard text plus private identity metadata.
        #[arg(long)]
        text: bool,
    },
    Import {
        input: PathBuf,
    },
    Simulate {
        scenario: PathBuf,
    },
    Demo,
    Status,
}
fn read(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).map_err(|e| Error::new("io_error", e.to_string()))
}
fn read_text_project(root: &Path) -> Result<BTreeMap<String, String>> {
    fn visit(root: &Path, dir: &Path, out: &mut BTreeMap<String, String>) -> Result<()> {
        for entry in std::fs::read_dir(dir).map_err(|e| Error::new("io_error", e.to_string()))? {
            let e = entry.map_err(|e| Error::new("io_error", e.to_string()))?;
            let kind = e
                .file_type()
                .map_err(|e| Error::new("io_error", e.to_string()))?;
            if kind.is_symlink() {
                return Err(Error::new(
                    "invalid_source",
                    "Text project symlinks are not followed",
                ));
            }
            if kind.is_dir() {
                visit(root, &e.path(), out)?;
            } else {
                let name = e
                    .path()
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                if out.len() >= 2000 {
                    return Err(Error::new(
                        "resource_limit",
                        "Text project exceeds 2000 files",
                    ));
                }
                let text = read(&e.path())?;
                if text.len() > 8 * 1024 * 1024
                    || out.values().map(|s| s.len()).sum::<usize>() + text.len() > 64 * 1024 * 1024
                {
                    return Err(Error::new(
                        "resource_limit",
                        "Text project exceeds size limits",
                    ));
                }
                out.insert(name, text);
            }
        }
        Ok(())
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files)?;
    Ok(files)
}
fn sources(paths: &[PathBuf]) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    for p in paths {
        if p.is_dir() {
            for entry in std::fs::read_dir(p).map_err(|e| Error::new("io_error", e.to_string()))? {
                let p = entry
                    .map_err(|e| Error::new("io_error", e.to_string()))?
                    .path();
                if p.extension().is_some_and(|s| s == "sysml" || s == "kerml") {
                    out.insert(
                        format!("models/{}", p.file_name().unwrap().to_string_lossy()),
                        read(&p)?,
                    );
                }
            }
        } else {
            out.insert(
                p.file_name().unwrap().to_string_lossy().to_string(),
                read(p)?,
            );
        }
    }
    Ok(out)
}
fn send(app: &mut Application, payload: Operation) -> Result<Json> {
    app.command(
        Actor::Operator,
        Command {
            command_id: new_id(),
            project_id: app.project_id.clone(),
            base_revision_id: app.head.id.clone(),
            payload,
        },
    )
}
fn act(app: &mut Application, action: Action) -> Result<Json> {
    send(
        app,
        Operation::Act {
            action,
            approval_id: None,
        },
    )
}
fn prep(app: &mut Application, scenario: agq_simulation::Scenario) -> Result<String> {
    let s = act(app, Action::SaveScenario { scenario })?;
    let r = act(
        app,
        Action::PrepareRun {
            scenario_revision_id: s["id"].as_str().unwrap().into(),
        },
    )?;
    Ok(r["prepared_run_id"].as_str().unwrap().into())
}
fn ctrl(app: &mut Application, id: &str, operation: &str) -> Result<Json> {
    let version = app.run(id)?.control_version;
    act(
        app,
        Action::ControlRun {
            run_id: id.into(),
            expected_control_version: version,
            operation: operation.into(),
        },
    )
}
fn main() {
    if let Err(e) = execute() {
        eprintln!("{}", serde_json::to_string_pretty(&e).unwrap());
        std::process::exit(1)
    }
}
fn execute() -> Result<()> {
    let args = Args::parse();
    if let Cmd::Validate { paths } = &args.command {
        let m = agq_semantics::compile(
            sources(if paths.is_empty() {
                std::slice::from_ref(&args.models)
            } else {
                paths
            })?,
            &BTreeMap::new(),
        );
        println!("{}",serde_json::to_string_pretty(&json!({"accepted":m.accepted(),"elements":m.elements.values().filter(|e|!e.library).count(),"diagnostics":m.diagnostics,"library_digest":m.library_digest})).unwrap());
        return if m.accepted() {
            Ok(())
        } else {
            Err(Error::diagnostics("invalid_model", m.diagnostics))
        };
    }
    if let Cmd::Simulate { scenario } = &args.command {
        let revision = agq_workspace::Revision::import(
            sources(std::slice::from_ref(&args.models))?,
            &BTreeMap::new(),
        )?;
        let model = &revision.model;
        let scenario: agq_simulation::Scenario = serde_json::from_str(&read(scenario)?)
            .map_err(|e| Error::new("invalid_scenario", e.to_string()))?;
        let plan = agq_simulation::prepare(
            model,
            &revision.id,
            &json_digest(&scenario),
            &scenario,
            agq_application::BUILD_ID,
        )?;
        let mut run = agq_simulation::Run::new(plan);
        run.execute()?;
        println!("{}", serde_json::to_string_pretty(&run).unwrap());
        return Ok(());
    }
    if let Cmd::Import { input } = &args.command {
        if args.workspace.exists() {
            return Err(Error::new(
                "workspace_exists",
                "Import creates a new workspace; choose a new --workspace path",
            ));
        }
        let revision = if input.is_dir() {
            agq_workspace::import_text(read_text_project(input)?)?
        } else {
            let bytes = std::fs::read(input).map_err(|e| Error::new("io_error", e.to_string()))?;
            agq_workspace::import_kpar(&bytes)?
        };
        let store = agq_storage::SqliteStore::open(&args.workspace)?;
        let app = Application::import(Box::new(store), revision)?;
        let imported = json!({"head":app.head.id,"project_id":app.project_id});
        println!("{imported}");
        return Ok(());
    }
    let store = agq_storage::SqliteStore::open(&args.workspace)?;
    let mut app = Application::open_session(
        Box::new(store),
        sources(std::slice::from_ref(&args.models))?,
        false,
    )?;
    let result = match args.command {
        Cmd::Inspect { revision, element } => app.inspect(
            revision.as_deref().unwrap_or(&app.head.id),
            element.as_deref(),
        )?,
        Cmd::Status => app.snapshot()?,
        Cmd::Command { file, assistant } => {
            let c = serde_json::from_str(&read(&file)?)
                .map_err(|e| Error::new("invalid_command", e.to_string()))?;
            app.command(
                if assistant {
                    Actor::Assistant
                } else {
                    Actor::Operator
                },
                c,
            )?
        }
        Cmd::Rename { element, name } => {
            let id = agq_semantics::locate(&app.head.model, &element)?
                .last()
                .unwrap()
                .clone();
            let proposal = send(
                &mut app,
                Operation::ProposeChange {
                    edits: vec![agq_workspace::Edit::Rename {
                        element_id: id,
                        name,
                    }],
                },
            )?;
            act(
                &mut app,
                Action::CommitChange {
                    proposal_id: proposal["proposal_id"].as_str().unwrap().into(),
                },
            )?
        }
        Cmd::Move { element, owner } => {
            let id = agq_semantics::locate(&app.head.model, &element)?
                .last()
                .unwrap()
                .clone();
            let owner = agq_semantics::locate(&app.head.model, &owner)?
                .last()
                .unwrap()
                .clone();
            let proposal = send(
                &mut app,
                Operation::ProposeChange {
                    edits: vec![agq_workspace::Edit::Move {
                        element_id: id,
                        new_owner_id: owner,
                    }],
                },
            )?;
            act(
                &mut app,
                Action::CommitChange {
                    proposal_id: proposal["proposal_id"].as_str().unwrap().into(),
                },
            )?
        }
        Cmd::Prepare { scenario } => {
            let scenario = serde_json::from_str(&read(&scenario)?)
                .map_err(|e| Error::new("invalid_scenario", e.to_string()))?;
            json!({"run_id":prep(&mut app,scenario)?})
        }
        Cmd::Control {
            run,
            operation,
            version,
        } => {
            let expected_control_version = version.unwrap_or(app.run(&run)?.control_version);
            let result = act(
                &mut app,
                Action::ControlRun {
                    run_id: run.clone(),
                    expected_control_version,
                    operation: operation.clone(),
                },
            )?;
            if operation == "run" {
                while app.tick(&run)? {}
            }
            result
        }
        Cmd::Trace { run, from } => app.read_run(&run, from, 1000)?,
        Cmd::Export {
            output,
            revision,
            text,
        } => {
            if output.exists() {
                return Err(Error::new(
                    "file_exists",
                    "Export never silently overwrites a file",
                ));
            }
            let revision = app.revision(revision.as_deref().unwrap_or(&app.head.id))?;
            if text {
                let files = agq_workspace::export_text(&revision)?;
                std::fs::create_dir(&output).map_err(|e| Error::new("io_error", e.to_string()))?;
                for (name, source) in &files {
                    let target = output.join(name);
                    std::fs::create_dir_all(target.parent().unwrap())
                        .map_err(|e| Error::new("io_error", e.to_string()))?;
                    use std::io::Write;
                    let mut file = std::fs::OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .open(target)
                        .map_err(|e| Error::new("io_error", e.to_string()))?;
                    file.write_all(source.as_bytes())
                        .and_then(|_| file.sync_all())
                        .map_err(|e| Error::new("io_error", e.to_string()))?;
                }
                json!({"path":output,"format":"standard-text-with-private-identity-sidecar","source_digest":revision.model.source_digest()})
            } else {
                let bytes = agq_workspace::export_kpar(&revision)?;
                use std::io::Write;
                let mut file = std::fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&output)
                    .map_err(|e| Error::new("io_error", e.to_string()))?;
                file.write_all(&bytes)
                    .and_then(|_| file.sync_all())
                    .map_err(|e| Error::new("io_error", e.to_string()))?;
                json!({"path":output,"sha256":digest(bytes)})
            }
        }
        Cmd::Demo => {
            let accepted: agq_simulation::Scenario =
                serde_json::from_str(include_str!("../../../scenarios/accepted.json")).unwrap();
            let rejected =
                serde_json::from_str(include_str!("../../../scenarios/rejected.json")).unwrap();
            let a = prep(&mut app, accepted.clone())?;
            ctrl(&mut app, &a, "initialise")?;
            while !app.run(&a)?.status.terminal() {
                ctrl(&mut app, &a, "step")?;
            }
            let b = prep(&mut app, accepted.clone())?;
            ctrl(&mut app, &b, "initialise")?;
            ctrl(&mut app, &b, "run")?;
            while app.tick(&b)? {}
            let r = prep(&mut app, rejected)?;
            ctrl(&mut app, &r, "initialise")?;
            ctrl(&mut app, &r, "run")?;
            while app.tick(&r)? {}
            let mut exhausted = accepted;
            exhausted.inputs.truncate(1);
            let e = prep(&mut app, exhausted)?;
            ctrl(&mut app, &e, "initialise")?;
            ctrl(&mut app, &e, "run")?;
            while app.tick(&e)? {}
            let equal = app.run(&a)?.trace_digest() == app.run(&b)?.trace_digest();
            if !equal {
                return Err(Error::new(
                    "verification_failed",
                    "Manual and continuous traces differ",
                ));
            }
            json!({"manual":app.read_run(&a,0,100)?,"continuous":app.read_run(&b,0,100)?,"rejected":app.read_run(&r,0,100)?,"exhausted":app.read_run(&e,0,100)?,"normalized_traces_equal":equal})
        }
        Cmd::Validate { .. } | Cmd::Simulate { .. } | Cmd::Import { .. } => unreachable!(),
    };
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
    Ok(())
}
