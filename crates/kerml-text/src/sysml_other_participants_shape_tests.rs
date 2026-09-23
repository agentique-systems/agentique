use super::*;
use agq_kernel::{MetaclassId, PropertyId, value::SlotValue};

#[test]
fn interfaces_other_participants_preserves_caller_shape() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root).unwrap();
    let source = sources
        .documents()
        .find(|source| source.path() == "Systems Library/Interfaces.sysml")
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
            agq_sysml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V9).unwrap(),
        )),
        None,
    )
    .unwrap();
    let model = draft.candidate().model();
    let id = ElementId::from_u128;
    fn references(model: &ModelView, element: ElementId, property: PropertyId) -> Vec<ElementId> {
        model
            .navigation_slot(element, property)
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
    let relationships = |element| references(model, element, p::ELEMENT_OWNED_RELATIONSHIP);
    let members = |element| references(model, element, p::RELATIONSHIP_OWNED_RELATED_ELEMENT);
    let child = |owner, element, class: MetaclassId| {
        let candidates: Vec<_> = relationships(owner)
            .into_iter()
            .filter(|relationship| {
                model.element(*relationship).unwrap().metaclass() == class
                    && members(*relationship) == [element]
            })
            .collect();
        assert_eq!(candidates.len(), 1, "{owner}/{element}");
        candidates[0]
    };
    let direct_owner = |element| {
        let membership = model
            .elements()
            .find(|record| members(record.id()) == [element])
            .unwrap()
            .id();
        model
            .elements()
            .find(|record| relationships(record.id()).contains(&membership))
            .unwrap()
            .id()
    };
    let flag = |element, property, value| {
        assert_eq!(
            model.navigation_slot(element, property).unwrap().value(),
            &SlotValue::Scalar(Value::Boolean(value))
        )
    };
    let direction = |element, name: &str| {
        let descriptor = model.registry().property(p::FEATURE_DIRECTION).unwrap();
        let agq_kernel::metamodel::ValueKind::Enumeration(domain) = model
            .registry()
            .storage_kind(descriptor.value_kind)
            .unwrap()
        else {
            panic!("direction")
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
    };
    let other = id(0x36893206121a59d7bcc03764144f3d76);
    let this = id(0x669263b6d3c85933b1988d3762ba63a9);
    let participant = direct_owner(other);
    let interface = direct_owner(participant);
    assert_eq!(direct_owner(this), participant);
    for (element, class) in [
        (other, agq_sysml::classes::REFERENCE_USAGE),
        (this, agq_sysml::classes::REFERENCE_USAGE),
        (participant, agq_sysml::classes::PORT_USAGE),
        (interface, agq_sysml::classes::INTERFACE_DEFINITION),
    ] {
        assert_eq!(model.element(element).unwrap().metaclass(), class);
    }
    for element in [other, this, participant] {
        flag(element, p::FEATURE_IS_END, false);
        flag(element, p::FEATURE_IS_COMPOSITE, false);
    }
    flag(participant, p::FEATURE_IS_ORDERED, true);
    flag(participant, p::FEATURE_IS_UNIQUE, false);
    flag(other, p::FEATURE_IS_UNIQUE, false);
    for (element, expected) in [(participant, "[2..*]"), (other, "[1..*]")] {
        let multiplicities: Vec<_> = relationships(element)
            .into_iter()
            .flat_map(members)
            .filter(|element| model.element(*element).unwrap().metaclass() == c::MULTIPLICITY_RANGE)
            .collect();
        assert_eq!(multiplicities.len(), 1);
        assert_eq!(
            syntax.text(
                draft
                    .source_map()
                    .get(&FactKey::Element(multiplicities[0]))
                    .unwrap()
                    .range
            ),
            Some(expected)
        );
    }
    let invocation = id(0x3885a3f86f075c70b38f0b2c048add9f);
    assert_eq!(
        model.element(invocation).unwrap().metaclass(),
        c::INVOCATION_EXPRESSION
    );
    let value = child(other, invocation, c::FEATURE_VALUE);
    flag(value, p::FEATURE_VALUE_IS_DEFAULT, true);
    flag(value, p::FEATURE_VALUE_IS_INITIAL, false);
    for (input, expression, result, name) in [
        (
            id(0xcd0230e79bad56cf89e0f847e2e4078a),
            id(0x0890be8435e85a6494e9caa064d215f0),
            id(0x2b5ba4c00b755aad81196e495a6dcb86),
            "participant",
        ),
        (
            id(0xeec8b69ade0b5ef9aac933c1dba5015c),
            id(0x54c808a7f6c651418cf889adc5727b2f),
            id(0xd3c8e12952945b5992076e392ea0618a),
            "thisParticipant",
        ),
    ] {
        child(invocation, input, c::PARAMETER_MEMBERSHIP);
        assert_eq!(model.element(input).unwrap().metaclass(), c::FEATURE);
        direction(input, "in");
        child(input, expression, c::FEATURE_VALUE);
        assert_eq!(
            model.element(expression).unwrap().metaclass(),
            c::FEATURE_REFERENCE_EXPRESSION
        );
        let result_membership = child(expression, result, c::RETURN_PARAMETER_MEMBERSHIP);
        flag(result_membership, p::RELATIONSHIP_IS_IMPLIED, false);
        assert_eq!(
            model.element(result).unwrap().metaclass(),
            agq_sysml::classes::REFERENCE_USAGE
        );
        direction(result, "out");
        let reference = relationships(expression)
            .into_iter()
            .find(|membership| model.element(*membership).unwrap().metaclass() == c::MEMBERSHIP)
            .unwrap();
        assert_eq!(
            draft
                .references()
                .iter()
                .find(|assertion| assertion.relationship == reference)
                .unwrap()
                .name
                .segments,
            [name]
        );
    }
    let result = id(0x420b87f60adf572187acd0a91c0ffbab);
    let membership = child(invocation, result, c::RETURN_PARAMETER_MEMBERSHIP);
    flag(membership, p::RELATIONSHIP_IS_IMPLIED, false);
    assert_eq!(
        model.element(result).unwrap().metaclass(),
        agq_sysml::classes::REFERENCE_USAGE
    );
    direction(result, "out");
    let callee = relationships(invocation)
        .into_iter()
        .find(|relationship| model.element(*relationship).unwrap().metaclass() == c::MEMBERSHIP)
        .unwrap();
    assert_eq!(
        draft
            .references()
            .iter()
            .find(|assertion| assertion.relationship == callee)
            .unwrap()
            .name
            .segments,
        ["excludingOnce"]
    );
}
