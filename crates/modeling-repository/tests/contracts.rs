use agq_modeling_repository::*;
use std::collections::BTreeMap;

fn candidate() -> CandidateRevision {
    let source = b"package Example {}\r\n".to_vec();
    let checkpoint = b"synthetic identity checkpoint, never semantic state".to_vec();
    let content_digest = ContentDigest::of(&source);
    let checkpoint_digest = ContentDigest::of(&checkpoint);
    CandidateRevision {
        manifest: RevisionManifest {
            format_version: RevisionManifest::FORMAT_VERSION,
            revision_id: ProjectRevisionId::new(),
            parent_revision_id: None,
            project_id: ProjectId::new(),
            metadata: ResourceMetadata {
                created: "2026-09-24T00:00:00Z".into(),
                ..Default::default()
            },
            documents: vec![DocumentManifest {
                document_id: DocumentId::new(),
                path: "example.sysml".into(),
                language: SourceLanguage::SysMl,
                source_revision_id: SourceRevisionId::new(),
                content_digest,
            }],
            accepted_publications: PublicationBinding {
                kerml: ContentDigest::of(b"synthetic KerML publication"),
                sysml: ContentDigest::of(b"synthetic Systems publication"),
            },
            checkpoint_digest,
            validation: ValidationState::Working,
            semantic_cache: None,
        },
        blobs: BTreeMap::from([(content_digest, source), (checkpoint_digest, checkpoint)]),
    }
}

fn with_receipt(mut manifest: RevisionManifest) -> RevisionManifest {
    manifest.validation = ValidationState::Validated(ValidationReceipt {
        acceptance_contract: "agentique-modeling-workspace-phase1/1".into(),
        source_binding: manifest.source_binding().unwrap(),
        semantic_digest: ContentDigest::of(b"synthetic semantic receipt"),
        semantic_context: ContentDigest::of(b"synthetic semantic context"),
        closure_digest: ContentDigest::of(b"synthetic closure certificate"),
    });
    manifest
}

