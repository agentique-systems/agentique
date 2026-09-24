use super::*;
use std::{
    collections::BTreeMap,
    sync::{Arc, Barrier},
};

const POINTS: [Point; 6] = [
    Point::AfterSourceBlobWrite,
    Point::AfterRevisionManifestWrite,
    Point::BeforeHeadUpdate,
    Point::AfterHeadUpdateStatement,
    Point::BeforeTransactionCommit,
    Point::AfterDurableCommit,
];

fn metadata() -> ResourceMetadata {
    ResourceMetadata {
        created: "2026-09-24T00:00:00Z".into(),
        ..Default::default()
    }
}

/// Synthetic storage fixture. These receipts are never used to issue a semantic handle.
fn fixture(documents: usize) -> CreateProject {
    let project_id = ProjectId::new();
    let branch_id = BranchId::new();
    let revision_id = ProjectRevisionId::new();
    let checkpoint = b"synthetic storage fixture identity checkpoint".to_vec();
    let checkpoint_digest = ContentDigest::of(&checkpoint);
    let mut blobs = BTreeMap::from([(checkpoint_digest, checkpoint)]);
    let documents = (0..documents)
        .map(|index| {
            let bytes = format!("package Document{index} {{}}\r\n").into_bytes();
            let content_digest = ContentDigest::of(&bytes);
            blobs.insert(content_digest, bytes);
            DocumentManifest {
                document_id: DocumentId::new(),
                path: format!(
                    "document-{index}.{}",
                    if index % 2 == 0 { "sysml" } else { "kerml" }
                ),
                language: if index % 2 == 0 {
                    SourceLanguage::SysMl
                } else {
                    SourceLanguage::KerMl
                },
                source_revision_id: SourceRevisionId::new(),
                content_digest,
            }
        })
        .collect();
    CreateProject {
        operation_id: OperationId::new(),
        project: Project {
            id: project_id,
            name: "Repository fixture".into(),
            default_branch: branch_id,
            metadata: metadata(),
        },
        branch: Branch {
            id: branch_id,
            project_id,
            name: "main".into(),
            head: revision_id,
            metadata: metadata(),
        },
        initial: CandidateRevision {
            manifest: RevisionManifest {
                format_version: RevisionManifest::FORMAT_VERSION,
                revision_id,
                parent_revision_id: None,
                project_id,
                metadata: metadata(),
                documents,
                accepted_publications: PublicationBinding {
                    kerml: ContentDigest::of(b"synthetic-kerml"),
                    sysml: ContentDigest::of(b"synthetic-sysml"),
                },
                checkpoint_digest,
                validation: ValidationState::Working,
                semantic_cache: None,
            },
            blobs,
        },
    }
}

fn child(parent: &CandidateRevision, branch_id: BranchId, edit: usize) -> CommitRevision {
    let mut candidate = parent.clone();
    candidate.manifest.revision_id = ProjectRevisionId::new();
    candidate.manifest.parent_revision_id = Some(parent.manifest.revision_id);
    candidate.manifest.validation = ValidationState::Working;
    let document = &mut candidate.manifest.documents[edit % parent.manifest.documents.len()];
    document.source_revision_id = SourceRevisionId::new();
    let bytes = format!("package Changed{edit} {{ part newPart; }}\n").into_bytes();
    document.content_digest = ContentDigest::of(&bytes);
    candidate.blobs.insert(document.content_digest, bytes);
    let required = candidate.manifest.required_blobs();
    candidate
        .blobs
        .retain(|digest, _| required.contains(digest));
    CommitRevision {
        operation_id: OperationId::new(),
        project_id: parent.manifest.project_id,
        branch_id,
        expected_head: parent.manifest.revision_id,
        candidate,
    }
}

fn validate_storage_receipt(candidate: &mut CandidateRevision) {
    candidate.manifest.validation = ValidationState::Validated(ValidationReceipt {
        acceptance_contract: "agentique-modeling-workspace-phase1/1".into(),
        source_binding: candidate.manifest.source_binding().unwrap(),
        semantic_digest: ContentDigest::of(b"synthetic semantic receipt; no semantic acceptance"),
        semantic_context: ContentDigest::of(b"synthetic semantic context"),
        closure_digest: ContentDigest::of(b"synthetic closure digest"),
    });
}

