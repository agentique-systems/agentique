//! Explicit acceptance gate. Missing caches fail when this ignored gate is
//! requested; ordinary unit tests do not replay either standard publication.
use super::*;
use crate::{ProjectChange, ProjectRevision, SourceLanguage, SourceProject};
use agq_sysml_semantics::{
    RequirementCaseRole, StandardSysmlRole, StateSubactionKind, SysmlQueries, SysmlQueryResult,
    SysmlSemanticContext,
};
use std::{collections::BTreeMap, fs::File, path::Path};

fn complete<T: std::fmt::Debug>(answer: &SysmlQueryResult<T>) {
    assert_eq!(answer.completeness(), Completeness::Complete, "{answer:?}");
}

fn authored_named(model: &ModelView, name: &str) -> ElementId {
    let ids: Vec<_> = model
        .elements()
        .filter(|record| {
            matches!(
                record.origin(),
                Origin::Declared(DeclaredOrigin::Authored { .. })
            ) && model
                .navigation_slot(record.id(), p::ELEMENT_DECLARED_NAME)
                .is_some_and(|slot| {
                    slot.value()
                        .values()
                        .any(|value| value == &Value::String(name.into()))
                })
        })
        .map(|record| record.id())
        .collect();
    assert_eq!(ids.len(), 1, "authored declaration {name}");
    ids[0]
}

fn names(queries: &SysmlQueries<'_>, ids: impl IntoIterator<Item = ElementId>) -> BTreeSet<String> {
    ids.into_iter()
        .filter_map(|id| {
            queries
                .model()
                .navigation_slot(id, p::ELEMENT_DECLARED_NAME)
                .and_then(|slot| match slot.value() {
                    SlotValue::Scalar(Value::String(name)) => Some(name.clone()),
                    _ => None,
                })
        })
        .collect()
}

fn assert_revision(revision: &ProjectRevision, accepted: &CanonicalSysmlSystemsLibrary) {
    let status = revision
        .producer_status()
        .expect("accepted authored scheduler");
    assert!(status.converged, "{status:?}");
    assert_eq!(status.completeness, Completeness::Complete, "{status:?}");
    assert!(
        revision.is_complete_slice(),
        "{:?}",
        revision.semantic_diagnostics()
    );
    assert!(revision.semantic_diagnostics().is_empty());
    assert!(revision.producer_closure().is_some());
    assert!(Arc::ptr_eq(
        revision.snapshot().immutable_dependency().unwrap(),
        accepted.project_snapshot().immutable_dependency().unwrap()
    ));
    assert_eq!(
        status.counters.declared_subjects,
        revision
            .snapshot()
            .model()
            .elements()
            .filter(|record| !revision.snapshot().is_dependency_element(record.id()))
            .count()
    );
    for reference in revision.references() {
        assert_eq!(
            reference.resolution.completeness,
            Completeness::Complete,
            "{reference:?}"
        );
        assert!(
            matches!(reference.resolution.value, Resolution::Resolved(_)),
            "{reference:?}"
        );
    }
    let q = revision.sysml_queries().unwrap();
    for (definition, role) in [
        ("ProjectWorkspace", StandardSysmlRole::Part),
        ("SemanticState", StandardSysmlRole::Item),
        ("SemanticQuery", StandardSysmlRole::Port),
        ("ValidateRevision", StandardSysmlRole::Action),
        ("Serving", StandardSysmlRole::StateAction),
    ] {
        let parents = q.effective_supertypes(authored_named(q.model(), definition));
        complete(&parents);
        assert!(
            parents
                .value()
                .contains(&accepted.bindings().targets()[&role]),
            "{definition}: {parents:?}"
        );
    }
}

