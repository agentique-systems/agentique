//! Durable identity restoration uses accepted inputs and full source reconstruction.
#![allow(dead_code)]
mod support;
use agq_kerml_text::SourceLanguage;
use agq_modeling_workspace::{ProjectRevisionCheckpoint, ProjectWorkspace};
use std::{collections::BTreeMap, sync::Arc, time::Instant};

const EDITED_SOURCE: &str = "package Edited { part def Unit; connection def Cable { end source : Unit; end target : Unit; } part def Assembly { attribute count : ScalarValues::Integer = 1; part firstUnit : Unit[1]; part secondUnit : Unit; connection cable : Cable connect firstUnit to secondUnit; } }";
const STABLE_SOURCE: &str = "package Stable { datatype Retained; }";
const OTHER_SOURCE: &str = "package Other { part def Unchanged; }";

#[test]
fn local_edit_frontend_fixtures_are_complete() {
    use agq_kerml_syntax::{DocumentId, SourceRevisionId, production};
    let kerml = production::parse(
        DocumentId::new(),
        SourceRevisionId::new(),
        STABLE_SOURCE,
        Default::default(),
    )
    .unwrap();
    assert!(kerml.is_complete(), "{:?}", kerml.diagnostics());
    for source in [EDITED_SOURCE, OTHER_SOURCE] {
        let syntax = production::parse_sysml_with_profile(
            production::SysmlSyntaxProfile::OperationalV3,
            DocumentId::new(),
            SourceRevisionId::new(),
            source,
            Default::default(),
        )
        .unwrap();
        assert!(syntax.is_complete(), "{:?}", syntax.diagnostics());
    }
}

#[test]
#[ignore = "requires accepted publication caches; never rebuilds standards"]
fn exact_source_restore_and_detached_candidate_acknowledgement() {
    let accepted = support::accepted();
    let mut workspace = ProjectWorkspace::open(accepted.clone()).unwrap();
    let parent = workspace.head().clone();
    let candidate = workspace
        .prepare(
            parent.revision(),
            [support::add(
                "platform.sysml",
                SourceLanguage::SysMl,
                "package Platform { part def Repository; part repository : Repository; }",
            )],
        )
        .unwrap();
    assert_eq!(workspace.head().revision(), parent.revision());
    assert!(workspace.revision(candidate.revision()).is_none());
    candidate.validate().unwrap();
    workspace
        .acknowledge(parent.revision(), candidate.clone())
        .unwrap();
    assert_eq!(workspace.head().revision(), candidate.revision());
    let bytes = serde_json::to_vec(&candidate.checkpoint()).unwrap();
    let checkpoint: ProjectRevisionCheckpoint = serde_json::from_slice(&bytes).unwrap();
    let sources: BTreeMap<_, _> = candidate
        .documents()
        .map(|(_, doc)| (doc.id(), doc.source().to_owned()))
        .collect();
    let fingerprint = candidate.semantic_fingerprint().unwrap();
    let root = candidate.root();
    let element = support::element(&candidate, &["Platform", "repository"]);
    drop(workspace);
    drop(parent);
    drop(candidate);
    let started = Instant::now();
    let restored = checkpoint.restore(accepted.clone(), &sources).unwrap();
    eprintln!("cold_source_restore_ms={}", started.elapsed().as_millis());
    assert_eq!(restored.revision(), checkpoint.project_revision_id);
    assert_eq!(restored.parent(), checkpoint.parent_revision_id);
    assert_eq!(restored.root(), root);
    assert_eq!(
        support::element(&restored, &["Platform", "repository"]),
        element
    );
    assert_eq!(restored.checkpoint(), checkpoint);
    assert_eq!(restored.semantic_fingerprint().unwrap(), fingerprint);
    restored.validate().unwrap();
    let fork = ProjectWorkspace::from_revision(restored.clone());
    assert!(Arc::ptr_eq(fork.head(), &restored));
    let mut corrupt = sources.clone();
    corrupt.values_mut().next().unwrap().push(' ');
    assert!(checkpoint.restore(accepted.clone(), &corrupt).is_err());
    let mut corrupt = checkpoint.clone();
    corrupt.source.accepted_sysml[0] ^= 1;
    assert!(corrupt.restore(accepted, &sources).is_err());
}

