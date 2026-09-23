use super::*;

#[test]
fn actions_index_assignment_preserves_frontend_ownership_shape() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root).unwrap();
    let source = sources
        .documents()
        .find(|source| source.path() == "Systems Library/Actions.sysml")
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
    let references = |element, property| {
        model
            .navigation_slot(element, property)
            .into_iter()
            .flat_map(|slot| slot.value().values())
            .filter_map(|value| match value {
                Value::Reference(id) => Some(*id),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let child = |owner, membership_class| {
        let memberships: Vec<_> = references(owner, p::ELEMENT_OWNED_RELATIONSHIP)
            .into_iter()
            .filter(|&id| model.element(id).unwrap().metaclass() == membership_class)
            .collect();
        assert_eq!(memberships.len(), 1);
        let members = references(memberships[0], p::RELATIONSHIP_OWNED_RELATED_ELEMENT);
        assert_eq!(members.len(), 1);
        (memberships[0], members[0])
    };
    let input = ElementId::from_u128(0x98c30374e65a579bbe500d23d18160d8);
    let value = ElementId::from_u128(0xd32765d8ddf25f1f9662a200aefd1b2e);
    assert_eq!(
        model.element(input).unwrap().metaclass(),
        agq_sysml::classes::REFERENCE_USAGE
    );
    assert_eq!(model.element(value).unwrap().metaclass(), c::FEATURE_VALUE);
    let index = ElementId::from_u128(0xf81c5d7e43d4534d907935fbf5b9d2f5);
    assert_eq!(child(input, c::FEATURE_VALUE), (value, index));
    assert_eq!(
        model.element(index).unwrap().metaclass(),
        c::INDEX_EXPRESSION
    );
    let (result_membership, result) = child(index, c::RETURN_PARAMETER_MEMBERSHIP);
    assert_eq!(
        result,
        ElementId::from_u128(0x7be8d3174a3a5f978fceebb47accaa91)
    );
    assert_eq!(model.element(result).unwrap().metaclass(), c::FEATURE);
    assert!(matches!(
        model.element(result).unwrap().origin(),
        agq_kernel::provenance::Origin::Declared(_)
    ));
    assert_eq!(
        model
            .navigation_slot(result_membership, p::RELATIONSHIP_IS_IMPLIED)
            .unwrap()
            .value(),
        &agq_kernel::value::SlotValue::Scalar(Value::Boolean(true))
    );
    assert_eq!(
        model
            .navigation_slot(index, p::ELEMENT_IS_IMPLIED_INCLUDED)
            .unwrap()
            .value(),
        &agq_kernel::value::SlotValue::Scalar(Value::Boolean(true))
    );
    let (_, primary) = child(index, c::PARAMETER_MEMBERSHIP);
    assert_eq!(model.element(primary).unwrap().metaclass(), c::FEATURE);
    let (_, sequence_reference) = child(primary, c::FEATURE_VALUE);
    assert_eq!(
        sequence_reference,
        ElementId::from_u128(0x2301cf942c23540988139f21f48e08b4)
    );
    let (_, index_reference) = child(index, c::FEATURE_MEMBERSHIP);
    assert_eq!(
        index_reference,
        ElementId::from_u128(0xa9ffe00f90b750aa9b96755d06dcc2da)
    );
    for expression in [sequence_reference, index_reference] {
        assert_eq!(
            model.element(expression).unwrap().metaclass(),
            c::FEATURE_REFERENCE_EXPRESSION
        );
        let (membership, result) = child(expression, c::RETURN_PARAMETER_MEMBERSHIP);
        assert_eq!(
            model.element(result).unwrap().metaclass(),
            agq_sysml::classes::REFERENCE_USAGE
        );
        assert_eq!(
            model
                .navigation_slot(membership, p::RELATIONSHIP_IS_IMPLIED)
                .unwrap()
                .value(),
            &agq_kernel::value::SlotValue::Scalar(Value::Boolean(false))
        );
        let origin = draft.source_map().get(&FactKey::Element(result)).unwrap();
        assert_eq!(
            origin.range.start(),
            origin.range.end(),
            "EmptyFeature syntax result"
        );
    }
}
