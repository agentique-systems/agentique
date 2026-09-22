use super::*;
use agq_kerml::{BaselineProfile, properties as p};
use agq_kerml_semantics::{SemanticContext, SemanticOptions};
use agq_kernel::{
    DocumentId, ModelView, SourceRevisionId,
    provenance::{FactKey, Origin},
    value::{SlotValue, Value},
};
use agq_sysml::{classes as s, properties as sp};

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
fn unsupported_sysml_semantics_is_rejected_without_fabricating_plain_features() {
    let syntax = parse("action def Work;");
    assert!(
        construction::check_supported_sysml(&syntax)
            .unwrap_err()
            .to_string()
            .contains("ActionDefinition")
    );
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
