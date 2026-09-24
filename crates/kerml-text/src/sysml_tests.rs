use super::*;
use agq_kerml::{BaselineProfile, properties as p};
use agq_kerml_semantics::{SemanticContext, SemanticOptions};
use agq_kernel::{
    DocumentId, ModelView, SourceRevisionId,
    provenance::{FactKey, Origin},
    value::{SlotValue, Value},
};
use agq_sysml::{classes as s, properties as sp};

#[test]
fn actions_trigger_lowering_preserves_input_parameter_defaults() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root).unwrap();
    let source = sources
        .documents()
        .find(|s| s.path() == "Systems Library/Actions.sysml")
        .unwrap();
    let syntax = production::parse_sysml_with_profile(
        production::SysmlSyntaxProfile::OperationalV2,
        source.document(),
        source.revision(),
        source.source(),
        Default::default(),
    )
    .unwrap();
    let base = Snapshot::new(Arc::new(
        agq_sysml::registry_for_profile(BaselineProfile::OPERATIONAL_V9).unwrap(),
    ));
    let draft = construction::construct_on(
        &[SourceInput {
            syntax: &syntax,
            library: Some(source),
            sysml: true,
        }],
        &Default::default(),
        BaselineProfile::OPERATIONAL_V9,
        base,
        None,
    )
    .unwrap();
    let model = draft.candidate().model();
    let trigger = ElementId::from_u128(0x35c0b23fb73e5c7e9079b8c8bc9c39e6);
    let agq_kernel::metamodel::ValueKind::Enumeration(domain) = model
        .registry()
        .property(p::FEATURE_DIRECTION)
        .unwrap()
        .value_kind
    else {
        unreachable!()
    };
    let input = *model
        .registry()
        .enumeration(domain)
        .unwrap()
        .literals
        .iter()
        .find(|(_, name)| name.as_str() == "in")
        .unwrap()
        .0;
    for (number, source_text) in [
        (trigger.as_u128(), "apayload: Anything via receiver"),
        (0x70506936f8f950579e3637230a4e02df, "apayload: Anything"),
        (0xa1ff5677154c54fab0b478c9a54a3c73, "receiver"),
    ] {
        let id = ElementId::from_u128(number);
        let record = model.element(id).unwrap();
        assert_eq!(
            record.metaclass(),
            if id == trigger {
                s::ACCEPT_ACTION_USAGE
            } else {
                s::REFERENCE_USAGE
            }
        );
        assert_eq!(
            model
                .navigation_slot(id, p::FEATURE_IS_COMPOSITE)
                .unwrap()
                .value(),
            &SlotValue::Scalar(Value::Boolean(id == trigger))
        );
        assert_eq!(
            model
                .navigation_slot(id, p::FEATURE_IS_PORTION)
                .unwrap()
                .value(),
            &SlotValue::Scalar(Value::Boolean(false))
        );
        assert_eq!(
            model
                .navigation_slot(id, p::FEATURE_DIRECTION)
                .map(|slot| slot.value()),
            (id != trigger).then_some(&SlotValue::Scalar(Value::Enumeration(input)))
        );
        let memberships: Vec<_> = model
            .incoming_for_property(id, p::RELATIONSHIP_OWNED_RELATED_ELEMENT)
            .map(|incoming| incoming.source)
            .collect();
        assert_eq!(memberships.len(), 1);
        let membership = model.element(memberships[0]).unwrap();
        assert_eq!(
            membership.metaclass(),
            if id == trigger {
                s::TRANSITION_FEATURE_MEMBERSHIP
            } else {
                c::PARAMETER_MEMBERSHIP
            }
        );
        let owners: Vec<_> = model
            .incoming_for_property(membership.id(), p::ELEMENT_OWNED_RELATIONSHIP)
            .map(|owner| owner.source)
            .collect();
        assert_eq!(
            owners,
            vec![if id == trigger {
                ElementId::from_u128(0x8cf3c953ad025312bf90b8ab9dca34ee)
            } else {
                trigger
            }]
        );
        let location = draft.source_map().get(&FactKey::Element(id)).unwrap();
        assert_eq!(
            &source.source()[location.range.start() as usize..location.range.end() as usize],
            source_text
        );
    }
}

