use super::*;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "agq-frontier-tests-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run(
    snapshot: &Snapshot,
    session: Option<Arc<PublicationFrontierSession>>,
    mut progress: impl FnMut(&PublicationStage),
) -> Result<PublicationClosure, PublicationOverlayError> {
    close_result_structure(
        snapshot,
        PublicationClosureOptions {
            frontier_checkpoints: session,
            ..Default::default()
        },
        |overlay| {
            SemanticContext::for_overlay(
                overlay,
                SemanticOptions {
                    baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V8,
                    ..Default::default()
                },
                BTreeSet::new(),
            )
            .map_err(PublicationOverlayError::Context)
        },
        |_, _, _, _| {},
        &mut progress,
    )
}

#[test]
fn structural_checkpoint_resume_is_exact_including_queries_and_transport() {
    let snapshot = reference_value_fixture(false);
    let uninterrupted = run(&snapshot, None, |_| {}).unwrap();
    assert!(uninterrupted.converged);
    let directory = Directory::new();
    let session = Arc::new(PublicationFrontierSession::create(&directory.0, [3; 32], 0).unwrap());
    let stopped = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run(&snapshot, Some(session.clone()), |stage| {
            if stage.stratum == ResultStructureStratum::ContextualBindings {
                panic!("external interruption after durable Structural frontier");
            }
        })
        .unwrap();
    }));
    assert!(stopped.is_err());
    let (journal, pin) = session
        .latest_checkpoint()
        .unwrap()
        .expect("Structural frontier committed");
    let resumed = Arc::new(PublicationFrontierSession::resume(&journal, pin, [3; 32], 0).unwrap());
    let actual = run(&snapshot, Some(resumed.clone()), |_| {}).unwrap();
    compare(&uninterrupted, &actual, None);
    uninterrupted
        .certificate
        .as_ref()
        .unwrap()
        .assert_exact(actual.certificate.as_ref().unwrap());
    assert_eq!(
        uninterrupted
            .certificate
            .as_ref()
            .unwrap()
            .revalidation_digest(),
        actual.certificate.as_ref().unwrap().revalidation_digest()
    );
    assert_eq!(resumed.statistics().restored_invocations, 1);
    assert_eq!(resumed.statistics().restored_completed_invocations, 0);
    assert!(resumed.statistics().skipped_rounds > 0);
}

#[test]
fn completed_invocation_restores_without_producer_replay() {
    let snapshot = reference_value_fixture(true);
    let directory = Directory::new();
    let session = Arc::new(PublicationFrontierSession::create(&directory.0, [4; 32], 0).unwrap());
    let original = run(&snapshot, Some(session.clone()), |_| {}).unwrap();
    let (journal, pin) = session.latest_checkpoint().unwrap().unwrap();
    let resumed = Arc::new(PublicationFrontierSession::resume(journal, pin, [4; 32], 0).unwrap());
    let restored = run(&snapshot, Some(resumed.clone()), |_| {
        panic!("completed invocation replayed")
    })
    .unwrap();
    compare(&original, &restored, None);
    original
        .certificate
        .as_ref()
        .unwrap()
        .assert_exact(restored.certificate.as_ref().unwrap());
    assert_eq!(resumed.statistics().restored_completed_invocations, 1);
    assert_eq!(resumed.statistics().committed_checkpoints, 0);
}