#[test]
fn exact_restart_history_and_source_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("repository.db");
    let repository = SqliteRepository::open(&path).unwrap();
    let identity = repository.repository_id();
    let initial = fixture(3);
    repository.create_project(&initial).unwrap();
    let request = child(&initial.initial, initial.branch.id, 1);
    repository.commit_revision(&request).unwrap();
    drop(repository);
    let repository = SqliteRepository::open(path).unwrap();
    assert_eq!(repository.repository_id(), identity);
    assert_eq!(
        repository.get_project(initial.project.id).unwrap(),
        initial.project
    );
    assert_eq!(
        repository
            .load_revision(initial.project.id, initial.initial.manifest.revision_id)
            .unwrap(),
        initial.initial.manifest
    );
    assert_eq!(
        repository
            .list_revision_history(initial.project.id, request.candidate.manifest.revision_id)
            .unwrap(),
        vec![request.candidate.manifest.clone(), initial.initial.manifest]
    );
    for (digest, bytes) in &request.candidate.blobs {
        assert_eq!(&repository.read_blob(*digest).unwrap(), bytes);
    }
    assert!(repository.check_integrity().unwrap().is_ok());
}

#[test]
fn all_precommit_failures_roll_back_and_lost_acknowledgement_replays() {
    for point in POINTS {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("repository.db");
        let repository = SqliteRepository::open(&path).unwrap();
        let initial = fixture(2);
        repository.create_project(&initial).unwrap();
        let request = child(&initial.initial, initial.branch.id, 1);
        repository.faults.set(point, false);
        let error = repository.commit_revision(&request).unwrap_err();
        assert_eq!(
            matches!(error, RepositoryError::OutcomeUnknown(_)),
            point == Point::AfterDurableCommit
        );
        drop(repository);
        let repository = SqliteRepository::open(path).unwrap();
        let committed = point == Point::AfterDurableCommit;
        assert_eq!(
            repository
                .get_branch(initial.project.id, initial.branch.id)
                .unwrap()
                .head,
            if committed {
                request.candidate.manifest.revision_id
            } else {
                initial.branch.head
            }
        );
        assert_eq!(
            repository.list_revisions(initial.project.id).unwrap().len(),
            if committed { 2 } else { 1 }
        );
        assert!(repository.check_integrity().unwrap().is_ok());
        let receipt = repository.commit_revision(&request).unwrap();
        assert_eq!(receipt.replayed, committed);
        assert!(repository.commit_revision(&request).unwrap().replayed);
        assert_eq!(
            repository.list_revisions(initial.project.id).unwrap().len(),
            2
        );
    }
}

#[test]
fn initial_project_commit_is_atomic_at_every_fault_boundary() {
    for point in POINTS {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("repository.db");
        let repository = SqliteRepository::open(&path).unwrap();
        let initial = fixture(2);
        repository.faults.set(point, false);
        assert!(repository.create_project(&initial).is_err());
        drop(repository);
        let repository = SqliteRepository::open(path).unwrap();
        assert_eq!(
            repository.list_projects().unwrap().len(),
            usize::from(point == Point::AfterDurableCommit)
        );
        assert!(repository.check_integrity().unwrap().is_ok());
        assert_eq!(
            repository.create_project(&initial).unwrap().replayed,
            point == Point::AfterDurableCommit
        );
    }
}

#[test]
#[ignore = "subprocess helper invoked by crash_recovery_at_every_durability_boundary"]
fn process_fault_child() {
    let path = std::env::var_os("AGQ_SQLITE_CRASH_DB").expect("crash db");
    let input =
        std::fs::read(std::env::var_os("AGQ_SQLITE_CRASH_REQUEST").expect("request path")).unwrap();
    let (operation_id, project_id, branch_id, expected_head, candidate) =
        serde_json::from_slice(&input).unwrap();
    let point: usize = std::env::var("AGQ_SQLITE_CRASH_POINT")
        .unwrap()
        .parse()
        .unwrap();
    let repository = SqliteRepository::open(path).unwrap();
    repository.faults.set(POINTS[point], true);
    repository
        .commit_revision(&CommitRevision {
            operation_id,
            project_id,
            branch_id,
            expected_head,
            candidate,
        })
        .unwrap();
    panic!("fault point did not terminate process");
}