fn architecture_invariants(q: &SysmlQueries<'_>) {
    let model = q.model();
    let deps = |owner: &str| {
        let members = q.owned_usages(authored_named(model, owner));
        complete(&members);
        let mut targets = BTreeSet::new();
        for &member in members.value() {
            if model.element(member).unwrap().metaclass() == s::PART_USAGE
                && model
                    .navigation_slot(member, p::FEATURE_IS_COMPOSITE)
                    .is_some_and(|slot| slot.value() == &SlotValue::Scalar(Value::Boolean(false)))
            {
                let types = q.kerml().direct_feature_types(member);
                assert_eq!(types.completeness, Completeness::Complete);
                targets.extend(names(q, types.value));
            }
        }
        targets
    };
    assert!(deps("SemanticKernel").is_empty());
    assert_eq!(
        deps("KerMLEngine"),
        BTreeSet::from(["SemanticKernel".into()])
    );
    assert_eq!(deps("SysMLEngine"), BTreeSet::from(["KerMLEngine".into()]));
    assert_eq!(deps("ViewService"), BTreeSet::from(["QueryService".into()]));
    let workspace = q.effective_usages(authored_named(model, "ProjectWorkspace"));
    complete(&workspace);
    for declaration in ["kermlPublication", "systemsPublication", "languageQueries"] {
        assert!(
            workspace
                .value()
                .contains(&authored_named(model, declaration))
        );
    }
    let input = q.effective_item_definitions(authored_named(model, "validatedInput"));
    complete(&input);
    assert!(
        input
            .value()
            .contains(&authored_named(model, "ValidatedSemanticState"))
    );
    assert_eq!(
        deps("SimulationRuntime"),
        BTreeSet::from(["ExecutionIR".into()])
    );
}

fn rich_summary(q: &SysmlQueries<'_>) -> BTreeMap<String, BTreeSet<String>> {
    let mut summary = BTreeMap::new();
    for owner in [
        "ProjectWorkspace",
        "IncrementalWorkspace",
        "ModelingPlatform",
        "ValidateRevision",
        "Serving",
        "ImmutableRevisions",
    ] {
        let usages = q.effective_usages(authored_named(q.model(), owner));
        complete(&usages);
        summary.insert(
            format!("usages/{owner}"),
            names(q, usages.value().iter().copied()),
        );
    }
    for (label, answer) in [
        (
            "ports",
            q.effective_ports(authored_named(q.model(), "IncrementalWorkspace")),
        ),
        (
            "attribute",
            q.effective_attribute_definitions(authored_named(q.model(), "acceptedRevision")),
        ),
        (
            "part",
            q.effective_part_definitions(authored_named(q.model(), "workspace")),
        ),
        (
            "item",
            q.effective_item_definitions(authored_named(q.model(), "checkedState")),
        ),
        (
            "redefinition",
            q.effective_redefined_features(authored_named(q.model(), "acceptedRevision")),
        ),
        (
            "interface",
            q.effective_interface_ends(authored_named(q.model(), "SemanticAccess")),
        ),
        (
            "ends",
            q.effective_connection_related_features(authored_named(q.model(), "queryConnection")),
        ),
        (
            "parameters",
            q.effective_parameters(authored_named(q.model(), "ValidateRevision")),
        ),
        (
            "entry",
            q.state_actions(
                authored_named(q.model(), "Serving"),
                StateSubactionKind::Entry,
            ),
        ),
        (
            "do",
            q.state_actions(authored_named(q.model(), "Serving"), StateSubactionKind::Do),
        ),
        (
            "exit",
            q.state_actions(
                authored_named(q.model(), "Serving"),
                StateSubactionKind::Exit,
            ),
        ),
        (
            "subject",
            q.requirement_case_features(
                authored_named(q.model(), "ImmutableRevisions"),
                RequirementCaseRole::Subject,
            ),
        ),
    ] {
        complete(&answer);
        summary.insert(label.into(), names(q, answer.value().iter().copied()));
    }
    let inherited = q.effective_usages(authored_named(q.model(), "IncrementalWorkspace"));
    assert_eq!(
        inherited
            .value()
            .iter()
            .filter(|&&id| names(q, [id]).contains("workspaceQuery"))
            .count(),
        1,
        "inherited lookup retains the one canonical member"
    );
    assert!(
        inherited
            .value()
            .contains(&authored_named(q.model(), "workspaceQuery"))
    );
    assert!(
        inherited
            .value()
            .contains(&authored_named(q.model(), "acceptedRevision"))
    );
    assert!(
        !inherited
            .value()
            .contains(&authored_named(q.model(), "revisionNumber"))
    );
    let related =
        q.effective_connection_related_features(authored_named(q.model(), "queryConnection"));
    complete(&related);
    assert_eq!(
        related.value(),
        &vec![
            authored_named(q.model(), "clientQueries"),
            authored_named(q.model(), "platformQueries")
        ],
        "authored endpoint order"
    );
    let path = q.effective_qualified_name(authored_named(q.model(), "acceptedRevision"));
    complete(&path);
    let segments = &path.value().as_ref().unwrap().segments;
    assert_eq!(
        segments,
        &vec![
            BTreeSet::from(["PlatformArchitecture".into()]),
            BTreeSet::from(["IncrementalWorkspace".into()]),
            BTreeSet::from(["acceptedRevision".into()])
        ]
    );
    summary
}