#[test]
fn construction_stratum_resume_and_prior_invocation_restore_share_exact_semantics() {
    let snapshot = reference_value_fixture(false);
    let input = Arc::new(snapshot.preview(&snapshot.change_set()).unwrap());
    let directory = Directory::new();
    let session = Arc::new(PublicationFrontierSession::create(&directory.0, [9; 32], 0).unwrap());
    let first = run(&snapshot, Some(session.clone()), |_| {}).unwrap();
    fn context(
        overlay: &agq_kernel::derived::ConstructionOverlay,
    ) -> Result<SemanticContext<'_>, PublicationOverlayError> {
        SemanticContext::for_construction_overlay(
            overlay,
            SemanticOptions {
                baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V8,
                ..Default::default()
            },
            BTreeSet::new(),
        )
        .map_err(PublicationOverlayError::Context)
    }
    let expected = close_construction_structure_with_extension(
        &input,
        Default::default(),
        context,
        &(),
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap();
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            close_construction_structure_with_extension(
                &input,
                PublicationClosureOptions {
                    frontier_checkpoints: Some(session.clone()),
                    ..Default::default()
                },
                context,
                &(),
                |_, _, _, _| {},
                |stage| {
                    if stage.stratum == ResultStructureStratum::ContextualBindings {
                        panic!("construction interruption");
                    }
                },
            )
            .unwrap();
        }))
        .is_err()
    );
    let (journal, pin) = session.latest_checkpoint().unwrap().unwrap();
    let resumed = Arc::new(PublicationFrontierSession::resume(&journal, pin, [9; 32], 0).unwrap());
    let restored_first = run(&snapshot, Some(resumed.clone()), |_| {
        panic!("first invocation replayed")
    })
    .unwrap();
    compare(&first, &restored_first, None);
    let actual = close_construction_structure_with_extension(
        &input,
        PublicationClosureOptions {
            frontier_checkpoints: Some(resumed.clone()),
            ..Default::default()
        },
        context,
        &(),
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap();
    assert_eq!(expected.completeness, actual.completeness);
    assert_eq!(expected.converged, actual.converged);
    assert!(
        expected
            .overlay
            .model()
            .elements()
            .eq(actual.overlay.model().elements())
    );
    assert!(expected.overlay.facts().eq(actual.overlay.facts()));
    assert!(
        expected
            .overlay
            .model()
            .computation_searches()
            .eq(actual.overlay.model().computation_searches())
    );
    assert!(
        expected
            .overlay
            .model()
            .ordered_reference_contributions()
            .eq(actual.overlay.model().ordered_reference_contributions())
    );
    expected
        .certificate
        .as_ref()
        .unwrap()
        .assert_exact(actual.certificate.as_ref().unwrap());
    crate::closure_equivalence_tests::assert_exact_queries(
        &KerMlQueries::new(context(&expected.overlay).unwrap()),
        &KerMlQueries::new(context(&actual.overlay).unwrap()),
    );
    assert_eq!(resumed.statistics().restored_invocations, 2);
    assert_eq!(resumed.statistics().restored_completed_invocations, 1);
}

#[test]
fn checkpoint_rejects_changed_source_graph_context_journal_and_archive() {
    let snapshot = reference_value_fixture(false);
    let directory = Directory::new();
    let session = Arc::new(PublicationFrontierSession::create(&directory.0, [5; 32], 0).unwrap());
    run(&snapshot, Some(session.clone()), |_| {}).unwrap();
    let (journal, pin) = session.latest_checkpoint().unwrap().unwrap();
    assert!(PublicationFrontierSession::resume(&journal, pin, [6; 32], 0).is_err());
    assert!(PublicationFrontierSession::resume(&journal, [0; 32], [5; 32], 0).is_err());
    let changed = reference_value_fixture(true);
    let resumed = Arc::new(PublicationFrontierSession::resume(&journal, pin, [5; 32], 0).unwrap());
    assert!(run(&changed, Some(resumed), |_| {}).is_err());
    let resumed = Arc::new(PublicationFrontierSession::resume(&journal, pin, [5; 32], 0).unwrap());
    assert!(
        close_result_structure(
            &snapshot,
            PublicationClosureOptions {
                frontier_checkpoints: Some(resumed),
                ..Default::default()
            },
            |overlay| SemanticContext::for_overlay(
                overlay,
                SemanticOptions {
                    baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V8,
                    exclude_implied: true
                },
                BTreeSet::new()
            )
            .map_err(PublicationOverlayError::Context),
            |_, _, _, _| {},
            |_| {}
        )
        .is_err()
    );
    let parsed: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&journal).unwrap()).unwrap();
    let digest: [u8; 32] =
        serde_json::from_value(parsed["entries"][0]["archive_sha256"].clone()).unwrap();
    let archive = directory.0.join(format!(
        "{}.zip",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    ));
    let mut bytes = std::fs::read(&archive).unwrap();
    bytes[0] ^= 1;
    std::fs::write(&archive, bytes).unwrap();
    let resumed = Arc::new(PublicationFrontierSession::resume(&journal, pin, [5; 32], 0).unwrap());
    assert!(run(&snapshot, Some(resumed), |_| {}).is_err());
    std::fs::write(&journal, b"{}").unwrap();
    assert!(PublicationFrontierSession::resume(&journal, pin, [5; 32], 0).is_err());
}