#[path = "sysml_authority_tests.rs"]
mod authority;
#[path = "sysml_enumeration_tests.rs"]
mod enumeration;

#[path = "sysml_rich_vertical_tests.rs"]
mod rich_vertical;

#[path = "sysml_programmatic_vertical_tests.rs"]
mod programmatic_vertical;

#[path = "accepted_self_model_tests.rs"]
mod accepted_self_model;
#[path = "agentique_self_model_tests.rs"]
mod agentique_self_model;

fn parse(source: &str) -> production::Document {
    let parsed = production::parse_sysml(
        DocumentId::new(),
        SourceRevisionId::new(),
        source,
        Default::default(),
    )
    .unwrap();
    assert!(parsed.is_complete(), "{:?}", parsed.diagnostics());
    parsed
}
fn options() -> SemanticOptions {
    SemanticOptions {
        baseline_profile: BaselineProfile::OPERATIONAL_V9,
        exclude_implied: true,
    }
}
fn lower(syntax: &production::Document) -> LibraryDraft {
    let inputs = [SourceInput {
        syntax,
        library: None,
        sysml: true,
    }];
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
                None,
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
fn named(model: &ModelView, name: &str) -> ElementId {
    let ids: Vec<_> = model
        .elements()
        .filter(|record| {
            model
                .navigation_slot(record.id(), p::ELEMENT_DECLARED_NAME)
                .is_some_and(|slot| {
                    slot.value()
                        .values()
                        .any(|value| value == &Value::String(name.into()))
                })
        })
        .map(|r| r.id())
        .collect();
    assert_eq!(ids.len(), 1, "{name}");
    ids[0]
}
fn q(snapshot: &Snapshot) -> KerMlQueries<'_> {
    KerMlQueries::new(SemanticContext::for_snapshot(snapshot, options(), BTreeSet::new()).unwrap())
}

#[test]
fn sysml_vertical_preserves_classes_relationships_provenance_and_inherited_identity() {
    let syntax = parse(
        "part def Engine; part def Vehicle { part engine : Engine; } part def SportsCar :> Vehicle;",
    );
    let draft = lower(&syntax);
    let snapshot = draft.strict_snapshot().unwrap();
    let model = snapshot.model();
    let ids = ["Engine", "Vehicle", "engine", "SportsCar"].map(|name| named(model, name));
    assert_eq!(
        ids.map(|id| model.element(id).unwrap().metaclass()),
        [
            s::PART_DEFINITION,
            s::PART_DEFINITION,
            s::PART_USAGE,
            s::PART_DEFINITION
        ]
    );
    assert_eq!(
        model.instances(c::FEATURE_TYPING, true).unwrap().count(),
        1,
        "FeatureTyping is a transparent dispatch"
    );
    assert_eq!(
        model.instances(c::SUBCLASSIFICATION, true).unwrap().count(),
        1
    );
    let count = model.elements().count();
    let query = q(&snapshot);
    let typed = query.direct_feature_types(ids[2]);
    assert_eq!(typed.completeness, Completeness::Complete);
    assert_eq!(typed.value, vec![ids[0]]);
    let inherited = query.effective_features(ids[3]);
    assert_eq!(inherited.completeness, Completeness::Complete);
    assert_eq!(inherited.value, vec![ids[2]]);
    assert_eq!(model.elements().count(), count);
    for record in model.elements() {
        let source = &draft.source_map()[&FactKey::Element(record.id())];
        assert_eq!(source.document, syntax.document());
        assert!(source.syntax_node.is_some());
        assert!(matches!(
            record.origin(),
            Origin::Declared(DeclaredOrigin::Authored { source: Some(_) })
        ));
    }
    assert_eq!(
        model
            .navigation_slot(ids[2], p::FEATURE_IS_COMPOSITE)
            .unwrap()
            .value(),
        &SlotValue::Scalar(Value::Boolean(true))
    );
    assert!(
        model
            .navigation_slot(ids[2], sp::USAGE_MAY_TIME_VARY)
            .is_none()
    );
    assert!(
        model
            .navigation_slot(ids[2], sp::USAGE_IS_REFERENCE)
            .is_none()
    );
}

