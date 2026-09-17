use agq_application::{Action, Actor, Application, Command, Operation};
use agq_model::*;
use agq_simulation::*;
use serde_json::{Value as Json, json};
use std::{collections::BTreeMap, time::Instant};
fn act(a: &mut Application, action: Action) -> Json {
    a.command(
        Actor::Operator,
        Command {
            command_id: new_id(),
            project_id: a.project_id.clone(),
            base_revision_id: a.head.id.clone(),
            payload: Operation::Act {
                action,
                approval_id: None,
            },
        },
    )
    .unwrap()
}
fn control(a: &mut Application, run: &str, operation: &str) -> f64 {
    let version = a.run(run).unwrap().control_version;
    let start = Instant::now();
    act(
        a,
        Action::ControlRun {
            run_id: run.into(),
            expected_control_version: version,
            operation: operation.into(),
        },
    );
    start.elapsed().as_secs_f64() * 1000.0
}
fn control_during_step(app: &mut Application, run: &str, operation: &str) -> f64 {
    let locked = std::sync::Mutex::new(app);
    let (tx, rx) = std::sync::mpsc::sync_channel(0);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let mut app = locked.lock().unwrap();
            tx.send(()).unwrap();
            app.tick(run).unwrap();
        });
        rx.recv().unwrap();
        let start = Instant::now();
        control(&mut locked.lock().unwrap(), run, operation);
        start.elapsed().as_secs_f64() * 1000.0
    })
}
fn main() {
    let workspace = std::env::args()
        .nth(1)
        .expect("Supply a fresh benchmark database path");
    let mut source = String::from(
        "package Benchmark { item def Tick; state def Toggle { port input; entry; then low; state low; state high; transition up first low accept Tick via input then high; transition down first high accept Tick via input then low; }\n",
    );
    for i in 0..990 {
        source.push_str(&format!("part def Component{i};\n"));
    }
    source.push('}');
    let sources = BTreeMap::from([("benchmark.sysml".into(), source)]);
    let bytes = sources.values().map(|s| s.len()).sum::<usize>();
    let start = Instant::now();
    let store = agq_storage::SqliteStore::open(std::path::Path::new(&workspace)).unwrap();
    let mut app = Application::open(Box::new(store), sources).unwrap();
    let prepare_workspace_ms = start.elapsed().as_secs_f64() * 1000.0;
    let user_elements = app
        .head
        .model
        .elements
        .values()
        .filter(|e| !e.library && !e.is_implied)
        .count();
    let library_elements = app
        .head
        .model
        .elements
        .values()
        .filter(|e| e.library)
        .count();
    let s = Scenario {
        execution_contract: CONTRACT.into(),
        selected_behaviour_path: "Benchmark::Toggle".into(),
        inputs: (1..=10000)
            .map(|ordinal| Input {
                ordinal,
                receiver_path: "Benchmark::Toggle.input".into(),
                payload_type: "Benchmark::Tick".into(),
                values: BTreeMap::new(),
            })
            .collect(),
        stop_when_active_state: None,
        bindings: BTreeMap::new(),
        limits: Limits {
            max_input_deliveries: 10000,
            max_semantic_steps: 10000,
            max_trace_records: 30000,
            max_memory_bytes: 128 * 1024 * 1024,
        },
        expected: None,
    };
    let scenario_bytes = serde_json::to_vec(&s).unwrap().len();
    let saved = act(&mut app, Action::SaveScenario { scenario: s });
    let start = Instant::now();
    let prepared = act(
        &mut app,
        Action::PrepareRun {
            scenario_revision_id: saved["id"].as_str().unwrap().into(),
        },
    );
    let prepare_run_ms = start.elapsed().as_secs_f64() * 1000.0;
    let run = prepared["prepared_run_id"].as_str().unwrap();
    control(&mut app, run, "initialise");
    control(&mut app, run, "run");
    let mut times = vec![];
    let mut pause_ms = 0.0;
    let mut contended_pause_ms = 0.0;
    let start = Instant::now();
    for i in 0..10000 {
        let step = Instant::now();
        let more = app.tick(run).unwrap();
        times.push(step.elapsed().as_secs_f64() * 1000.0);
        if i == 20 {
            pause_ms = control(&mut app, run, "pause");
            control(&mut app, run, "run");
        }
        if i == 9000 {
            contended_pause_ms = control_during_step(&mut app, run, "pause");
            control(&mut app, run, "run");
        }
        if !more {
            break;
        }
        if i % 1000 == 0 {
            eprintln!("benchmark: committed {} steps", i + 1)
        }
    }
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    let result = app.run(run).unwrap();
    let reset = act(
        &mut app,
        Action::ControlRun {
            run_id: run.into(),
            expected_control_version: result.control_version,
            operation: "reset_as_new_run".into(),
        },
    );
    let stop_run = reset["run_id"].as_str().unwrap();
    control(&mut app, stop_run, "initialise");
    control(&mut app, stop_run, "run");
    app.tick(stop_run).unwrap();
    let stop_ms = control_during_step(&mut app, stop_run, "stop");
    times.sort_by(f64::total_cmp);
    let p95 = times[(times.len() * 95 / 100).min(times.len() - 1)];
    let max = *times.last().unwrap();
    let mean = times.iter().sum::<f64>() / times.len() as f64;
    println!("{}",serde_json::to_string_pretty(&json!({"engine_build":agq_application::BUILD_ID,"library_digest":app.head.model.library_digest,"source_bytes":bytes,"scenario_bytes":scenario_bytes,"user_elements":user_elements,"indexed_library_elements":library_elements,"prepare_workspace_ms":prepare_workspace_ms,"prepare_run_ms":prepare_run_ms,"duration_ms":elapsed_ms,"committed_semantic_steps":result.semantic_steps,"trace_records":result.trace.len(),"step_ms":{"mean":mean,"p95":p95,"max":max},"pause_ack_ms":pause_ms,"contended_pause_ack_ms":contended_pause_ms,"contended_stop_ack_ms":stop_ms,"pause_safe_point_bound_ms":max+pause_ms,"stop_safe_point_bound_ms":max+stop_ms,"stop_reason":result.stop_reason,"trace_digest":result.trace_digest(),"measurement":"release binary, SQLite FULL synchronous; actual pause/stop requested while a background step holds the application mutex; safe-point bounds are separately derived from longest observed step"})).unwrap());
}