#[test]
fn crash_recovery_at_every_durability_boundary() {
    for (index, point) in POINTS.into_iter().enumerate() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("repository.db");
        let request_path = directory.path().join("request.json");
        let repository = SqliteRepository::open(&path).unwrap();
        let initial = fixture(2);
        repository.create_project(&initial).unwrap();
        let request = child(&initial.initial, initial.branch.id, 1);
        std::fs::write(
            &request_path,
            serde_json::to_vec(&(
                request.operation_id,
                request.project_id,
                request.branch_id,
                request.expected_head,
                &request.candidate,
            ))
            .unwrap(),
        )
        .unwrap();
        drop(repository);
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "tests::process_fault_child",
                "--ignored",
                "--nocapture",
            ])
            .env("AGQ_SQLITE_CRASH_DB", &path)
            .env("AGQ_SQLITE_CRASH_REQUEST", request_path)
            .env("AGQ_SQLITE_CRASH_POINT", index.to_string())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(86),
            "child output: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let repository = SqliteRepository::open(&path).unwrap();
        let committed = point == Point::AfterDurableCommit;
        assert_eq!(
            repository
                .get_branch(initial.project.id, initial.branch.id)
                .unwrap()
                .head,
            if committed {
                request.candidate.manifest.revision_id
            } else {
                initial.branch.head
            }
        );
        assert!(repository.check_integrity().unwrap().is_ok());
        assert_eq!(
            repository.commit_revision(&request).unwrap().replayed,
            committed
        );
    }
}

