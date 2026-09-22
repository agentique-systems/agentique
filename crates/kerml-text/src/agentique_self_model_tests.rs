//! Permanent multi-document architecture fixture. Current-graph tests do not
//! substitute for accepted Systems publication and authored producer closure.
use super::*;
use std::collections::BTreeMap;

const DOCUMENTS: [(&str, &str); 5] = [
    (
        "Contracts.sysml",
        include_str!("../../../models/agentique/Contracts.sysml"),
    ),
    (
        "LanguageEngine.sysml",
        include_str!("../../../models/agentique/LanguageEngine.sysml"),
    ),
    (
        "ModelingPlatform.sysml",
        include_str!("../../../models/agentique/ModelingPlatform.sysml"),
    ),
    (
        "ExecutionRuntime.sysml",
        include_str!("../../../models/agentique/ExecutionRuntime.sysml"),
    ),
    (
        "Agentique.sysml",
        include_str!("../../../models/agentique/Agentique.sysml"),
    ),
];

fn syntax_documents() -> Vec<production::Document> {
    DOCUMENTS
        .iter()
        .enumerate()
        .map(|(index, (path, source))| {
            let syntax = production::parse_sysml_with_profile(
                production::SysmlSyntaxProfile::OperationalV2,
                DocumentId::from_u128(930_000 + index as u128),
                SourceRevisionId::from_u128(940_000 + index as u128),
                *source,
                Default::default(),
            )
            .unwrap();
            assert!(syntax.is_complete(), "{path}: {:?}", syntax.diagnostics());
            syntax
        })
        .collect()
}

fn lower_documents(documents: &[production::Document]) -> LibraryDraft {
    let inputs: Vec<_> = documents
        .iter()
        .map(|syntax| SourceInput {
            syntax,
            library: None,
            sysml: true,
        })
        .collect();
    let base = Snapshot::new(Arc::new(
        agq_sysml::registry_for_profile(BaselineProfile::OPERATIONAL_V9).unwrap(),
    ));
    library::refinement::refine(
        |resolved| {
            construction::construct_on(
                &inputs,
                resolved,
                BaselineProfile::OPERATIONAL_V9,
                base.clone(),
                Some((
                    ElementId::from_u128(950_000),
                    DeclaredOrigin::Authored { source: None },
                )),
            )
        },
        |draft| {
            Ok(KerMlQueries::new(
                SemanticContext::for_construction(draft.candidate(), options(), BTreeSet::new())
                    .unwrap(),
            )
            .status_queries())
        },
        ReferenceRefinementStrategy::DependencyDriven,
        |_| {},
    )
    .unwrap()
}

fn names(model: &ModelView, ids: impl IntoIterator<Item = ElementId>) -> BTreeSet<String> {
    ids.into_iter()
        .filter_map(|id| {
            model
                .navigation_slot(id, p::ELEMENT_DECLARED_NAME)
                .and_then(|slot| match slot.value() {
                    SlotValue::Scalar(Value::String(name)) => Some(name.clone()),
                    _ => None,
                })
        })
        .collect()
}

fn dependency_names(query: &KerMlQueries<'_>, model: &ModelView, owner: &str) -> BTreeSet<String> {
    let members = query.effective_features(named(model, owner));
    assert_eq!(members.completeness, Completeness::Complete, "{members:?}");
    let mut targets = BTreeSet::new();
    for member in members.value {
        if model.element(member).unwrap().metaclass() == s::PART_USAGE
            && model
                .navigation_slot(member, p::FEATURE_IS_COMPOSITE)
                .is_some_and(|slot| slot.value() == &SlotValue::Scalar(Value::Boolean(false)))
        {
            let types = query.direct_feature_types(member);
            assert_eq!(types.completeness, Completeness::Complete, "{types:?}");
            targets.extend(names(model, types.value));
        }
    }
    targets
}

