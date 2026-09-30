//! Results kept in the app's data, runs on a background thread, and the
//! time a model run takes (a budget for the run path, ROADMAP §3.3).
use agq_language::{Source, parse};
use agq_simulation::digest::model_digest;
use agq_simulation::{
    Answers, BackgroundRun, Mode, Request, RunStatus, RunStore, compile, freshness, run,
};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

const SHORTENER: &str = include_str!("../../../models/link-screening/UrlShortener.sysml");

#[test]
fn results_are_saved_listed_and_reloaded() {
    let tree = parse(&[Source::new("UrlShortener.sysml", SHORTENER)]);
    let id = tree.find("UrlShortener::ReviewRequired").unwrap();
    let program = compile(&tree, id).unwrap();
    let result = run(
        &program,
        model_digest(&tree, id),
        &Request::new(Mode::Model),
        Answers::StandIns,
        Arc::new(AtomicBool::new(false)),
    );
    let folder = tempfile::tempdir().unwrap();
    let store = RunStore::new(folder.path().join("runs"));
    assert!(store.list().is_empty());
    store.save(&result).unwrap();
    let list = store.list();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].scenario_name, "ReviewRequired");
    assert!(list[0].all_passed);
    let again = store.load(&result.id).unwrap();
    assert_eq!(again, result);
    assert_eq!(store.latest(id.raw(), Mode::Model).unwrap().id, result.id);
    assert!(store.latest(id.raw(), Mode::Replay).is_none());
    // Reopened later, it is still current until the model changes.
    assert!(freshness(&again, &tree).is_current());
    assert!(store.load("../escape").is_none());
    store.delete(&result.id).unwrap();
    assert!(store.list().is_empty());
}

#[test]
fn a_background_run_delivers_its_result_and_can_be_cancelled() {
    let tree = parse(&[Source::new("UrlShortener.sysml", SHORTENER)]);
    let id = tree.find("UrlShortener::ShortenAllowed").unwrap();
    let program = compile(&tree, id).unwrap();
    let mut background = BackgroundRun::start(
        program.clone(),
        model_digest(&tree, id),
        Request::new(Mode::Model),
        Answers::StandIns,
    );
    let started = Instant::now();
    let result = loop {
        if let Some(result) = background.try_result() {
            break result;
        }
        assert!(started.elapsed() < Duration::from_secs(10));
        std::thread::yield_now();
    };
    assert_eq!(result.status, RunStatus::Completed);
    assert!(!background.running());
    assert!(background.try_result().is_none(), "delivered once");
    let cancelled = BackgroundRun::start(
        program,
        model_digest(&tree, id),
        Request::new(Mode::Model),
        Answers::StandIns,
    );
    cancelled.cancel();
    drop(cancelled);
}

/// Budget for the run path: compiling and running one Scenario I scenario
/// in model execution. Measured in debug builds here with a generous
/// ceiling; the release measurement is in `docs/stages.md`.
#[test]
fn a_scenario_compiles_and_runs_within_its_budget() {
    let tree = parse(&[Source::new("UrlShortener.sysml", SHORTENER)]);
    let id = tree.find("UrlShortener::ReviewRequired").unwrap();
    let started = Instant::now();
    let rounds = 20;
    for _ in 0..rounds {
        let program = compile(&tree, id).unwrap();
        let result = run(
            &program,
            model_digest(&tree, id),
            &Request::new(Mode::Model),
            Answers::StandIns,
            Arc::new(AtomicBool::new(false)),
        );
        assert!(result.all_passed());
    }
    let each = started.elapsed() / rounds;
    println!("compile and run ReviewRequired: {each:?} each");
    let ceiling = if cfg!(debug_assertions) { 250 } else { 25 };
    assert!(
        each < Duration::from_millis(ceiling),
        "{each:?} over {ceiling} ms"
    );
}