#[test]
fn two_independent_writers_have_one_explicit_cas_winner() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("repository.db");
    let repository = SqliteRepository::open(&path).unwrap();
    let initial = fixture(2);
    repository.create_project(&initial).unwrap();
    let barrier = Arc::new(Barrier::new(2));
    let requests = [
        child(&initial.initial, initial.branch.id, 10),
        child(&initial.initial, initial.branch.id, 11),
    ];
    let results = std::thread::scope(|scope| {
        let handles = requests
            .iter()
            .map(|request| {
                let barrier = barrier.clone();
                let path = &path;
                scope.spawn(move || {
                    let repository = SqliteRepository::open(path).unwrap();
                    barrier.wait();
                    repository.commit_revision(request)
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(result, Err(RepositoryError::Conflict { .. })))
            .count(),
        1
    );
    assert_eq!(
        repository.list_revisions(initial.project.id).unwrap().len(),
        2
    );
    assert!(repository.check_integrity().unwrap().is_ok());
}

#[test]
fn exact_operation_identity_survives_later_head_moves() {
    let directory = tempfile::tempdir().unwrap();
    let repository = SqliteRepository::open(directory.path().join("repository.db")).unwrap();
    let initial = fixture(2);
    repository.create_project(&initial).unwrap();
    let request = child(&initial.initial, initial.branch.id, 1);
    repository.commit_revision(&request).unwrap();
    let next = child(&request.candidate, initial.branch.id, 2);
    repository.commit_revision(&next).unwrap();
    let replay = repository.commit_revision(&request).unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.revision_id, request.candidate.manifest.revision_id);
    assert_eq!(
        repository
            .get_branch(initial.project.id, initial.branch.id)
            .unwrap()
            .head,
        next.candidate.manifest.revision_id
    );
    let mut collision = child(&initial.initial, initial.branch.id, 3);
    collision.operation_id = request.operation_id;
    assert!(matches!(
        repository.commit_revision(&collision),
        Err(RepositoryError::OperationCollision(_))
    ));
}

#[test]
fn branch_deletion_retains_history_and_other_heads() {
    let directory = tempfile::tempdir().unwrap();
    let repository = SqliteRepository::open(directory.path().join("repository.db")).unwrap();
    let initial = fixture(2);
    repository.create_project(&initial).unwrap();
    let experiment = Branch {
        id: BranchId::new(),
        name: "architecture-experiment".into(),
        ..initial.branch.clone()
    };
    repository.create_branch(&experiment).unwrap();
    let request = child(&initial.initial, experiment.id, 1);
    repository.commit_revision(&request).unwrap();
    assert_eq!(
        repository
            .get_branch(initial.project.id, initial.branch.id)
            .unwrap()
            .head,
        initial.branch.head
    );
    repository
        .delete_branch(initial.project.id, experiment.id)
        .unwrap();
    assert!(matches!(
        repository.get_branch(initial.project.id, experiment.id),
        Err(RepositoryError::NotFound(_))
    ));
    assert_eq!(
        repository.list_revisions(initial.project.id).unwrap().len(),
        2
    );
    assert_eq!(
        repository
            .load_revision(initial.project.id, request.candidate.manifest.revision_id)
            .unwrap(),
        request.candidate.manifest
    );
    assert!(
        repository
            .delete_branch(initial.project.id, initial.branch.id)
            .is_err()
    );
    assert!(repository.check_integrity().unwrap().is_ok());
}

#[test]
fn revision_source_and_operation_collisions_are_rejected_atomically() {
    let directory = tempfile::tempdir().unwrap();
    let repository = SqliteRepository::open(directory.path().join("repository.db")).unwrap();
    let initial = fixture(2);
    repository.create_project(&initial).unwrap();
    let mut request = child(&initial.initial, initial.branch.id, 0);
    request.candidate.manifest.documents[0].source_revision_id =
        initial.initial.manifest.documents[0].source_revision_id;
    assert!(matches!(
        repository.commit_revision(&request),
        Err(RepositoryError::Integrity(_))
    ));
    assert_eq!(
        repository
            .get_branch(initial.project.id, initial.branch.id)
            .unwrap()
            .head,
        initial.branch.head
    );
    let mut other = fixture(2);
    other.initial.manifest.revision_id = initial.initial.manifest.revision_id;
    other.branch.head = initial.initial.manifest.revision_id;
    assert!(matches!(
        repository.create_project(&other),
        Err(RepositoryError::Integrity(_))
    ));
    assert_eq!(repository.list_projects().unwrap().len(), 1);
    let mut foreign = initial.branch.clone();
    foreign.id = BranchId::new();
    foreign.project_id = ProjectId::new();
    assert!(repository.create_branch(&foreign).is_err());
    assert!(repository.check_integrity().unwrap().is_ok());
}

#[test]
fn optional_cache_corruption_is_discardable_but_source_corruption_is_fatal() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("repository.db");
    let repository = SqliteRepository::open(&path).unwrap();
    let mut initial = fixture(2);
    let cache = b"opaque synthetic cache".to_vec();
    let digest = ContentDigest::of(&cache);
    initial.initial.manifest.semantic_cache = Some(SemanticCacheReference {
        format: "synthetic-cache/1".into(),
        content_digest: digest,
        source_binding: initial.initial.manifest.source_binding().unwrap(),
        semantic_context: ContentDigest::of(b"synthetic-context"),
        closure_digest: ContentDigest::of(b"synthetic-closure"),
    });
    initial.initial.blobs.insert(digest, cache);
    repository.create_project(&initial).unwrap();
    let raw = Connection::open(&path).unwrap();
    raw.execute(
        "UPDATE blobs SET bytes=? WHERE digest=?",
        params![b"corrupt".to_vec(), digest.hex()],
    )
    .unwrap();
    let report = repository.check_integrity().unwrap();
    assert!(report.is_ok(), "{:?}", report.errors);
    assert_eq!(
        report.discardable_caches,
        vec![initial.initial.manifest.revision_id]
    );
    raw.execute("DELETE FROM blobs WHERE digest=?", [digest.hex()])
        .unwrap();
    assert!(repository.check_integrity().unwrap().is_ok());
    let source_digest = initial.initial.manifest.documents[0].content_digest;
    raw.execute(
        "UPDATE blobs SET bytes=? WHERE digest=?",
        params![b"corrupt".to_vec(), source_digest.hex()],
    )
    .unwrap();
    assert!(!repository.check_integrity().unwrap().is_ok());
    assert!(repository.read_blob(source_digest).is_err());
}

#[test]
fn manifest_checksum_and_document_row_tampering_are_detected() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("repository.db");
    let repository = SqliteRepository::open(&path).unwrap();
    let initial = fixture(2);
    repository.create_project(&initial).unwrap();
    let raw = Connection::open(path).unwrap();
    raw.execute(
        "UPDATE revision_documents SET path='tampered' WHERE document_id=?",
        [key(&initial.initial.manifest.documents[0].document_id).unwrap()],
    )
    .unwrap();
    assert!(!repository.check_integrity().unwrap().is_ok());
    raw.execute("UPDATE revisions SET manifest=?", [b"{}".to_vec()])
        .unwrap();
    assert!(matches!(
        repository.load_revision(initial.project.id, initial.branch.head),
        Err(RepositoryError::Integrity(_))
    ));
}