#[test]
fn agentique_self_model_parses_lowers_and_checks_architectural_dependencies() {
    let documents = syntax_documents();
    let draft = lower_documents(&documents);
    assert!(
        draft.candidate().obligations().is_empty(),
        "{:?}",
        draft.candidate().obligations()
    );
    let snapshot = draft.strict_snapshot().unwrap();
    let model = snapshot.model();
    let queries = q(&snapshot);
    assert!(
        draft.producer_closure().is_none(),
        "structural acceptance is distinct from producer closure"
    );
    assert!(!draft.references().is_empty());
    for reference in draft.references() {
        let answer = queries.lookup_relationship_target(
            reference.relationship,
            reference.property,
            &reference.name,
        );
        assert_eq!(
            answer.completeness,
            Completeness::Complete,
            "{reference:?}: {answer:?}"
        );
        assert_eq!(answer.value.len(), 1, "{reference:?}: {answer:?}");
        let endpoint = if reference.membership_target {
            answer.value[0].membership
        } else {
            answer.value[0].element
        };
        assert!(
            model
                .navigation_slot(reference.relationship, reference.property)
                .unwrap()
                .value()
                .values()
                .any(|value| value == &Value::Reference(endpoint)),
            "canonical endpoint mismatch: {reference:?}"
        );
    }
    assert!(dependency_names(&queries, model, "SemanticKernel").is_empty());
    for (owner, expected) in [
        ("KerMLEngine", vec!["SemanticKernel"]),
        ("SysMLEngine", vec!["KerMLEngine"]),
        (
            "ProjectWorkspace",
            vec!["StandardLibraryManager", "SysMLEngine"],
        ),
        ("ViewService", vec!["QueryService"]),
        ("ExecutionCompiler", vec!["ValidationService"]),
        ("SimulationRuntime", vec!["ExecutionIR"]),
    ] {
        assert_eq!(
            dependency_names(&queries, model, owner),
            expected.into_iter().map(str::to_owned).collect(),
            "{owner}"
        );
    }
    for publication in ["kermlPublication", "systemsPublication"] {
        assert_eq!(
            queries.owner(named(model, publication)).value,
            Some(named(model, "ProjectWorkspace"))
        );
    }
    assert_eq!(
        queries
            .direct_feature_types(named(model, "validatedInput"))
            .value,
        [named(model, "ValidatedSemanticState")]
    );
    assert_eq!(
        names(
            model,
            queries.effective_features(named(model, "Agentique")).value
        ),
        ["execution", "languageSubsystem", "modeling"]
            .map(str::to_owned)
            .into()
    );
    for record in model.elements() {
        if record.id() == ElementId::from_u128(950_000) {
            continue;
        }
        let source = &draft.source_map()[&FactKey::Element(record.id())];
        assert!(
            documents
                .iter()
                .any(|document| document.document() == source.document)
        );
        assert!(source.syntax_node.is_some());
    }
}

#[test]
fn agentique_rich_platform_preserves_inheritance_redefinition_connections_and_ids() {
    let documents = syntax_documents();
    let draft = lower_documents(&documents);
    let snapshot = draft.strict_snapshot().unwrap();
    let model = snapshot.model();
    let before: BTreeSet<_> = model.elements().map(|record| record.id()).collect();
    let queries = q(&snapshot);
    let inherited = queries.effective_features(named(model, "IncrementalWorkspace"));
    assert_eq!(inherited.completeness, Completeness::Complete);
    assert!(inherited.value.contains(&named(model, "workspaceQuery")));
    assert!(inherited.value.contains(&named(model, "acceptedRevision")));
    assert!(!inherited.value.contains(&named(model, "revisionNumber")));
    assert_eq!(
        queries
            .redefined_features(named(model, "acceptedRevision"))
            .value,
        [named(model, "revisionNumber")]
    );
    let endpoints = queries.connector_endpoints(named(model, "queryConnection"));
    assert_eq!(
        endpoints.completeness,
        Completeness::Complete,
        "{endpoints:?}"
    );
    assert_eq!(
        endpoints.value,
        [
            named(model, "clientQueries"),
            named(model, "platformQueries")
        ]
    );
    for end in queries
        .connector_related_structure(named(model, "queryConnection"))
        .value
        .ends
    {
        assert_eq!(
            model.element(end).unwrap().metaclass(),
            s::PORT_USAGE,
            "interface end {end}"
        );
    }
    for (name, class) in [
        ("SemanticAccess", s::INTERFACE_DEFINITION),
        ("queryConnection", s::INTERFACE_USAGE),
        ("ValidateRevision", s::ACTION_DEFINITION),
        ("Serving", s::STATE_DEFINITION),
        ("ImmutableRevisions", s::REQUIREMENT_DEFINITION),
        ("preservesPriorState", s::CONSTRAINT_USAGE),
    ] {
        assert_eq!(
            model.element(named(model, name)).unwrap().metaclass(),
            class
        );
    }
    assert_eq!(
        names(
            model,
            queries.effective_features(named(model, "Serving")).value
        ),
        ["checkRevision", "startServing", "stopServing"]
            .map(str::to_owned)
            .into()
    );
    assert_eq!(
        names(
            model,
            queries
                .effective_features(named(model, "ImmutableRevisions"))
                .value
        ),
        ["preservesPriorState", "subjectWorkspace"]
            .map(str::to_owned)
            .into()
    );
    assert_eq!(before, model.elements().map(|record| record.id()).collect());
    let reconstructed = lower_documents(&documents).strict_snapshot().unwrap();
    assert_eq!(
        before,
        reconstructed
            .model()
            .elements()
            .map(|record| record.id())
            .collect(),
        "the same syntax identities reproduce the same canonical identities"
    );
}