#[test]
fn port_synthesis_and_reference_modifiers_use_canonical_storage() {
    let syntax = parse(
        "port def Connector { out item signal; ref port peer; } attribute def Number; part def Box { attribute weight : Number; }",
    );
    let draft = lower(&syntax);
    let snapshot = draft.strict_snapshot().unwrap();
    let model = snapshot.model();
    let original = named(model, "Connector");
    let conjugated: Vec<_> = model
        .instances(s::CONJUGATED_PORT_DEFINITION, true)
        .unwrap()
        .map(|r| r.id())
        .collect();
    assert_eq!(conjugated.len(), 1);
    let conjugations: Vec<_> = model
        .instances(s::PORT_CONJUGATION, true)
        .unwrap()
        .map(|r| r.id())
        .collect();
    assert_eq!(conjugations.len(), 1);
    let query = q(&snapshot);
    assert_eq!(query.owner(conjugated[0]).value, Some(original));
    assert_eq!(
        query.owning_related_element(conjugations[0]).value,
        Some(conjugated[0])
    );
    assert_eq!(
        model
            .navigation_slot(
                conjugations[0],
                sp::PORT_CONJUGATION_ORIGINAL_PORT_DEFINITION
            )
            .unwrap()
            .value(),
        &SlotValue::Scalar(Value::Reference(original))
    );
    for name in ["signal", "peer", "weight"] {
        assert_eq!(
            model
                .navigation_slot(named(model, name), p::FEATURE_IS_COMPOSITE)
                .unwrap()
                .value(),
            &SlotValue::Scalar(Value::Boolean(false)),
            "{name}"
        );
    }
}

#[test]
fn all_pinned_systems_documents_construct_with_source_provenance() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root).unwrap();
    let base = Snapshot::new(Arc::new(
        agq_sysml::registry_for_profile(BaselineProfile::OPERATIONAL_V9).unwrap(),
    ));
    let mut constructed = 0;
    let mut structural_deficits = 0;
    for source in sources
        .documents()
        .filter(|source| source.language() == LibraryLanguage::SysMl)
    {
        let syntax = production::parse_sysml_with_profile(
            production::SysmlSyntaxProfile::OperationalV1,
            source.document(),
            source.revision(),
            source.source(),
            Default::default(),
        )
        .unwrap();
        assert!(
            syntax.is_complete(),
            "{}: {:?}",
            source.path(),
            syntax.diagnostics()
        );
        let result = construction::construct_on(
            &[SourceInput {
                syntax: &syntax,
                library: Some(source),
                sysml: true,
            }],
            &Default::default(),
            BaselineProfile::OPERATIONAL_V9,
            base.clone(),
            None,
        );
        match result {
            Ok(draft) => {
                constructed += 1;
                let model = draft.candidate().model();
                println!(
                    "{}: constructed {} elements, {} references, {} obligations",
                    source.path(),
                    model.elements().count(),
                    draft.references().len(),
                    draft.candidate().obligations().len()
                );
                for obligation in draft.candidate().obligations() {
                    if !draft.references().iter().any(|reference| {
                        reference.relationship == obligation.element
                            && reference.property == obligation.property
                    }) {
                        structural_deficits += 1;
                        let element = model.element(obligation.element).unwrap();
                        println!(
                            "  non-reference obligation: {}.{} {:?}",
                            model.registry().class(element.metaclass()).unwrap().name,
                            model.registry().property(obligation.property).unwrap().name,
                            draft.source_map()[&FactKey::Element(element.id())]
                        );
                    }
                }
                for element in model.elements() {
                    assert!(matches!(
                        element.origin(),
                        Origin::Declared(DeclaredOrigin::StandardLibrary { .. })
                    ));
                    let origin = &draft.source_map()[&FactKey::Element(element.id())];
                    assert_eq!(origin.document, source.document());
                    assert_eq!(origin.revision, source.revision());
                    assert!(origin.syntax_node.is_some());
                }
            }
            Err(error) => println!("{}: {}", source.path(), error),
        }
    }
    assert_eq!(constructed, 21);
    assert_eq!(structural_deficits, 0);
}

