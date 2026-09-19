include!("common/namespace_fixture.rs");

fn errors(q: &KerMlQueries<'_>, element: u128) -> BTreeSet<&'static str> {
    q.validate_local_structure(id(element))
        .diagnostics
        .into_iter()
        .map(|d| d.code)
        .collect()
}

#[test]
fn subsetting_reports_independent_uniqueness_and_constant_violations() {
    for source_unique in [false, true] {
        for source_constant in [false, true] {
            let mut f = Fixture::new();
            for feature in [1, 2] {
                f.create(feature, c::FEATURE);
            }
            f.value(1, p::FEATURE_IS_VARIABLE, Value::Boolean(true));
            f.value(1, p::FEATURE_IS_UNIQUE, Value::Boolean(source_unique));
            f.value(1, p::FEATURE_IS_CONSTANT, Value::Boolean(source_constant));
            f.value(2, p::FEATURE_IS_UNIQUE, Value::Boolean(true));
            f.value(2, p::FEATURE_IS_CONSTANT, Value::Boolean(true));
            f.create(3, c::SUBSETTING);
            f.value(3, p::SUBSETTING_SUBSETTING_FEATURE, Value::Reference(id(1)));
            f.value(3, p::SUBSETTING_SUBSETTED_FEATURE, Value::Reference(id(2)));
            let snapshot = f.finish();
            let q = KerMlQueries::new(
                SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default())
                    .unwrap(),
            );
            let codes = errors(&q, 3);
            assert_eq!(
                codes.contains("validateSubsettingUniquenessConformance"),
                !source_unique
            );
            assert_eq!(
                codes.contains("validateSubsettingConstantConformance"),
                !source_constant
            );
        }
    }
}

#[test]
fn classifier_specialization_checks_semantic_kinds() {
    for (specific, general, rule, invalid) in [
        (c::CLASS, c::DATA_TYPE, "validateClassSpecialization", true),
        (
            c::CLASS,
            c::ASSOCIATION,
            "validateClassSpecialization",
            true,
        ),
        (
            c::ASSOCIATION_STRUCTURE,
            c::ASSOCIATION,
            "validateClassSpecialization",
            false,
        ),
        (
            c::DATA_TYPE,
            c::CLASS,
            "validateDataTypeSpecialization",
            true,
        ),
        (
            c::BEHAVIOR,
            c::STRUCTURE,
            "validateBehaviorSpecialization",
            true,
        ),
        (
            c::STRUCTURE,
            c::BEHAVIOR,
            "validateStructureSpecialization",
            true,
        ),
    ] {
        let mut f = Fixture::new();
        f.create(1, specific);
        f.create(2, general);
        f.create(3, c::SPECIALIZATION);
        f.value(3, p::SPECIALIZATION_SPECIFIC, Value::Reference(id(1)));
        f.value(3, p::SPECIALIZATION_GENERAL, Value::Reference(id(2)));
        f.own(1, 3);
        let model = f.construction();
        let q = KerMlQueries::new(
            SemanticContext::for_construction(&model, Default::default(), Default::default())
                .unwrap(),
        );
        assert_eq!(errors(&q, 1).contains(rule), invalid);
    }
}

#[test]
fn constructor_result_parameter_ownership_exception_is_preserved() {
    for constructor in [true, false] {
        let mut f = Fixture::new();
        f.create(
            1,
            if constructor {
                c::CONSTRUCTOR_EXPRESSION
            } else {
                c::EXPRESSION
            },
        );
        f.create(2, c::RETURN_PARAMETER_MEMBERSHIP);
        f.create(3, c::FEATURE);
        f.changes.set(
            id(2),
            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(id(3))]),
            origin(),
        );
        f.own(1, 2);
        f.create(4, c::PARAMETER_MEMBERSHIP);
        f.create(5, c::FEATURE);
        f.enumeration(5, p::FEATURE_DIRECTION, "in");
        f.changes.set(
            id(4),
            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(id(5))]),
            origin(),
        );
        f.own(3, 4);
        let snapshot = f.finish();
        let q = KerMlQueries::new(
            SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default())
                .unwrap(),
        );
        let codes = errors(&q, 4);
        assert_eq!(
            codes.contains("validateParameterMembershipOwningType"),
            !constructor
        );
        assert!(!codes.contains("validateParameterMembershipParameterDirection"));
    }
}