#[test]
fn agentique_implementation_traceability_is_separate_and_resolves_existing_paths() {
    let manifest: serde_json::Value = serde_json::from_str(include_str!(
        "../../../models/agentique/implementation-map.json"
    ))
    .unwrap();
    let snapshot = lower_documents(&syntax_documents())
        .strict_snapshot()
        .unwrap();
    let query = q(&snapshot);
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let named_elements: BTreeMap<_, _> = snapshot
        .model()
        .elements()
        .filter_map(|record| {
            let own = names(snapshot.model(), [record.id()]).into_iter().next()?;
            let parent = query.owner(record.id()).value?;
            let parent_name = names(snapshot.model(), [parent]).into_iter().next()?;
            Some((format!("{parent_name}::{own}"), record.id()))
        })
        .collect();
    for (path, entry) in manifest["elements"].as_object().unwrap() {
        assert!(
            named_elements.contains_key(path),
            "missing architecture element {path}"
        );
        let paths = entry["paths"].as_array().unwrap();
        if entry["status"] == "planned-gen2" {
            assert!(paths.is_empty());
        }
        for implementation in paths {
            let implementation = implementation.as_str().unwrap();
            assert!(!implementation.contains(".."));
            assert!(
                root.join(implementation).exists(),
                "missing implementation {implementation}"
            );
        }
    }
}

#[test]
fn agentique_rich_platform_matches_independent_programmatic_semantics() {
    let textual = lower_documents(&syntax_documents())
        .strict_snapshot()
        .unwrap();
    let programmatic = super::programmatic_vertical::programmatic_platform();
    let summaries = [&textual, &programmatic].map(|snapshot| {
        let query = q(snapshot);
        let model = snapshot.model();
        let mut summary = BTreeMap::new();
        for owner in [
            "ProjectWorkspace",
            "IncrementalWorkspace",
            "ModelingPlatform",
            "ValidateRevision",
            "Serving",
            "ImmutableRevisions",
        ] {
            let answer = query.effective_features(named(model, owner));
            assert_eq!(
                answer.completeness,
                Completeness::Complete,
                "{owner}: {answer:?}"
            );
            let members = answer.value.clone();
            summary.insert(format!("members/{owner}"), names(model, members.clone()));
            for member in members {
                let Some(name) = names(model, [member]).into_iter().next() else {
                    continue;
                };
                let types = query.direct_feature_types(member);
                assert_eq!(
                    types.completeness,
                    Completeness::Complete,
                    "{name}: {types:?}"
                );
                summary.insert(format!("types/{name}"), names(model, types.value));
            }
        }
        let redefined = query.redefined_features(named(model, "acceptedRevision"));
        assert_eq!(redefined.completeness, Completeness::Complete);
        summary.insert(
            "redefined/acceptedRevision".into(),
            names(model, redefined.value),
        );
        let endpoints = query.connector_endpoints(named(model, "queryConnection"));
        assert_eq!(endpoints.completeness, Completeness::Complete);
        assert_eq!(
            endpoints.value,
            [
                named(model, "clientQueries"),
                named(model, "platformQueries")
            ]
        );
        summary
    });
    assert_eq!(summaries[0], summaries[1]);
    assert_ne!(
        named(textual.model(), "workspaceQuery"),
        named(programmatic.model(), "workspaceQuery")
    );
}