#[test]
fn content_digest_matches_sha256_and_preserves_exact_bytes() {
    assert_eq!(
        ContentDigest::of(b"abc").hex(),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_ne!(ContentDigest::of(b"line\r\n"), ContentDigest::of(b"line\n"));
    assert_ne!(ContentDigest::of(b"name"), ContentDigest::of(b"name "));
}

#[test]
fn canonical_digest_json_supports_map_keys_and_rejects_malformed_values() {
    let value = candidate();
    let bytes = serde_json::to_vec(&value).unwrap();
    let restored: CandidateRevision = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(restored, value);
    restored.verify().unwrap();
    for malformed in [
        "".to_owned(),
        "a".repeat(63),
        "a".repeat(65),
        "A".repeat(64),
        "g".repeat(64),
        "é".repeat(32),
        format!("{} ", "a".repeat(63)),
    ] {
        assert!(
            serde_json::from_str::<ContentDigest>(&serde_json::to_string(&malformed).unwrap())
                .is_err(),
            "accepted malformed digest {malformed:?}"
        );
    }
}

#[test]
fn future_manifest_versions_and_self_parent_are_rejected() {
    let mut value = candidate();
    value.manifest.format_version = RevisionManifest::FORMAT_VERSION + 1;
    assert!(matches!(
        value.verify(),
        Err(RepositoryError::UnsupportedFormat(_))
    ));
    value.manifest.format_version = RevisionManifest::FORMAT_VERSION;
    value.manifest.parent_revision_id = Some(value.manifest.revision_id);
    assert!(matches!(value.verify(), Err(RepositoryError::Integrity(_))));
}

#[test]
fn resource_metadata_requires_rfc3339_without_rewriting_valid_timestamps() {
    for created in [
        "2026-09-24T00:00:00Z",
        "2026-09-24T02:30:00.123456789+02:30",
        "2024-02-29T12:30:45-05:00",
    ] {
        let metadata = ResourceMetadata {
            created: created.into(),
            ..Default::default()
        };
        metadata.verify().unwrap();
        assert_eq!(metadata.created, created);
    }
    for created in [
        "",
        "not a timestamp",
        "2026-09-24",
        "2026-09-24T00:00:00",
        "2026-02-29T00:00:00Z",
        "2026-09-24T24:00:00Z",
        "2026-09-24T00:00:00+25:00",
        " 2026-09-24T00:00:00Z",
        "2026-09-24T00:00:00Z trailing",
    ] {
        let metadata = ResourceMetadata {
            created: created.into(),
            ..Default::default()
        };
        assert!(
            matches!(metadata.verify(), Err(RepositoryError::Integrity(_))),
            "accepted malformed timestamp {created:?}"
        );
    }
}

#[test]
fn invalid_revision_metadata_is_rejected_even_with_valid_sources_and_receipt() {
    let mut value = candidate();
    value.manifest = with_receipt(value.manifest);
    value.manifest.metadata.created = "2026-09-24".into();
    assert!(matches!(value.verify(), Err(RepositoryError::Integrity(_))));
    // Metadata remains outside the semantic source binding, but it must satisfy
    // its own storage contract before a checksum can authenticate the revision.
    assert!(matches!(
        value.manifest.verify(),
        Err(RepositoryError::Integrity(_))
    ));
}

#[test]
fn document_identity_path_and_source_revision_populations_are_unique() {
    let value = candidate();
    let original = value.manifest.documents[0].clone();
    for duplicate in [
        DocumentManifest {
            path: "other.sysml".into(),
            source_revision_id: SourceRevisionId::new(),
            ..original.clone()
        },
        DocumentManifest {
            document_id: DocumentId::new(),
            source_revision_id: SourceRevisionId::new(),
            ..original.clone()
        },
        DocumentManifest {
            document_id: DocumentId::new(),
            path: "other.sysml".into(),
            ..original
        },
    ] {
        let mut invalid = value.clone();
        invalid.manifest.documents.push(duplicate);
        assert!(invalid.verify().is_err());
    }
    let mut invalid = value;
    invalid.manifest.documents[0].path.clear();
    assert!(invalid.verify().is_err());
}

#[test]
fn candidate_requires_every_source_and_identity_checkpoint_blob() {
    let value = candidate();
    for digest in value.manifest.required_blobs() {
        let mut missing = value.clone();
        missing.blobs.remove(&digest);
        assert!(matches!(
            missing.verify(),
            Err(RepositoryError::Integrity(_))
        ));
    }
    let mut corrupt = value.clone();
    corrupt
        .blobs
        .get_mut(&value.manifest.documents[0].content_digest)
        .unwrap()
        .push(b' ');
    assert!(matches!(
        corrupt.verify(),
        Err(RepositoryError::Integrity(_))
    ));
}

#[test]
fn authenticated_non_utf8_authored_source_is_rejected() {
    let mut value = candidate();
    let source = vec![0xff, 0xfe];
    let digest = ContentDigest::of(&source);
    value.blobs.insert(digest, source);
    value.manifest.documents[0].content_digest = digest;
    assert!(matches!(value.verify(), Err(RepositoryError::Integrity(_))));
}

#[test]
fn validation_receipt_authenticates_each_source_identity_and_publication_binding() {
    let original = with_receipt(candidate().manifest);
    original.verify().unwrap();
    let mut mutations = Vec::new();
    let mut changed = original.clone();
    changed.project_id = ProjectId::new();
    mutations.push(changed);
    let mut changed = original.clone();
    changed.revision_id = ProjectRevisionId::new();
    mutations.push(changed);
    let mut changed = original.clone();
    changed.parent_revision_id = Some(ProjectRevisionId::new());
    mutations.push(changed);
    let mut changed = original.clone();
    changed.checkpoint_digest = ContentDigest::of(b"other identity checkpoint");
    mutations.push(changed);
    let mut changed = original.clone();
    changed.accepted_publications.kerml = ContentDigest::of(b"other KerML publication");
    mutations.push(changed);
    let mut changed = original.clone();
    changed.accepted_publications.sysml = ContentDigest::of(b"other Systems publication");
    mutations.push(changed);
    let mut changed = original.clone();
    changed.documents[0].document_id = DocumentId::new();
    mutations.push(changed);
    let mut changed = original.clone();
    changed.documents[0].source_revision_id = SourceRevisionId::new();
    mutations.push(changed);
    let mut changed = original.clone();
    changed.documents[0].path = "renamed.sysml".into();
    mutations.push(changed);
    let mut changed = original.clone();
    changed.documents[0].language = SourceLanguage::KerMl;
    mutations.push(changed);
    let mut changed = original.clone();
    changed.documents[0].content_digest = ContentDigest::of(b"different source");
    mutations.push(changed);
    for changed in mutations {
        assert_ne!(
            original.source_binding().unwrap(),
            changed.source_binding().unwrap()
        );
        assert!(matches!(
            changed.verify(),
            Err(RepositoryError::Integrity(_))
        ));
    }
}

#[test]
fn unknown_acceptance_contract_is_rejected_without_promoting_working_state() {
    let mut manifest = with_receipt(candidate().manifest);
    let ValidationState::Validated(receipt) = &mut manifest.validation else {
        unreachable!();
    };
    receipt.acceptance_contract = "full-language-conformance/1".into();
    assert!(manifest.verify().is_err());
    manifest.validation = ValidationState::Working;
    manifest.verify().unwrap();
    let round_trip: RevisionManifest =
        serde_json::from_slice(&serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert_eq!(round_trip.validation, ValidationState::Working);
}

#[test]
fn cache_and_display_metadata_do_not_rewrite_the_source_binding() {
    let original = with_receipt(candidate().manifest);
    let mut changed = original.clone();
    changed.metadata.name = Some("display metadata".into());
    changed.semantic_cache = Some(SemanticCacheReference {
        format: "unknown-cache-format/99".into(),
        content_digest: ContentDigest::of(b"missing optional cache bytes"),
        source_binding: original.source_binding().unwrap(),
        semantic_context: ContentDigest::of(b"cache context"),
        closure_digest: ContentDigest::of(b"cache closure"),
    });
    assert_eq!(
        original.source_binding().unwrap(),
        changed.source_binding().unwrap()
    );
    assert_ne!(original.digest().unwrap(), changed.digest().unwrap());
    assert_eq!(original.required_blobs(), changed.required_blobs());
    changed.verify().unwrap();
}

#[test]
fn storage_receipt_check_does_not_claim_semantic_authentication() {
    let mut manifest = with_receipt(candidate().manifest);
    let ValidationState::Validated(receipt) = &mut manifest.validation else {
        unreachable!();
    };
    receipt.semantic_digest = ContentDigest::of(b"unverified graph");
    receipt.semantic_context = ContentDigest::of(b"unverified context");
    receipt.closure_digest = ContentDigest::of(b"unverified closure");
    // Only the service rebuild and actual workspace validation may authenticate
    // these fields. A source/format check must never claim to have done that work.
    manifest.verify().unwrap();
}
