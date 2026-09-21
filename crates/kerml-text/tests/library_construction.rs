use agq_kerml::{classes as c, properties as p};
use agq_kerml_text::library::lower_declarations;
use agq_kernel::provenance::{DeclaredOrigin, FactKey, Origin};
use agq_standard_libraries::VerifiedLibrarySet;
use std::{collections::BTreeSet, path::Path};

#[test]
fn exact_corpus_construction_has_repeatable_canonical_facts_and_separate_source_evidence() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root).unwrap();
    let first = lower_declarations(&sources).unwrap();
    let second = lower_declarations(&sources).unwrap();
    let model = first.candidate().model();
    let queries = first.queries(&sources).unwrap();
    assert_eq!(
        first.baseline_profile(),
        agq_kerml::BaselineProfile::OPERATIONAL
    );
    assert_eq!(
        queries.context().baseline_profile_id,
        first.baseline_profile().id()
    );
    assert_eq!(queries.context().pinned_libraries.len(), 3);
    let bindings = queries.context().standard_bindings.as_ref().unwrap();
    assert_eq!(
        bindings.iter().count(),
        agq_kerml_semantics::StandardRole::ALL.len()
    );
    for (role, target) in bindings.iter() {
        let (_, expected) = role.specification();
        let record = model.element(target).unwrap();
        assert_eq!(record.metaclass(), expected);
        assert_eq!(
            record.origin(),
            &Origin::Declared(DeclaredOrigin::StandardLibrary {
                library: bindings.bound(role).library
            })
        );
    }
    let other_queries = second.queries(&sources).unwrap();
    assert_eq!(
        queries.context().library_graph_digest,
        other_queries.context().library_graph_digest
    );
    assert_eq!(
        queries.context().standard_bindings,
        other_queries.context().standard_bindings
    );
    for fault in [
        Fault::Missing,
        Fault::Duplicate,
        Fault::WrongClass,
        Fault::WrongLibrary,
        Fault::Private,
    ] {
        use agq_kerml_semantics::{
            BindingError, KerMlQueries, SemanticContext, StandardKermlBindings, StandardRole,
        };
        let candidate =
            defective_candidate(model, &queries, bindings.get(StandardRole::Anything), fault);
        let q = KerMlQueries::new(
            SemanticContext::for_construction(
                &candidate,
                agq_kerml_semantics::SemanticOptions {
                    baseline_profile: first.baseline_profile(),
                    ..Default::default()
                },
                bindings.library_set().pins.clone(),
            )
            .unwrap(),
        );
        let error =
            StandardKermlBindings::validate(&q, first.roots(), bindings.library_set()).unwrap_err();
        assert!(matches!(
            (fault, error),
            (
                Fault::Missing,
                BindingError::Missing(StandardRole::Anything)
            ) | (
                Fault::Duplicate,
                BindingError::Ambiguous(StandardRole::Anything)
            ) | (
                Fault::WrongClass,
                BindingError::WrongMetaclass(StandardRole::Anything, _)
            ) | (
                Fault::WrongLibrary,
                BindingError::WrongLibrary(StandardRole::Anything, _)
            ) | (
                Fault::Private,
                BindingError::Inaccessible(StandardRole::Anything, _)
            )
        ));
    }
    for role in [
        agq_kerml_semantics::StandardRole::ThingsThat,
        agq_kerml_semantics::StandardRole::OccurrenceStartShot,
        agq_kerml_semantics::StandardRole::CollectionsArray,
    ] {
        use agq_kerml_semantics::{
            BindingError, KerMlQueries, SemanticContext, StandardKermlBindings,
        };
        let bound = bindings.bound(role);
        let artifact = sources
            .libraries()
            .values()
            .find(|l| l.resource() == role.library_artifact().resource())
            .unwrap();
        assert_eq!(bound.library, artifact.id());
        for fault in [Fault::Missing, Fault::WrongLibrary, Fault::Private] {
            let candidate = defective_candidate(model, &queries, bound.element, fault);
            let q = KerMlQueries::new(
                SemanticContext::for_construction(
                    &candidate,
                    agq_kerml_semantics::SemanticOptions {
                        baseline_profile: first.baseline_profile(),
                        ..Default::default()
                    },
                    bindings.library_set().pins.clone(),
                )
                .unwrap(),
            );
            let error = StandardKermlBindings::validate(&q, first.roots(), bindings.library_set())
                .unwrap_err();
            let failed_role = match error {
                BindingError::Missing(role)
                | BindingError::WrongLibrary(role, _)
                | BindingError::Inaccessible(role, _) => role,
                other => panic!("Unexpected binding failure: {other:?}"),
            };
            assert_eq!(failed_role, role);
        }
    }
    assert_eq!(first.roots().len(), 36);
    assert_eq!(
        model.elements().collect::<Vec<_>>(),
        second.candidate().model().elements().collect::<Vec<_>>()
    );
    assert_eq!(
        model.association_occurrences().collect::<Vec<_>>(),
        second
            .candidate()
            .model()
            .association_occurrences()
            .collect::<Vec<_>>()
    );
    assert_eq!(first.source_map(), second.source_map());
    assert_eq!(
        first.candidate().obligations(),
        second.candidate().obligations()
    );
    let endpoints: BTreeSet<_> = first
        .references()
        .iter()
        .map(|r| (r.relationship, r.property))
        .collect();
    assert_eq!(
        endpoints.len(),
        first.references().len(),
        "Each source assertion needs its own canonical endpoint"
    );
    assert_eq!(model.instances(c::IMPORT, true).unwrap().count(), 204);
    assert_eq!(
        model
            .instances(c::MEMBERSHIP_IMPORT, false)
            .unwrap()
            .count(),
        167
    );
    assert_eq!(
        model.instances(c::NAMESPACE_IMPORT, false).unwrap().count(),
        37
    );
    assert!(
        model.instances(c::EXPRESSION, true).unwrap().count() > 1000,
        "Expressions must survive lowering"
    );
    for record in model.elements() {
        let source = &first.source_map()[&FactKey::Element(record.id())];
        let document = sources
            .documents()
            .find(|d| d.document() == source.document)
            .unwrap();
        assert_eq!(source.revision, document.revision());
        assert!(source.syntax_node.is_some());
        assert!(
            document
                .source()
                .get(source.range.start() as usize..source.range.end() as usize)
                .is_some()
        );
        let expected = Origin::Declared(DeclaredOrigin::StandardLibrary {
            library: document.library(),
        });
        assert_eq!(record.origin(), &expected);
        for (property, slot) in record.slots() {
            assert_eq!(slot.origin(), &expected);
            assert!(first.source_map().contains_key(&FactKey::Property {
                element: record.id(),
                property
            }));
        }
    }
    // Missing targets are explicit obligations; construction does not claim a
    // published snapshot or invent references merely to meet multiplicities.
    assert!(
        first
            .candidate()
            .obligations()
            .iter()
            .any(|o| o.property == p::MEMBERSHIP_IMPORT_IMPORTED_MEMBERSHIP)
    );
    assert!(first.references().iter().any(|r| r.executable_expression));
    let mut declared_expressions = BTreeSet::new();
    for document in sources
        .documents()
        .filter(|d| d.language() == agq_standard_libraries::LibraryLanguage::KerMl)
    {
        let parsed = agq_kerml_syntax::production::parse(
            document.document(),
            document.revision(),
            document.source(),
            Default::default(),
        )
        .unwrap();
        use agq_kerml_syntax::production::Production as P;
        declared_expressions.extend(
            parsed
                .nodes()
                .filter(|n| {
                    matches!(
                        n.kind(),
                        P::Expression | P::BooleanExpression | P::Invariant
                    )
                })
                .map(|n| n.id()),
        );
    }
    for expression in model.instances(c::EXPRESSION, true).unwrap() {
        let results = queries
            .memberships(expression.id())
            .value
            .into_iter()
            .filter(|m| model.element(*m).unwrap().metaclass() == c::RETURN_PARAMETER_MEMBERSHIP)
            .count();
        if first.source_map()[&FactKey::Element(expression.id())]
            .syntax_node
            .is_some_and(|id| declared_expressions.contains(&id))
        {
            assert!(
                results <= 1,
                "Declared expressions may inherit their result; never copy it"
            );
        } else {
            assert_eq!(
                results, 1,
                "Structural expression forms, including typed casts, retain their local result"
            );
        }
    }
    for role in [
        agq_kerml_semantics::StandardRole::Evaluations,
        agq_kerml_semantics::StandardRole::TrueEvaluations,
        agq_kerml_semantics::StandardRole::FalseEvaluations,
    ] {
        assert!(
            !queries
                .memberships(bindings.get(role))
                .value
                .iter()
                .any(|m| model.element(*m).unwrap().metaclass() == c::RETURN_PARAMETER_MEMBERSHIP),
            "These declarations inherit their result; corroborated by the reference XMI including implied relationships"
        );
    }
}

