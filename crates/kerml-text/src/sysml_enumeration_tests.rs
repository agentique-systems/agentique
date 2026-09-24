use super::*;

#[test]
fn enumeration_text_defaults_are_exact_and_source_backed() {
    let syntax = parse("enum def Color { enum red; enum green; } attribute def Plain;");
    let draft = lower(&syntax);
    let model = draft.candidate().model();
    let color = named(model, "Color");
    let plain = named(model, "Plain");
    for (subject, value) in [(color, true), (plain, false)] {
        assert_eq!(
            model
                .navigation_slot(subject, p::TYPE_IS_ABSTRACT)
                .unwrap()
                .value(),
            &SlotValue::Scalar(Value::Boolean(value))
        );
        let property = if subject == color {
            sp::ENUMERATION_DEFINITION_IS_VARIATION
        } else {
            sp::DEFINITION_IS_VARIATION
        };
        let slot = model.navigation_slot(subject, property).unwrap();
        assert_eq!(slot.value(), &SlotValue::Scalar(Value::Boolean(value)));
        assert!(matches!(
            slot.origin(),
            Origin::Declared(agq_kernel::provenance::DeclaredOrigin::Authored { source: Some(_) })
        ));
    }
    let queries = KerMlQueries::new(
        SemanticContext::for_construction(draft.candidate(), options(), BTreeSet::new()).unwrap(),
    );
    for name in ["red", "green"] {
        let literal = named(model, name);
        let membership = queries.owning_relationship(literal).value.unwrap();
        assert_eq!(
            model.element(membership).unwrap().metaclass(),
            s::VARIANT_MEMBERSHIP
        );
        assert_eq!(
            queries.owning_related_element(membership).value,
            Some(color)
        );
        assert_eq!(
            model.element(literal).unwrap().metaclass(),
            s::ENUMERATION_USAGE
        );
    }
}

#[test]
fn all_systems_enumerations_lower_as_variations_and_retain_literal_memberships() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root).unwrap();
    let mut definitions = BTreeSet::new();
    let mut literal_count = 0;
    for source in sources
        .documents()
        .filter(|source| source.path().starts_with("Systems Library/"))
    {
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
        let queries = KerMlQueries::new(
            SemanticContext::for_construction(draft.candidate(), options(), BTreeSet::new())
                .unwrap(),
        );
        for definition in model.instances(s::ENUMERATION_DEFINITION, true).unwrap() {
            let slot = model
                .navigation_slot(definition.id(), sp::ENUMERATION_DEFINITION_IS_VARIATION)
                .unwrap();
            assert_eq!(slot.value(), &SlotValue::Scalar(Value::Boolean(true)));
            assert!(matches!(
                slot.origin(),
                Origin::Declared(agq_kernel::provenance::DeclaredOrigin::StandardLibrary { .. })
            ));
            let SlotValue::Scalar(Value::String(name)) = model
                .navigation_slot(definition.id(), p::ELEMENT_DECLARED_NAME)
                .unwrap()
                .value()
                .clone()
            else {
                panic!("named enum")
            };
            definitions.insert(name);
        }
        for literal in model.instances(s::ENUMERATION_USAGE, true).unwrap() {
            let membership = queries.owning_relationship(literal.id()).value.unwrap();
            if model.element(membership).unwrap().metaclass() == s::VARIANT_MEMBERSHIP {
                let owner = queries.owning_related_element(membership).value.unwrap();
                assert_eq!(
                    model.element(owner).unwrap().metaclass(),
                    s::ENUMERATION_DEFINITION
                );
                literal_count += 1;
            }
        }
    }
    assert_eq!(literal_count, 21);
    assert_eq!(
        definitions,
        [
            "PortionKind",
            "RequirementConstraintKind",
            "StateSubactionKind",
            "TransitionFeatureKind",
            "TriggerKind",
            "VerdictKind",
            "VerificationMethodKind"
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    );
}