#[test]
fn source_reference_audit_outlives_construction_without_retaining_its_graph() {
    let syntax = parse(
        "package Architecture { part def Engine; part def Vehicle { part engine : Engine; } private alias Motor for Engine; }",
    );
    let draft = lower(&syntax);
    let construction = Arc::downgrade(draft.candidate_shared());
    let snapshot = draft.strict_snapshot().unwrap();
    let pending = draft.references().to_vec();
    let sources = draft.source_map().clone();
    drop(draft);
    assert!(construction.upgrade().is_none());

    let root = named(snapshot.model(), "Architecture");
    let engine = named(snapshot.model(), "Engine");
    let inputs = [SourceInput {
        syntax: &syntax,
        library: None,
        sysml: true,
    }];
    let (references, diagnostics) =
        source_references(&inputs, &pending, &sources, root, &q(&snapshot)).unwrap();
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    assert_eq!(references.len(), 2);
    for reference in &references {
        assert_eq!(reference.origin.document, syntax.document());
        assert_eq!(reference.resolution.completeness, Completeness::Complete);
        assert_eq!(reference.resolution.value, Resolution::Resolved(engine));
        assert_eq!(
            reference.origin,
            pending
                .iter()
                .find(|input| input.relationship == reference.relationship)
                .unwrap()
                .origin
        );
    }
    let alias = references
        .iter()
        .find(|reference| reference.kind == ReferenceKind::Alias)
        .unwrap();
    assert_eq!(alias.alias(), Some("Motor"));
    assert_eq!(alias.visibility, Visibility::Private);
}

#[test]
fn reference_metadata_preserves_private_aliases_and_imports_and_rejects_unknown_kinds() {
    let syntax = parse(
        "package Supply { part def Original; } package User { private import Supply::*; private alias Hidden for Supply::Original; }",
    );
    let draft = lower(&syntax);
    let snapshot = draft.strict_snapshot().unwrap();
    let import = snapshot
        .model()
        .instances(c::NAMESPACE_IMPORT, true)
        .unwrap()
        .next()
        .unwrap()
        .id();
    let metadata = reference_metadata(snapshot.model(), import, false).unwrap();
    assert_eq!(
        metadata,
        (ReferenceKind::NamespaceImport, None, Visibility::Private)
    );
    let alias = snapshot
        .model()
        .instances(c::MEMBERSHIP, false)
        .unwrap()
        .next()
        .unwrap()
        .id();
    assert_eq!(
        reference_metadata(snapshot.model(), alias, true).unwrap(),
        (
            ReferenceKind::Alias,
            Some("Hidden".into()),
            Visibility::Private
        )
    );
    assert!(
        reference_metadata(snapshot.model(), alias, false).is_err(),
        "arbitrary reference membership is never labeled Alias"
    );
}