#[test]
fn project_id_and_revision_id_roundtrip_without_aliasing() {
    use agq_kerml_text::ProjectId;
    use agq_modeling_workspace::ProjectRevisionId;
    let project = ProjectId::new();
    let revision = ProjectRevisionId::new();
    assert_eq!(project.to_string().parse::<ProjectId>().unwrap(), project);
    assert_eq!(
        revision.to_string().parse::<ProjectRevisionId>().unwrap(),
        revision
    );
    assert_eq!(
        serde_json::from_str::<ProjectId>(&serde_json::to_string(&project).unwrap()).unwrap(),
        project
    );
    assert_eq!(
        serde_json::from_str::<ProjectRevisionId>(&serde_json::to_string(&revision).unwrap())
            .unwrap(),
        revision
    );
}

#[test]
#[ignore = "requires accepted publication caches; never rebuilds standards"]
fn persisted_semantic_cache_authenticates_exact_source_and_closure() {
    use agq_modeling_workspace::ProjectSemanticCache;
    let accepted = support::accepted();
    let mut workspace = ProjectWorkspace::open(accepted.clone()).unwrap();
    let candidate = workspace
        .apply(
            workspace.head().revision(),
            [support::add(
                "Cache.sysml",
                SourceLanguage::SysMl,
                "package Platform { part def Repository; part repository : Repository; }",
            )],
        )
        .unwrap();
    let validated = candidate.validate().unwrap();
    let checkpoint = candidate.checkpoint();
    let sources: BTreeMap<_, _> = candidate
        .documents()
        .map(|(_, doc)| (doc.id(), doc.source().to_owned()))
        .collect();
    let fingerprint = candidate.semantic_fingerprint().unwrap();
    let element = support::element(&candidate, &["Platform", "repository"]);
    let reference_answers: Vec<_> = candidate
        .references()
        .iter()
        .map(|reference| (reference.relationship, reference.resolution.clone()))
        .collect();
    let audit_subjects = candidate.effective_audit().unwrap().subjects().to_vec();
    let audit_counts = candidate
        .effective_audit()
        .unwrap()
        .report()
        .checked
        .clone();
    let cache = validated.semantic_cache().unwrap();
    let cache_bytes = serde_json::to_vec(&cache).unwrap();
    eprintln!(
        "semantic_cache_bytes={} kernel_frontier_bytes={}",
        cache_bytes.len(),
        cache.source.kernel_frontier.len()
    );
    drop(cache);
    drop(validated);
    drop(candidate);
    drop(workspace);
    let cache: ProjectSemanticCache = serde_json::from_slice(&cache_bytes).unwrap();
    let started = Instant::now();
    let restored = checkpoint
        .restore_cached(accepted.clone(), &sources, &cache)
        .unwrap();
    eprintln!(
        "authenticated_cache_restore_ms={} work={:?}",
        started.elapsed().as_millis(),
        restored.compilation_work()
    );
    eprintln!(
        "authenticated_cache_restore_closure_retained={} reopened={} phases={:?}",
        restored.producer_status().unwrap().retained_evaluations,
        restored.producer_status().unwrap().reopened_evaluations,
        restored.compilation_timings()
    );
    assert!(restored.compilation_work().semantic_cache_used);
    // Restored cache bytes do not replace the strict audit or its population.
    assert_eq!(
        restored.effective_audit().unwrap().subjects(),
        audit_subjects
    );
    assert_eq!(
        restored.effective_audit().unwrap().report().checked,
        audit_counts
    );
    assert_eq!(
        restored
            .compilation_work()
            .effective_audit_subjects_evaluated,
        audit_subjects.len()
    );
    assert_eq!(restored.references().len(), reference_answers.len());
    for (reference, (relationship, expected)) in
        restored.references().iter().zip(&reference_answers)
    {
        assert_eq!(reference.relationship, *relationship);
        assert_query(&reference.resolution, expected);
    }
    assert_eq!(restored.checkpoint(), checkpoint);
    assert_eq!(restored.semantic_fingerprint().unwrap(), fingerprint);
    assert_eq!(
        support::element(&restored, &["Platform", "repository"]),
        element
    );
    restored.validate().unwrap();
    drop(restored);
    let mut stale = cache.clone();
    stale.source.source_identity_digest[0] ^= 1;
    assert!(
        checkpoint
            .restore_cached(accepted.clone(), &sources, &stale)
            .is_err()
    );
    let mut corrupt = cache;
    corrupt.source.kernel_frontier[0] ^= 1;
    assert!(
        checkpoint
            .restore_cached(accepted.clone(), &sources, &corrupt)
            .is_err()
    );
    let started = Instant::now();
    let rebuilt = checkpoint.restore(accepted, &sources).unwrap();
    eprintln!(
        "source_fallback_restore_ms={}",
        started.elapsed().as_millis()
    );
    assert!(!rebuilt.compilation_work().semantic_cache_used);
    assert_eq!(rebuilt.semantic_fingerprint().unwrap(), fingerprint);
    rebuilt.validate().unwrap();
}

