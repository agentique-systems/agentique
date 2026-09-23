use super::*;
use agq_kernel::{MetaclassId, PropertyId, value::SlotValue};
use agq_sysml::classes as s;

fn references(model: &ModelView, owner: ElementId, property: PropertyId) -> Vec<ElementId> {
    model
        .navigation_slot(owner, property)
        .into_iter()
        .flat_map(|slot| slot.value().values())
        .filter_map(|value| {
            if let Value::Reference(id) = value {
                Some(*id)
            } else {
                None
            }
        })
        .collect()
}

fn relationship(model: &ModelView, owner: ElementId, class: MetaclassId) -> ElementId {
    let candidates: Vec<_> = references(model, owner, p::ELEMENT_OWNED_RELATIONSHIP)
        .into_iter()
        .filter(|id| model.element(*id).unwrap().metaclass() == class)
        .collect();
    assert_eq!(candidates.len(), 1, "{owner}/{class}: {candidates:?}");
    candidates[0]
}

fn member(model: &ModelView, relationship: ElementId) -> ElementId {
    let values = references(model, relationship, p::RELATIONSHIP_OWNED_RELATED_ELEMENT);
    assert_eq!(values.len(), 1);
    values[0]
}

fn named(model: &ModelView, class: MetaclassId, name: &str) -> ElementId {
    let matches: Vec<_> = model
        .elements()
        .filter(|record| {
            record.metaclass() == class
                && record.slot(p::ELEMENT_DECLARED_NAME).is_some_and(|slot| {
                    slot.value() == &SlotValue::Scalar(Value::String(name.into()))
                })
        })
        .map(|record| record.id())
        .collect();
    assert_eq!(matches.len(), 1, "{name}: {matches:?}");
    matches[0]
}

fn boolean(model: &ModelView, element: ElementId, property: PropertyId, value: bool) {
    assert_eq!(
        model.navigation_slot(element, property).unwrap().value(),
        &SlotValue::Scalar(Value::Boolean(value))
    );
}

fn direction(model: &ModelView, element: ElementId, name: &str) {
    let descriptor = model.registry().property(p::FEATURE_DIRECTION).unwrap();
    let agq_kernel::metamodel::ValueKind::Enumeration(domain) = model
        .registry()
        .storage_kind(descriptor.value_kind)
        .unwrap()
    else {
        panic!("direction enumeration")
    };
    let literal = *model
        .registry()
        .enumeration(domain)
        .unwrap()
        .literals
        .iter()
        .find(|(_, value)| value.as_str() == name)
        .unwrap()
        .0;
    assert_eq!(
        model
            .navigation_slot(element, p::FEATURE_DIRECTION)
            .unwrap()
            .value(),
        &SlotValue::Scalar(Value::Enumeration(literal))
    );
}

fn result(model: &ModelView, expression: ElementId, class: MetaclassId, implied: bool) {
    let membership = relationship(model, expression, c::RETURN_PARAMETER_MEMBERSHIP);
    boolean(model, membership, p::RELATIONSHIP_IS_IMPLIED, implied);
    let result = member(model, membership);
    assert_eq!(model.element(result).unwrap().metaclass(), class);
    direction(model, result, "out");
}

#[test]
fn verification_case_objective_and_requirement_value_shape() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root).unwrap();
    for path in [
        "Systems Library/Cases.sysml",
        "Systems Library/VerificationCases.sysml",
    ] {
        let source = sources
            .documents()
            .find(|source| source.path() == path)
            .unwrap();
        let syntax = production::parse_sysml_with_profile(
            production::SysmlSyntaxProfile::OperationalV2,
            source.document(),
            source.revision(),
            source.source(),
            Default::default(),
        )
        .unwrap();
        assert!(syntax.is_complete());
        let draft = construction::construct_on(
            &[SourceInput {
                syntax: &syntax,
                library: Some(source),
                sysml: true,
            }],
            &Default::default(),
            agq_kerml::BaselineProfile::OPERATIONAL_V9,
            Snapshot::new(Arc::new(
                agq_sysml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V9)
                    .unwrap(),
            )),
            None,
        )
        .unwrap();
        let model = draft.candidate().model();
        let default = path.ends_with("/Cases.sysml");
        let case = if default {
            named(model, s::CASE_DEFINITION, "Case")
        } else {
            named(model, s::VERIFICATION_CASE_DEFINITION, "VerificationCase")
        };
        let objective = member(model, relationship(model, case, s::OBJECTIVE_MEMBERSHIP));
        assert_eq!(
            model.element(objective).unwrap().metaclass(),
            s::REQUIREMENT_USAGE
        );
        boolean(model, objective, p::FEATURE_IS_COMPOSITE, true);
        let subject = member(model, relationship(model, objective, s::SUBJECT_MEMBERSHIP));
        assert_eq!(
            model.element(subject).unwrap().metaclass(),
            s::REFERENCE_USAGE
        );
        direction(model, subject, "in");
        let value = relationship(model, subject, c::FEATURE_VALUE);
        boolean(model, value, p::FEATURE_VALUE_IS_DEFAULT, default);
        boolean(model, value, p::FEATURE_VALUE_IS_INITIAL, false);
        let expression = member(model, value);
        assert_eq!(
            model.element(expression).unwrap().metaclass(),
            c::FEATURE_REFERENCE_EXPRESSION
        );
        relationship(model, expression, c::MEMBERSHIP);
        result(model, expression, s::REFERENCE_USAGE, false);
        if !default {
            let inner = member(model, relationship(model, objective, c::FEATURE_MEMBERSHIP));
            assert_eq!(
                model.element(inner).unwrap().metaclass(),
                s::REQUIREMENT_USAGE
            );
            boolean(model, inner, p::FEATURE_IS_COMPOSITE, true);
            let outer = references(model, case, p::ELEMENT_OWNED_RELATIONSHIP)
                .into_iter()
                .filter(|id| model.element(*id).unwrap().metaclass() == c::FEATURE_MEMBERSHIP)
                .map(|id| member(model, id))
                .find(|id| model.element(*id).unwrap().metaclass() == s::REQUIREMENT_USAGE)
                .unwrap();
            boolean(model, outer, p::FEATURE_IS_COMPOSITE, false);
            let value = relationship(model, outer, c::FEATURE_VALUE);
            boolean(model, value, p::FEATURE_VALUE_IS_DEFAULT, false);
            let chain = member(model, value);
            assert_eq!(
                model.element(chain).unwrap().metaclass(),
                c::FEATURE_CHAIN_EXPRESSION
            );
            assert_eq!(
                model
                    .navigation_slot(chain, p::FEATURE_CHAIN_EXPRESSION_OPERATOR)
                    .unwrap()
                    .value(),
                &SlotValue::Scalar(Value::String(".".into()))
            );
            result(model, chain, c::FEATURE, true);
            relationship(model, chain, c::MEMBERSHIP);
            let input = member(model, relationship(model, chain, c::PARAMETER_MEMBERSHIP));
            assert_eq!(model.element(input).unwrap().metaclass(), c::FEATURE);
            direction(model, input, "in");
            let referent = member(model, relationship(model, input, c::FEATURE_VALUE));
            assert_eq!(
                model.element(referent).unwrap().metaclass(),
                c::FEATURE_REFERENCE_EXPRESSION
            );
            result(model, referent, s::REFERENCE_USAGE, false);
        }
    }
}
