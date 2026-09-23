use super::*;

#[test]
fn items_touched_ends_keep_named_cross_features_multiplicity_and_redefinitions() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root).unwrap();
    let source = sources
        .documents()
        .find(|s| s.path() == "Systems Library/Items.sysml")
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
    fn owned(
        model: &ModelView,
        owner: ElementId,
        property: agq_kernel::PropertyId,
    ) -> Vec<ElementId> {
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
    for (end, cross_name) in [
        (
            ElementId::from_u128(0x7661b183d02f5ba1ae738f0f8b60b3c3),
            "touchesToo",
        ),
        (
            ElementId::from_u128(0xfc5a3876bdc7555da38d0720ca04e233),
            "touches",
        ),
    ] {
        assert_eq!(
            model.element(end).unwrap().metaclass(),
            agq_sysml::classes::ITEM_USAGE
        );
        let relationships = owned(model, end, p::ELEMENT_OWNED_RELATIONSHIP);
        assert_eq!(
            relationships
                .iter()
                .filter(|&&r| model.element(r).unwrap().metaclass() == c::REDEFINITION)
                .count(),
            2
        );
        let members: Vec<_> = relationships
            .iter()
            .filter(|&&r| model.element(r).unwrap().metaclass() == c::OWNING_MEMBERSHIP)
            .flat_map(|&m| owned(model, m, p::RELATIONSHIP_OWNED_RELATED_ELEMENT))
            .collect();
        let cross = members
            .iter()
            .copied()
            .find(|&m| model.element(m).unwrap().metaclass() == agq_sysml::classes::REFERENCE_USAGE)
            .unwrap();
        assert_eq!(
            model
                .navigation_slot(cross, p::ELEMENT_DECLARED_NAME)
                .unwrap()
                .value(),
            &agq_kernel::value::SlotValue::Scalar(Value::String(cross_name.into()))
        );
        assert_eq!(
            model
                .navigation_slot(end, p::FEATURE_IS_END)
                .unwrap()
                .value(),
            &agq_kernel::value::SlotValue::Scalar(Value::Boolean(true))
        );
        assert!(
            members
                .iter()
                .all(|&member| model.element(member).unwrap().metaclass() != c::MULTIPLICITY_RANGE)
        );
        let cross_members: Vec<_> = owned(model, cross, p::ELEMENT_OWNED_RELATIONSHIP)
            .into_iter()
            .filter(|&r| model.element(r).unwrap().metaclass() == c::OWNING_MEMBERSHIP)
            .flat_map(|m| owned(model, m, p::RELATIONSHIP_OWNED_RELATED_ELEMENT))
            .collect();
        assert_eq!(cross_members.len(), 1);
        let range = cross_members[0];
        assert_eq!(
            model.element(range).unwrap().metaclass(),
            c::MULTIPLICITY_RANGE
        );
        assert_eq!(
            owned(model, range, p::ELEMENT_OWNED_RELATIONSHIP).len(),
            2,
            "lower and upper bound expressions belong to cross multiplicity"
        );
    }
}