#[test]
fn mixed_production_namespaces_compose_imports_visibility_shadowing_and_cycles() {
    let sysml = parse(
        r#"
        package BaseDefinitions {
            part def Visible;
            private part def Hidden;
        }
        package User {
            private import BaseDefinitions::*;
            public alias Model for Visible;
            part def Mixed :> Legacy::Old;
            package Inner {
                part def Visible;
                part selected : Visible;
            }
        }
        package CycleA { private import CycleB::*; part def AType; }
        package CycleB { private import CycleA::*; part def BType; }
    "#,
    );
    let kerml = production::parse(
        DocumentId::new(),
        SourceRevisionId::new(),
        "package Legacy { class Old; }",
        Default::default(),
    )
    .unwrap();
    assert!(kerml.is_complete());
    let inputs = [
        SourceInput {
            syntax: &sysml,
            library: None,
            sysml: true,
        },
        SourceInput {
            syntax: &kerml,
            library: None,
            sysml: false,
        },
    ];
    let root = ElementId::new();
    let base = Snapshot::new(Arc::new(
        agq_sysml::registry_for_profile(BaselineProfile::OPERATIONAL_V9).unwrap(),
    ));
    let draft = library::refinement::refine(
        |resolved| {
            construction::construct_on(
                &inputs,
                resolved,
                BaselineProfile::OPERATIONAL_V9,
                base.clone(),
                Some((root, DeclaredOrigin::Authored { source: None })),
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
    .unwrap();
    let snapshot = draft.strict_snapshot().unwrap();
    let query = q(&snapshot);
    let lookup = |scope, path: &[&str]| {
        let answer = query.lookup_path(
            scope,
            &agq_kerml_semantics::QualifiedName {
                absolute: false,
                segments: path.iter().map(|s| (*s).to_owned()).collect(),
            },
        );
        assert_eq!(
            answer.completeness,
            Completeness::Complete,
            "{path:?}: {answer:?}"
        );
        answer
            .value
            .into_iter()
            .map(|member| member.element)
            .collect::<Vec<_>>()
    };
    let visible = lookup(root, &["BaseDefinitions", "Visible"])[0];
    assert_eq!(lookup(root, &["User", "Model"]), vec![visible]);
    assert!(lookup(root, &["BaseDefinitions", "Hidden"]).is_empty());
    assert!(
        lookup(root, &["User", "Visible"]).is_empty(),
        "private import is not re-exported"
    );
    let user = lookup(root, &["User"])[0];
    assert_eq!(lookup(user, &["Visible"]), vec![visible]);
    assert!(lookup(user, &["Hidden"]).is_empty());
    let inner_visible = lookup(root, &["User", "Inner", "Visible"])[0];
    let selected = lookup(root, &["User", "Inner", "selected"])[0];
    assert_ne!(inner_visible, visible);
    let typing = query.direct_feature_types(selected);
    assert_eq!(typing.completeness, Completeness::Complete);
    assert_eq!(
        typing.value,
        vec![inner_visible],
        "nearer declaration shadows outer import"
    );
    let mixed = lookup(root, &["User", "Mixed"])[0];
    let old = lookup(root, &["Legacy", "Old"])[0];
    let bases = query.direct_specializations(mixed);
    assert_eq!(bases.completeness, Completeness::Complete);
    assert_eq!(bases.value, vec![old]);
    assert_eq!(snapshot.model().element(old).unwrap().metaclass(), c::CLASS);
    let cycle_a = lookup(root, &["CycleA"])[0];
    let cycle_b = lookup(root, &["CycleB"])[0];
    assert_eq!(
        lookup(cycle_a, &["BType"]),
        lookup(root, &["CycleB", "BType"])
    );
    assert_eq!(
        lookup(cycle_b, &["AType"]),
        lookup(root, &["CycleA", "AType"])
    );
    for reference in draft.references() {
        let answer = query.lookup_relationship_target(
            reference.relationship,
            reference.property,
            &reference.name,
        );
        assert_eq!(answer.completeness, Completeness::Complete, "{reference:?}");
        assert_eq!(answer.value.len(), 1, "{reference:?}");
    }
}

#[test]
fn authored_edit_reuses_unaffected_semantic_and_relationship_identities() {
    let first = parse("part def Engine; part def Vehicle { part engine : Engine; }");
    let before = lower(&first);
    let edit = agq_kerml_syntax::TextEdit {
        range: agq_kernel::provenance::ByteRange::new(
            first.source().len() as u64,
            first.source().len() as u64,
        )
        .unwrap(),
        replacement: " part def Other;".into(),
    };
    let next = first.edit(&edit, Default::default()).unwrap();
    let after = lower(&next);
    for name in ["Engine", "Vehicle", "engine"] {
        assert_eq!(
            named(before.candidate().model(), name),
            named(after.candidate().model(), name)
        );
    }
    let ids = |draft: &LibraryDraft| {
        draft
            .candidate()
            .model()
            .instances(c::FEATURE_TYPING, true)
            .unwrap()
            .map(|r| r.id())
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&before), ids(&after));
}

#[test]
fn textual_and_direct_changeset_graphs_have_equivalent_semantic_answers() {
    let text=lower(&parse("part def Engine; part def Vehicle { part engine : Engine; } part def SportsCar :> Vehicle;")).strict_snapshot().unwrap();
    let base = Snapshot::new(Arc::new(
        agq_sysml::registry_for_profile(BaselineProfile::OPERATIONAL_V9).unwrap(),
    ));
    let mut changes = base.change_set();
    let origin = DeclaredOrigin::Authored { source: None };
    let id = ElementId::from_u128;
    let classes = [
        (1, c::NAMESPACE),
        (2, s::PART_DEFINITION),
        (3, s::PART_DEFINITION),
        (4, s::PART_DEFINITION),
        (5, s::PART_USAGE),
        (11, c::OWNING_MEMBERSHIP),
        (12, c::OWNING_MEMBERSHIP),
        (13, c::OWNING_MEMBERSHIP),
        (14, c::FEATURE_MEMBERSHIP),
        (21, c::FEATURE_TYPING),
        (22, c::SUBCLASSIFICATION),
    ];
    for (number, class) in classes {
        changes.create(id(number), class, origin.clone());
        changes.set(
            id(number),
            p::ELEMENT_ELEMENT_ID,
            SlotValue::Scalar(Value::String(id(number).to_string())),
            origin.clone(),
        );
        // Explicit fixture defaults, checked against effective descriptor storage.
        for property in [
            p::ELEMENT_IS_IMPLIED_INCLUDED,
            p::RELATIONSHIP_IS_IMPLIED,
            p::TYPE_IS_ABSTRACT,
            p::TYPE_IS_SUFFICIENT,
            p::FEATURE_IS_COMPOSITE,
            p::FEATURE_IS_CONSTANT,
            p::FEATURE_IS_DERIVED,
            p::FEATURE_IS_END,
            p::FEATURE_IS_ORDERED,
            p::FEATURE_IS_PORTION,
            sp::DEFINITION_IS_VARIATION,
            sp::USAGE_IS_VARIATION,
            sp::OCCURRENCE_DEFINITION_IS_INDIVIDUAL,
            sp::OCCURRENCE_USAGE_IS_INDIVIDUAL,
        ] {
            if base
                .model()
                .registry()
                .resolve_property(class, property)
                .unwrap()
                .is_some_and(|p| !p.derived)
            {
                changes.set(
                    id(number),
                    property,
                    SlotValue::Scalar(Value::Boolean(false)),
                    origin.clone(),
                );
            }
        }
        if base
            .model()
            .registry()
            .is_subtype(class, c::MEMBERSHIP)
            .unwrap()
        {
            let agq_kernel::metamodel::ValueKind::Enumeration(domain) = base
                .model()
                .registry()
                .property(p::MEMBERSHIP_VISIBILITY)
                .unwrap()
                .value_kind
            else {
                unreachable!()
            };
            let literal = *base
                .model()
                .registry()
                .enumeration(domain)
                .unwrap()
                .literals
                .iter()
                .find(|(_, name)| name.as_str() == "public")
                .unwrap()
                .0;
            changes.set(
                id(number),
                p::MEMBERSHIP_VISIBILITY,
                SlotValue::Scalar(Value::Enumeration(literal)),
                origin.clone(),
            );
        }
    }
    for (number, name) in [
        (2, "Engine"),
        (3, "Vehicle"),
        (4, "SportsCar"),
        (5, "engine"),
    ] {
        changes.set(
            id(number),
            p::ELEMENT_DECLARED_NAME,
            SlotValue::Scalar(Value::String(name.into())),
            origin.clone(),
        );
    }
    changes.set(
        id(5),
        p::FEATURE_IS_UNIQUE,
        SlotValue::Scalar(Value::Boolean(true)),
        origin.clone(),
    );
    changes.set(
        id(5),
        p::FEATURE_IS_COMPOSITE,
        SlotValue::Scalar(Value::Boolean(true)),
        origin.clone(),
    );
    for (owner, property, targets) in [
        (1, p::ELEMENT_OWNED_RELATIONSHIP, vec![11, 12, 13]),
        (11, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, vec![2]),
        (12, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, vec![3]),
        (13, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, vec![4]),
        (3, p::ELEMENT_OWNED_RELATIONSHIP, vec![14]),
        (14, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, vec![5]),
        (5, p::ELEMENT_OWNED_RELATIONSHIP, vec![21]),
        (4, p::ELEMENT_OWNED_RELATIONSHIP, vec![22]),
    ] {
        changes.set(
            id(owner),
            property,
            SlotValue::Ordered(
                targets
                    .into_iter()
                    .map(|target| Value::Reference(id(target)))
                    .collect(),
            ),
            origin.clone(),
        );
    }
    changes.set(
        id(21),
        p::FEATURE_TYPING_TYPE,
        SlotValue::Scalar(Value::Reference(id(2))),
        origin.clone(),
    );
    changes.set(
        id(21),
        p::FEATURE_TYPING_TYPED_FEATURE,
        SlotValue::Scalar(Value::Reference(id(5))),
        origin.clone(),
    );
    changes.set(
        id(22),
        p::SUBCLASSIFICATION_SUBCLASSIFIER,
        SlotValue::Scalar(Value::Reference(id(4))),
        origin.clone(),
    );
    changes.set(
        id(22),
        p::SUBCLASSIFICATION_SUPERCLASSIFIER,
        SlotValue::Scalar(Value::Reference(id(3))),
        origin,
    );
    let direct = base.apply(&changes).unwrap();
    let summary = |snapshot: &Snapshot| {
        let model = snapshot.model();
        let queries = q(snapshot);
        let name = |id| {
            model
                .navigation_slot(id, p::ELEMENT_DECLARED_NAME)
                .unwrap()
                .value()
                .values()
                .find_map(|v| {
                    if let Value::String(s) = v {
                        Some(s.clone())
                    } else {
                        None
                    }
                })
                .unwrap()
        };
        let engine = named(model, "engine");
        let car = named(model, "SportsCar");
        (
            queries
                .direct_feature_types(engine)
                .value
                .into_iter()
                .map(name)
                .collect::<Vec<_>>(),
            queries
                .effective_features(car)
                .value
                .into_iter()
                .map(name)
                .collect::<Vec<_>>(),
            queries
                .supertypes(car)
                .value
                .into_iter()
                .map(name)
                .collect::<Vec<_>>(),
        )
    };
    assert_eq!(summary(&text), summary(&direct));
    assert_eq!(
        summary(&text),
        (
            vec!["Engine".into()],
            vec!["engine".into()],
            vec!["Vehicle".into()]
        )
    );
}

#[test]
fn behavior_and_requirement_members_preserve_roles_and_referential_parameters() {
    let syntax = parse(
        r#"
        attribute def Number;
        calc def Measure { in attribute input : Number; return attribute answer : Number; }
        state def Ready { entry action entering; do action working; exit action leaving; }
        requirement def Check { subject target; actor observer; require constraint predicate { true } }
        analysis def Analysis { subject analyzed; objective goal; }
    "#,
    );
    let draft = lower(&syntax);
    let snapshot = draft.strict_snapshot().unwrap();
    let model = snapshot.model();
    for (name, class) in [
        ("Measure", s::CALCULATION_DEFINITION),
        ("Ready", s::STATE_DEFINITION),
        ("Check", s::REQUIREMENT_DEFINITION),
        ("Analysis", s::ANALYSIS_CASE_DEFINITION),
        ("goal", s::REQUIREMENT_USAGE),
        ("observer", s::PART_USAGE),
    ] {
        assert_eq!(
            model.element(named(model, name)).unwrap().metaclass(),
            class,
            "{name}"
        );
    }
    for (class, expected) in [
        (s::STATE_SUBACTION_MEMBERSHIP, 3),
        (s::SUBJECT_MEMBERSHIP, 2),
        (s::ACTOR_MEMBERSHIP, 1),
        (s::OBJECTIVE_MEMBERSHIP, 1),
        (s::REQUIREMENT_CONSTRAINT_MEMBERSHIP, 1),
    ] {
        assert_eq!(model.instances(class, false).unwrap().count(), expected);
    }
    let states: BTreeSet<_> = model
        .instances(s::STATE_SUBACTION_MEMBERSHIP, false)
        .unwrap()
        .map(|membership| {
            model
                .navigation_slot(membership.id(), sp::STATE_SUBACTION_MEMBERSHIP_KIND)
                .unwrap()
                .value()
                .values()
                .find_map(|value| match value {
                    Value::Enumeration(id) => Some(*id),
                    _ => None,
                })
                .unwrap()
        })
        .collect();
    assert_eq!(states.len(), 3, "entry/do/exit remain distinct enum values");
    for name in ["input", "answer", "target", "observer", "analyzed"] {
        assert_eq!(
            model
                .navigation_slot(named(model, name), p::FEATURE_IS_COMPOSITE)
                .unwrap()
                .value(),
            &SlotValue::Scalar(Value::Boolean(false)),
            "directed parameter {name}"
        );
    }
    let returns: Vec<_> = model
        .instances(c::RETURN_PARAMETER_MEMBERSHIP, true)
        .unwrap()
        .filter(|membership| {
            q(&snapshot).owning_related_element(membership.id()).value
                == Some(named(model, "Measure"))
        })
        .map(|membership| membership.id())
        .collect();
    assert_eq!(
        returns.len(),
        1,
        "explicit result has one canonical membership"
    );
}

#[test]
fn operational_connection_interface_and_payload_preserve_canonical_families() {
    let syntax = production::parse_sysml_with_profile(
        production::SysmlSyntaxProfile::OperationalV1,
        DocumentId::new(),
        SourceRevisionId::new(),
        r#"
        part def Unit;
        port def Socket;
        connection def Cable { end source : Unit; end target : Unit; }
        interface def Pair { end port left : Socket; end port right : Socket; }
        part def Assembly {
            part firstUnit : Unit; part secondUnit : Unit;
            connection cable : Cable connect firstUnit to secondUnit;
            event occurrence happened;
            flow transfer of Unit;
        }
        "#,
        Default::default(),
    )
    .unwrap();
    assert!(syntax.is_complete(), "{:?}", syntax.diagnostics());
    let draft = lower(&syntax);
    let snapshot = draft.strict_snapshot().unwrap();
    let model = snapshot.model();
    for (name, class) in [
        ("Cable", s::CONNECTION_DEFINITION),
        ("Pair", s::INTERFACE_DEFINITION),
        ("left", s::PORT_USAGE),
        ("right", s::PORT_USAGE),
        ("cable", s::CONNECTION_USAGE),
        ("transfer", s::FLOW_USAGE),
    ] {
        assert_eq!(
            model.element(named(model, name)).unwrap().metaclass(),
            class,
            "{name}"
        );
    }
    for name in ["source", "target", "left", "right"] {
        assert_eq!(
            model
                .navigation_slot(named(model, name), p::FEATURE_IS_END)
                .unwrap()
                .value(),
            &SlotValue::Scalar(Value::Boolean(true)),
            "{name}"
        );
    }
    assert_eq!(
        model
            .navigation_slot(named(model, "happened"), p::FEATURE_IS_COMPOSITE)
            .unwrap()
            .value(),
        &SlotValue::Scalar(Value::Boolean(false))
    );
    let endpoints = q(&snapshot).connector_endpoints(named(model, "cable"));
    assert_eq!(endpoints.completeness, Completeness::Complete);
    assert_eq!(
        endpoints.value,
        vec![named(model, "firstUnit"), named(model, "secondUnit")]
    );
    assert_eq!(
        model.instances(c::PAYLOAD_FEATURE, false).unwrap().count(),
        1
    );
}

#[test]
fn end_cross_feature_modifiers_do_not_leak_to_the_end_usage() {
    let syntax = production::parse_sysml_with_profile(
        production::SysmlSyntaxProfile::OperationalV1,
        DocumentId::new(),
        SourceRevisionId::new(),
        "item def Container { end abstract crossing item endValue; }",
        Default::default(),
    )
    .unwrap();
    assert!(syntax.is_complete(), "{:?}", syntax.diagnostics());
    let draft = lower(&syntax);
    let snapshot = draft.strict_snapshot().unwrap();
    for (name, expected) in [("crossing", true), ("endValue", false)] {
        assert_eq!(
            snapshot
                .model()
                .navigation_slot(named(snapshot.model(), name), p::TYPE_IS_ABSTRACT)
                .unwrap()
                .value(),
            &SlotValue::Scalar(Value::Boolean(expected)),
            "{name}"
        );
    }
}
