use super::*;

#[test]
fn flows_anonymous_connection_end_preserves_frontend_shape() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root).unwrap();
    let source = sources
        .documents()
        .find(|source| source.path() == "Systems Library/Flows.sysml")
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
    let refs = |element, property| {
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
            .collect::<Vec<_>>()
    };
    let single = |element, property| {
        let values = refs(element, property);
        assert_eq!(values.len(), 1, "{element}/{property:?}: {values:?}");
        values[0]
    };
    let owned_member = |element, member_class| {
        let relationship = single(element, p::ELEMENT_OWNED_RELATIONSHIP);
        assert_eq!(
            model.element(relationship).unwrap().metaclass(),
            c::OWNING_MEMBERSHIP
        );
        let member = single(relationship, p::RELATIONSHIP_OWNED_RELATED_ELEMENT);
        assert_eq!(model.element(member).unwrap().metaclass(), member_class);
        member
    };
    for (id, text, referent) in [
        (0x63d0949c20e25ffa81a262b9826530dc, "[1] target", "target"),
        (0x86ddb7cdac2253229aa7ff4755f00198, "[1] source", "source"),
    ] {
        let id = ElementId::from_u128(id);
        assert_eq!(
            model.element(id).unwrap().metaclass(),
            agq_sysml::classes::REFERENCE_USAGE
        );
        let origin = draft.source_map().get(&FactKey::Element(id)).unwrap();
        assert_eq!(syntax.text(origin.range).unwrap(), text);
        assert!(
            model
                .navigation_slot(id, p::ELEMENT_DECLARED_NAME)
                .is_none()
        );
        for (property, value) in [(p::FEATURE_IS_END, true), (p::FEATURE_IS_COMPOSITE, false)] {
            assert_eq!(
                model.navigation_slot(id, property).unwrap().value(),
                &agq_kernel::value::SlotValue::Scalar(Value::Boolean(value))
            );
        }
        let membership = single(id, p::ELEMENT_OWNING_RELATIONSHIP);
        assert_eq!(
            model.element(membership).unwrap().metaclass(),
            c::END_FEATURE_MEMBERSHIP
        );
        let connection = single(membership, p::RELATIONSHIP_OWNING_RELATED_ELEMENT);
        assert_eq!(
            model.element(connection).unwrap().metaclass(),
            agq_sysml::classes::CONNECTION_USAGE
        );
        let owned = refs(id, p::ELEMENT_OWNED_RELATIONSHIP);
        assert_eq!(owned.len(), 2);
        assert_eq!(
            model.element(owned[0]).unwrap().metaclass(),
            c::OWNING_MEMBERSHIP
        );
        assert_eq!(
            model.element(owned[1]).unwrap().metaclass(),
            c::REFERENCE_SUBSETTING
        );
        let reference = draft
            .references()
            .iter()
            .find(|reference| reference.relationship == owned[1])
            .unwrap();
        assert_eq!(reference.name.segments, [referent]);
        let cross = single(owned[0], p::RELATIONSHIP_OWNED_RELATED_ELEMENT);
        assert_eq!(model.element(cross).unwrap().metaclass(), c::FEATURE);
        assert_eq!(
            syntax
                .text(
                    draft
                        .source_map()
                        .get(&FactKey::Element(cross))
                        .unwrap()
                        .range
                )
                .unwrap(),
            "[1]"
        );
        let multiplicity = owned_member(cross, c::MULTIPLICITY_RANGE);
        let bound = owned_member(multiplicity, c::LITERAL_INTEGER);
        assert_eq!(
            model
                .navigation_slot(bound, p::LITERAL_INTEGER_VALUE)
                .unwrap()
                .value(),
            &agq_kernel::value::SlotValue::Scalar(Value::Integer(1.into()))
        );
        let result_membership = single(bound, p::ELEMENT_OWNED_RELATIONSHIP);
        assert_eq!(
            model.element(result_membership).unwrap().metaclass(),
            c::RETURN_PARAMETER_MEMBERSHIP
        );
        assert_eq!(
            model
                .navigation_slot(result_membership, p::RELATIONSHIP_IS_IMPLIED)
                .unwrap()
                .value(),
            &agq_kernel::value::SlotValue::Scalar(Value::Boolean(true))
        );
        let result = single(result_membership, p::RELATIONSHIP_OWNED_RELATED_ELEMENT);
        assert_eq!(model.element(result).unwrap().metaclass(), c::FEATURE);
    }
}