fn programmatic_equivalence(
    accepted: &Arc<CanonicalSysmlSystemsLibrary>,
    textual: &ProjectRevision,
) {
    let witness = accepted.producer_closed_dependency().unwrap();
    let snapshot =
        super::programmatic_vertical::programmatic_platform_on(witness.project_snapshot());
    // The independent builder allocates this unnamed root before any other element.
    let root = ElementId::from_u128(910_000);
    let roots = std::iter::once(root)
        .chain(accepted.roots().iter().copied())
        .chain(accepted.accepted_kerml().roots().iter().copied())
        .collect();
    let extension = agq_sysml_semantics::SysmlProducerExtension::new(
        accepted.identity().dependencies.sysml_profile,
        accepted.bindings().clone(),
        roots,
    );
    let closed = agq_kerml_semantics::close_result_structure_with_extension(
        &snapshot,
        Default::default(),
        |overlay| {
            witness
                .project_overlay_context(overlay, &[root])
                .map_err(agq_kerml_semantics::PublicationOverlayError::Context)
        },
        &extension,
        |_, _, _, _| {},
        |_| {},
    )
    .unwrap();
    assert!(closed.converged, "{:?}", closed.stages);
    assert_eq!(
        closed.completeness,
        Completeness::Complete,
        "{:?}",
        closed.stages
    );
    let context = witness
        .project_overlay_context(&closed.overlay, &[root])
        .unwrap()
        .with_producer_closure(closed.certificate.unwrap())
        .unwrap();
    let q = SysmlQueries::new(
        SysmlSemanticContext::for_closed_dependency(
            context,
            &accepted.identity().dependencies,
            accepted.bindings().clone(),
        )
        .unwrap(),
    );
    assert_eq!(
        rich_summary(&textual.sysml_queries().unwrap()),
        rich_summary(&q)
    );
    assert_ne!(
        authored_named(textual.semantic_model(), "workspaceQuery"),
        authored_named(q.model(), "workspaceQuery")
    );
    assert!(Arc::ptr_eq(
        snapshot.immutable_dependency().unwrap(),
        textual.snapshot().immutable_dependency().unwrap()
    ));
}

fn insert(
    project: &mut SourceProject,
    document: DocumentId,
    before: &str,
    text: &str,
) -> Arc<ProjectRevision> {
    let source = project.current().document(document).unwrap().source();
    let offset = source.find(before).unwrap() as u64;
    project
        .apply(
            project.current().revision(),
            [ProjectChange::Edit {
                document,
                edit: crate::syntax::TextEdit {
                    range: crate::syntax::ByteRange::new(offset, offset).unwrap(),
                    replacement: text.into(),
                },
            }],
        )
        .unwrap()
}

