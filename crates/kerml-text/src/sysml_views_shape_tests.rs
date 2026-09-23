use super::*;

#[test]
fn views_satisfaction_keeps_subject_value_and_required_constraint_shape() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root).unwrap();
    let source = sources
        .documents()
        .find(|source| source.path() == "Systems Library/Views.sysml")
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
    let satisfies: Vec<_> = model
        .elements()
        .filter(|record| record.metaclass() == agq_sysml::classes::SATISFY_REQUIREMENT_USAGE)
        .map(|record| record.id())
        .collect();
    assert_eq!(satisfies.len(), 1);
    fn references(
        model: &ModelView,
        element: ElementId,
        property: agq_kernel::PropertyId,
    ) -> Vec<ElementId> {
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
    fn relation(model: &ModelView, owner: ElementId, class: agq_kernel::MetaclassId) -> ElementId {
        let candidates: Vec<_> = references(model, owner, p::ELEMENT_OWNED_RELATIONSHIP)
            .into_iter()
            .filter(|&id| model.element(id).unwrap().metaclass() == class)
            .collect();
        assert_eq!(candidates.len(), 1);
        candidates[0]
    }
    fn member(model: &ModelView, relationship: ElementId) -> ElementId {
        let candidates = references(model, relationship, p::RELATIONSHIP_OWNED_RELATED_ELEMENT);
        assert_eq!(candidates.len(), 1);
        candidates[0]
    }
    fn flag(model: &ModelView, element: ElementId, property: agq_kernel::PropertyId, value: bool) {
        assert_eq!(
            model.navigation_slot(element, property).unwrap().value(),
            &agq_kernel::value::SlotValue::Scalar(Value::Boolean(value))
        );
    }
    fn enumeration(
        model: &ModelView,
        element: ElementId,
        property: agq_kernel::PropertyId,
        name: &str,
    ) {
        let descriptor = model.registry().property(property).unwrap();
        let agq_kernel::metamodel::ValueKind::Enumeration(domain) = model
            .registry()
            .storage_kind(descriptor.value_kind)
            .unwrap()
        else {
            panic!("enumeration")
        };
        let literal = model
            .registry()
            .enumeration(domain)
            .unwrap()
            .literals
            .iter()
            .find(|(_, value)| value.as_str() == name)
            .unwrap()
            .0;
        assert_eq!(
            model.navigation_slot(element, property).unwrap().value(),
            &agq_kernel::value::SlotValue::Scalar(Value::Enumeration(*literal))
        );
    }
    let satisfy = satisfies[0];
    assert_eq!(
        satisfy,
        ElementId::from_u128(0x5c001eb4d4e05bd4866bb756155e1bea)
    );
    flag(model, satisfy, p::FEATURE_IS_COMPOSITE, true);
    flag(model, satisfy, p::INVARIANT_IS_NEGATED, false);
    let subject = member(
        model,
        relation(model, satisfy, agq_sysml::classes::SUBJECT_MEMBERSHIP),
    );
    assert_eq!(
        model.element(subject).unwrap().metaclass(),
        agq_sysml::classes::REFERENCE_USAGE
    );
    enumeration(model, subject, p::FEATURE_DIRECTION, "in");
    flag(model, subject, p::FEATURE_IS_COMPOSITE, false);
    let value = relation(model, subject, c::FEATURE_VALUE);
    flag(model, value, p::FEATURE_VALUE_IS_DEFAULT, false);
    flag(model, value, p::FEATURE_VALUE_IS_INITIAL, false);
    let expression = member(model, value);
    assert_eq!(
        model.element(expression).unwrap().metaclass(),
        c::FEATURE_REFERENCE_EXPRESSION
    );
    relation(model, expression, c::MEMBERSHIP);
    let result_membership = relation(model, expression, c::RETURN_PARAMETER_MEMBERSHIP);
    let result = member(model, result_membership);
    assert_eq!(model.element(result).unwrap().metaclass(), c::FEATURE);
    enumeration(model, result, p::FEATURE_DIRECTION, "out");
    flag(model, result_membership, p::RELATIONSHIP_IS_IMPLIED, true);
    flag(model, expression, p::ELEMENT_IS_IMPLIED_INCLUDED, true);
    let required_membership = relation(
        model,
        satisfy,
        agq_sysml::classes::REQUIREMENT_CONSTRAINT_MEMBERSHIP,
    );
    enumeration(
        model,
        required_membership,
        agq_sysml::properties::REQUIREMENT_CONSTRAINT_MEMBERSHIP_KIND,
        "requirement",
    );
    let required = member(model, required_membership);
    assert_eq!(
        model.element(required).unwrap().metaclass(),
        agq_sysml::classes::CONSTRAINT_USAGE
    );
    flag(model, required, p::FEATURE_IS_COMPOSITE, true);
    relation(model, required, c::REFERENCE_SUBSETTING);
}