#[derive(Clone, Copy)]
enum Fault {
    Missing,
    Duplicate,
    WrongClass,
    WrongLibrary,
    Private,
}

// Deliberately corrupted adversarial fixtures copied from the real candidate.
// They are never accepted or published as standard libraries.
fn defective_candidate(
    model: &agq_kernel::ModelView,
    queries: &agq_kerml_semantics::KerMlQueries<'_>,
    target: agq_kernel::ElementId,
    fault: Fault,
) -> agq_kernel::ConstructionView {
    use agq_kernel::{value::*, *};
    let membership = queries.owning_relationship(target).value.unwrap();
    let namespace = queries.owner(target).value.unwrap();
    let duplicate = ElementId::from_u128(u128::MAX);
    let duplicate_member = ElementId::from_u128(u128::MAX - 1);
    let empty = Snapshot::new(std::sync::Arc::new(model.registry().clone()));
    let mut changes = empty.change_set();
    for record in model.elements() {
        let Origin::Declared(original) = record.origin() else {
            panic!("canonical record origin")
        };
        let origin = if record.id() == target && matches!(fault, Fault::WrongLibrary) {
            DeclaredOrigin::Authored { source: None }
        } else {
            original.clone()
        };
        let class = if record.id() == target && matches!(fault, Fault::WrongClass) {
            c::TYPE
        } else {
            record.metaclass()
        };
        changes.create(record.id(), class, origin.clone());
        for (property, slot) in record.slots() {
            if record.id() == target
                && property == p::ELEMENT_DECLARED_NAME
                && matches!(fault, Fault::Missing)
            {
                continue;
            }
            let mut value = slot.value().clone();
            if record.id() == membership
                && property == p::MEMBERSHIP_VISIBILITY
                && matches!(fault, Fault::Private)
            {
                let agq_kernel::metamodel::ValueKind::Enumeration(domain) =
                    model.registry().property(property).unwrap().value_kind
                else {
                    unreachable!()
                };
                let literal = *model
                    .registry()
                    .enumeration(domain)
                    .unwrap()
                    .literals
                    .iter()
                    .find(|(_, name)| name.as_str() == "private")
                    .unwrap()
                    .0;
                value = SlotValue::Scalar(Value::Enumeration(literal));
            }
            if record.id() == namespace
                && property == p::ELEMENT_OWNED_RELATIONSHIP
                && matches!(fault, Fault::Duplicate)
                && let SlotValue::Ordered(values) = &mut value
            {
                values.push(Value::Reference(duplicate_member));
            }
            changes.set(record.id(), property, value, origin.clone());
        }
    }
    for link in model.association_occurrences() {
        changes.link(
            link.id(),
            link.association(),
            link.ends().clone(),
            link.positions().clone(),
            link.origin().clone(),
        );
    }
    if matches!(fault, Fault::Duplicate) {
        for (old, new) in [(target, duplicate), (membership, duplicate_member)] {
            let record = model.element(old).unwrap();
            let Origin::Declared(origin) = record.origin() else {
                unreachable!()
            };
            changes.create(new, record.metaclass(), origin.clone());
            for (property, slot) in record.slots() {
                if property == p::ELEMENT_OWNED_RELATIONSHIP {
                    continue;
                }
                let value = if property == p::ELEMENT_ELEMENT_ID {
                    SlotValue::Scalar(Value::String(new.to_string()))
                } else if property == p::RELATIONSHIP_OWNED_RELATED_ELEMENT {
                    SlotValue::Ordered(vec![Value::Reference(duplicate)])
                } else {
                    slot.value().clone()
                };
                changes.set(new, property, value, origin.clone());
            }
        }
    }
    empty.preview(&changes).unwrap()
}