fn assert_query<T: std::fmt::Debug + PartialEq>(
    left: &agq_kerml_semantics::QueryResult<T>,
    right: &agq_kerml_semantics::QueryResult<T>,
) {
    let mut context = right.context.clone();
    context.revision = left.context.revision;
    assert_eq!(left.context, context);
    assert_eq!(left.value, right.value);
    assert_eq!(left.completeness, right.completeness);
    assert_eq!(left.diagnostics, right.diagnostics);
    assert_eq!(left.positive_dependencies, right.positive_dependencies);
    assert_eq!(left.search_dependencies, right.search_dependencies);
    assert_eq!(left.explanations, right.explanations);
    assert_eq!(left.fact_origins, right.fact_origins);
    assert_eq!(left.declared_fact_origins, right.declared_fact_origins);
    assert_eq!(left.canonical_dependencies, right.canonical_dependencies);
}

#[test]
#[ignore = "requires accepted publication caches; never rebuilds standards"]
fn local_edit_classes_match_full_semantic_oracle() {
    eprintln!("oracle: restoring accepted publications");
    let mut workspace = support::open();
    eprintln!("oracle: constructing source fixture");
    let source = EDITED_SOURCE;
    let base = workspace
        .apply(
            workspace.head().revision(),
            [
                support::add("Stable.kerml", SourceLanguage::KerMl, STABLE_SOURCE),
                support::add("Other.sysml", SourceLanguage::SysMl, OTHER_SOURCE),
                support::add("Edited.sysml", SourceLanguage::SysMl, source),
            ],
        )
        .unwrap();
    for diagnostic in base.diagnostics() {
        use agq_kerml_text::SourceDiagnostic;
        match diagnostic {
            SourceDiagnostic::Syntax {
                document, status, ..
            } => eprintln!("fixture syntax {document}: {status:?}"),
            SourceDiagnostic::Unsupported { origin, construct } => {
                eprintln!("fixture unsupported {:?}: {construct}", origin)
            }
            SourceDiagnostic::Frontend(diagnostic) => eprintln!(
                "fixture frontend {}: {}",
                diagnostic.code, diagnostic.message
            ),
            SourceDiagnostic::Construction(obligation) => {
                eprintln!("fixture construction: {obligation:?}")
            }
            SourceDiagnostic::Capability {
                subject, answer, ..
            } => eprintln!("fixture capability {subject}: {:?}", answer.completeness()),
            SourceDiagnostic::EffectiveAudit { finding, .. } => {
                eprintln!("fixture audit: {finding:?}")
            }
        }
    }
    base.validate()
        .expect("oracle fixture must first establish a supported Validated revision");
    for (name, old, replacement) in [
        ("attribute-value", "= 1", "= 2"),
        ("attribute-type", "Integer", "Real"),
        ("multiplicity", "Unit[1]", "Unit[2]"),
        ("rename", "count", "quantity"),
        (
            "add-part",
            "part secondUnit",
            "part thirdUnit : Unit; part secondUnit",
        ),
        (
            "remove-connection",
            "connection cable : Cable connect firstUnit to secondUnit;",
            "",
        ),
        (
            "add-connection",
            "connection cable",
            "connection extra : Cable connect firstUnit to secondUnit; connection cable",
        ),
    ] {
        eprintln!("oracle: {name} incremental");
        let mut branch = ProjectWorkspace::from_revision(base.clone());
        let start = source.find(old).unwrap();
        let started = Instant::now();
        let incremental = branch
            .apply(
                base.revision(),
                [support::edit(
                    &base,
                    "Edited.sysml",
                    start,
                    start + old.len(),
                    replacement,
                )],
            )
            .unwrap();
        let incremental_ms = started.elapsed().as_millis();
        eprintln!("oracle: {name} full");
        let started = Instant::now();
        let full = agq_modeling_workspace::testing::full_rebuild(&incremental).unwrap();
        let full_ms = started.elapsed().as_millis();
        assert_eq!(
            incremental.semantic_fingerprint().unwrap(),
            full.semantic_fingerprint().unwrap(),
            "{name}"
        );
        let left = incremental.semantic_model().unwrap();
        let right = full.semantic_model().unwrap();
        assert!(
            left.elements().eq(right.elements()),
            "canonical records and provenance: {name}"
        );
        assert!(
            left.association_occurrences()
                .eq(right.association_occurrences()),
            "association identity/order: {name}"
        );
        assert_eq!(
            incremental
                .producer_closure()
                .map(|c| c.semantic_closure_digest()),
            full.producer_closure().map(|c| c.semantic_closure_digest())
        );
        assert_eq!(incremental.references().len(), full.references().len());
        for (left, right) in incremental.references().iter().zip(full.references()) {
            assert_eq!(left.relationship, right.relationship);
            assert_eq!(left.specific, right.specific);
            assert_eq!(left.name, right.name);
            assert_eq!(left.origin, right.origin);
            assert_query(&left.resolution, &right.resolution);
        }
        // Diagnostics include native evidence; only the newly minted kernel
        // revision label is normalized. Source and semantic identities are exact.
        let normalized = format!("{:?}", full.diagnostics()).replace(
            &format!("{:?}", full.kernel_revision().unwrap()),
            &format!("{:?}", incremental.kernel_revision().unwrap()),
        );
        assert_eq!(format!("{:?}", incremental.diagnostics()), normalized);
        let path = agq_kerml_semantics::QualifiedName {
            absolute: false,
            segments: vec!["Edited".into(), "Assembly".into()],
        };
        let left_lookup = incremental
            .kerml_queries()
            .unwrap()
            .lookup_path(incremental.root(), &path);
        let right_lookup = full
            .kerml_queries()
            .unwrap()
            .lookup_path(full.root(), &path);
        assert_query(&left_lookup, &right_lookup);
        let owners: std::collections::BTreeSet<_> = left_lookup
            .value
            .iter()
            .map(|member| member.element)
            .collect();
        assert_eq!(
            owners.len(),
            1,
            "fixture retains its canonical Assembly owner even when Working"
        );
        let owner = *owners.first().unwrap();
        let l = incremental.sysml_queries().unwrap().effective_usages(owner);
        let r = full.sysml_queries().unwrap().effective_usages(owner);
        assert_query(&l.kerml, &r.kerml);
        assert_eq!(l.completeness(), r.completeness());
        assert_eq!(l.pending, r.pending);
        assert_eq!(l.diagnostics, r.diagnostics);
        assert_eq!(l.rejected_targets, r.rejected_targets);
        assert_eq!(l.filtered_targets, r.filtered_targets);
        assert_eq!(l.supporting_queries.len(), r.supporting_queries.len());
        for (left, right) in l.supporting_queries.iter().zip(&r.supporting_queries) {
            assert_query(left, right);
        }
        assert_eq!(l.supporting_names.len(), r.supporting_names.len());
        for (left, right) in l.supporting_names.iter().zip(&r.supporting_names) {
            assert_query(left, right);
        }
        assert!(l.observations.keys().eq(r.observations.keys()));
        for (fact, left) in &l.observations {
            assert_query(left, &r.observations[fact]);
        }
        assert_eq!(incremental.compilation_work().documents_reparsed, 1);
        assert_eq!(incremental.compilation_work().documents_lowered, 1);
        assert!(full.compilation_work().documents_lowered >= 3);
        assert_eq!(incremental.edit_frontier().source_revisions.len(), 1);
        eprintln!(
            "{name}: incremental_ms={incremental_ms} full_ms={full_ms} incremental_work={:?} full_work={:?}",
            incremental.compilation_work(),
            full.compilation_work()
        );
        eprintln!(
            "{name}: incremental_phases={} full_phases={}",
            serde_json::to_string(incremental.compilation_timings()).unwrap(),
            serde_json::to_string(full.compilation_timings()).unwrap()
        );
    }
}