#[test]
fn forged_parent_cycle_is_detected_even_with_recomputed_manifest_checksums() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("repository.db");
    let repository = SqliteRepository::open(&path).unwrap();
    let initial = fixture(2);
    repository.create_project(&initial).unwrap();
    let request = child(&initial.initial, initial.branch.id, 1);
    repository.commit_revision(&request).unwrap();
    let mut manifest = initial.initial.manifest.clone();
    manifest.parent_revision_id = Some(request.candidate.manifest.revision_id);
    let (data, digest) = encoded(&manifest).unwrap();
    let raw = Connection::open(path).unwrap();
    raw.execute(
        "UPDATE revisions SET parent_id=?,manifest=?,digest=? WHERE id=?",
        params![
            key(&request.candidate.manifest.revision_id).unwrap(),
            data,
            digest,
            key(&manifest.revision_id).unwrap()
        ],
    )
    .unwrap();
    let report = repository.check_integrity().unwrap();
    assert!(
        report
            .errors
            .iter()
            .any(|error| error.contains("parent cycle"))
    );
    assert!(
        repository
            .list_revision_history(initial.project.id, initial.branch.head)
            .is_err()
    );
}

#[test]
fn corrupt_operation_receipt_is_detected_by_offline_integrity_check() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("repository.db");
    let repository = SqliteRepository::open(&path).unwrap();
    repository.create_project(&fixture(2)).unwrap();
    let raw = Connection::open(path).unwrap();
    raw.execute("UPDATE operation_receipts SET data=?", [b"{}".to_vec()])
        .unwrap();
    let report = repository.check_integrity().unwrap();
    assert!(
        report
            .errors
            .iter()
            .any(|error| error.contains("operation") && error.contains("checksum"))
    );
}

#[test]
fn durable_storage_scale_100_documents_10_revisions_two_branches_four_readers() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("repository.db");
    let repository = SqliteRepository::open(&path).unwrap();
    let mut initial = fixture(100);
    validate_storage_receipt(&mut initial.initial);
    repository.create_project(&initial).unwrap();
    let experiment = Branch {
        id: BranchId::new(),
        name: "architecture-experiment".into(),
        ..initial.branch.clone()
    };
    repository.create_branch(&experiment).unwrap();
    let mut revisions = vec![initial.initial.clone()];
    for edit in 1..=8 {
        let mut request = child(revisions.last().unwrap(), initial.branch.id, edit);
        if edit % 2 == 0 {
            validate_storage_receipt(&mut request.candidate);
        }
        repository.commit_revision(&request).unwrap();
        revisions.push(request.candidate);
    }
    let request = child(&initial.initial, experiment.id, 9);
    repository.commit_revision(&request).unwrap();
    revisions.push(request.candidate);
    assert_eq!(repository.check_integrity().unwrap().blobs_checked, 110);
    drop(repository);
    let repository = Arc::new(SqliteRepository::open(path).unwrap());
    assert_eq!(
        repository.list_branches(initial.project.id).unwrap().len(),
        2
    );
    assert_eq!(
        repository.list_revisions(initial.project.id).unwrap().len(),
        10
    );
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let repository = repository.clone();
            let revisions = &revisions;
            scope.spawn(move || {
                for expected in revisions {
                    let actual = repository
                        .load_revision(expected.manifest.project_id, expected.manifest.revision_id)
                        .unwrap();
                    assert_eq!(actual, expected.manifest);
                    for document in &actual.documents {
                        assert_eq!(
                            repository.read_blob(document.content_digest).unwrap(),
                            expected.blobs[&document.content_digest]
                        );
                    }
                }
            });
        }
    });
    let report = repository.check_integrity().unwrap();
    assert!(report.is_ok(), "{:?}", report.errors);
    assert_eq!((report.revisions_checked, report.blobs_checked), (10, 110));
}