#[test]
#[ignore = "requires exact accepted KerML and Systems caches; never rebuilds standards"]
fn accepted_agentique_self_model_closes_queries_edits_and_matches_programmatic_semantics() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root).unwrap();
    let kerml_path = std::env::var_os("AGENTIQUE_KERML_CACHE")
        .expect("AGENTIQUE_KERML_CACHE is required for this requested acceptance gate");
    let systems_path = std::env::var_os("AGENTIQUE_SYSTEMS_CACHE")
        .expect("AGENTIQUE_SYSTEMS_CACHE is required for this requested acceptance gate");
    let kerml = Arc::new(
        CanonicalKermlStandardLibraries::restore_cache(File::open(kerml_path).unwrap(), &sources)
            .unwrap(),
    );
    let accepted = Arc::new(
        CanonicalSysmlSystemsLibrary::restore_cache(
            File::open(systems_path).unwrap(),
            &sources,
            kerml,
        )
        .unwrap(),
    );
    let mut project =
        SourceProject::with_accepted_sysml_standard_libraries(accepted.clone()).unwrap();
    let r1 = project
        .apply(
            project.current().revision(),
            super::agentique_self_model::DOCUMENTS
                .into_iter()
                .map(|(path, source)| ProjectChange::Add {
                    path: path.into(),
                    language: SourceLanguage::SysMl,
                    source: source.into(),
                }),
        )
        .unwrap();
    assert_revision(&r1, &accepted);
    architecture_invariants(&r1.sysml_queries().unwrap());
    let original = rich_summary(&r1.sysml_queries().unwrap());
    programmatic_equivalence(&accepted, &r1);
    let document = r1.document_at("ModelingPlatform.sysml").unwrap().id();
    let retained_query = authored_named(r1.semantic_model(), "workspaceQuery");
    let r2 = insert(
        &mut project,
        document,
        "        attribute revisionNumber",
        "        port auditPort : ArchitectureContracts::SemanticQuery;\n",
    );
    assert_revision(&r2, &accepted);
    assert_eq!(
        authored_named(r2.semantic_model(), "workspaceQuery"),
        retained_query
    );
    let audit_port = authored_named(r2.semantic_model(), "auditPort");
    let r3 = insert(
        &mut project,
        document,
        "        attribute acceptedRevision",
        "        port auditedPort : ArchitectureContracts::SemanticQuery :>> ProjectWorkspace::auditPort;\n",
    );
    assert_revision(&r3, &accepted);
    assert_eq!(
        authored_named(r3.semantic_model(), "workspaceQuery"),
        retained_query
    );
    assert_eq!(authored_named(r3.semantic_model(), "auditPort"), audit_port);
    assert!(std::ptr::eq(
        r1.document_at("Contracts.sysml").unwrap(),
        r2.document_at("Contracts.sysml").unwrap()
    ));
    assert!(std::ptr::eq(
        r2.document_at("Contracts.sysml").unwrap(),
        r3.document_at("Contracts.sysml").unwrap()
    ));
    for (revision, expected_port) in [
        (&r2, audit_port),
        (&r3, authored_named(r3.semantic_model(), "auditedPort")),
    ] {
        let q = revision.sysml_queries().unwrap();
        let ports = q.effective_ports(authored_named(q.model(), "IncrementalWorkspace"));
        complete(&ports);
        assert!(ports.value().contains(&expected_port));
    }
    assert_eq!(
        rich_summary(&r1.sysml_queries().unwrap()),
        original,
        "old revision retained"
    );
    assert!(
        !r2.document(document)
            .unwrap()
            .source()
            .contains("auditedPort")
    );
    assert!(Arc::ptr_eq(project.revision(r1.revision()).unwrap(), &r1));
    assert!(Arc::ptr_eq(
        project.accepted_sysml_standard_library().unwrap(),
        &accepted
    ));
    let second = SourceProject::with_accepted_sysml_standard_libraries(accepted.clone()).unwrap();
    assert!(Arc::ptr_eq(
        second.current().snapshot().immutable_dependency().unwrap(),
        r1.snapshot().immutable_dependency().unwrap()
    ));
    std::thread::scope(|scope| {
        for revision in [&r1, &r2, &r3] {
            scope.spawn(move || {
                let q = revision.sysml_queries().unwrap();
                let ports = q.effective_ports(authored_named(q.model(), "IncrementalWorkspace"));
                complete(&ports);
                assert!(ports.value().contains(&retained_query));
            });
        }
    });
    println!(
        "Agentique self-model: {} documents, {} Complete mandatory references, producer closure Complete, semantic architecture invariants pass, programmatic equivalence pass, 3 immutable revisions, shared standards",
        r1.documents().count(),
        r1.references().len()
    );
}
