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
    }
}
