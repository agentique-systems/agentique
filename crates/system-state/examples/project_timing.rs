//! Times the URL shortener (Scenario A) as a project (ROADMAP §3.3,
//! "performance is part of correctness"): opening it (parse, identities,
//! validation), one change saved, a checkpoint, and the "what changed" view.
//!
//! `cargo run --release -p agq-system-state --example project_timing`
use agq_system_state::{Actor, Change, Operation, Project, compare};
use std::fs;
use std::time::{Duration, Instant};

const RUNS: usize = 21;
const MODEL: &str = include_str!("../../../models/url-shortener/UrlShortener.sysml");

fn timed<T>(f: impl FnOnce() -> T) -> (Duration, T) {
    let start = Instant::now();
    let value = f();
    (start.elapsed(), value)
}

fn median(mut times: Vec<Duration>) -> Duration {
    times.sort();
    times[times.len() / 2]
}

fn report(label: &str, time: Duration) {
    println!("{label:<48} {:8.2} ms", time.as_secs_f64() * 1000.0);
}

fn main() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("url-shortener");
    fs::create_dir_all(folder.join("model")).unwrap();
    fs::write(folder.join("model/UrlShortener.sysml"), MODEL).unwrap();

    let (first, project) = timed(|| Project::open(&folder).unwrap());
    let tree = project.state().tree();
    println!(
        "URL shortener: {} elements, {} diagnostics",
        tree.len(),
        project.state().diagnostics().len()
    );
    report("first open (no identity file yet)", first);
    drop(project);
    let opens = (0..RUNS)
        .map(|_| timed(|| Project::open(&folder).unwrap()).0)
        .collect();
    report("open: read, parse, identities, validate", median(opens));

    let mut project = Project::open(&folder).unwrap();
    let store = project
        .state()
        .tree()
        .find("UrlShortener::LinkStore")
        .unwrap();
    let rename = |project: &mut Project, run: usize| {
        let name = if run.is_multiple_of(2) {
            "LinkRepository"
        } else {
            "LinkStore"
        };
        let operation = Operation::Rename {
            element: store,
            name: name.into(),
        };
        project
            .apply(Change::new(Actor::Operator, "Rename", vec![operation]))
            .unwrap();
    };
    let applies = (0..RUNS)
        .map(|run| timed(|| rename(&mut project, run)).0)
        .collect();
    report("apply one rename and save", median(applies));

    let checkpoints = (0..RUNS)
        .map(|run| {
            rename(&mut project, run);
            timed(|| project.checkpoint("Rename the link store").unwrap()).0
        })
        .collect();
    report("checkpoint", median(checkpoints));

    let list = project.checkpoints().unwrap();
    let (diff, _) = timed(|| {
        let before = project.tree_at(&list[1].id).unwrap();
        compare(&before, project.state().tree())
    });
    report("tree_at + compare (what changed)", diff);
}
