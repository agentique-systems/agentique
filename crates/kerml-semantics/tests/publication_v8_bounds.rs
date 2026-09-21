include!("common/result_fixture.rs");

#[test]
fn structural_bounds_use_owned_expression_order_without_evaluation() {
    for count in 0..=3 {
        let mut f = Fixture::new();
        f.create(1, c::MULTIPLICITY_RANGE);
        for (index, n) in [90, 40, 10].into_iter().take(count).enumerate() {
            f.create(n, c::EXPRESSION);
            member(&mut f, 1, n, 100 + index as u128, c::OWNING_MEMBERSHIP);
        }
        let snapshot = f.finish();
        let q = KerMlQueries::new(
            SemanticContext::for_snapshot(&snapshot, Default::default(), Default::default())
                .unwrap(),
        );
        let bounds = q.multiplicity_bounds(id(1));
        assert_eq!(bounds.completeness, Completeness::Complete);
        let expected = match count {
            0 => vec![],
            1 => vec![id(90)],
            _ => vec![id(90), id(40)],
        };
        assert_eq!(bounds.value.bound, expected);
        assert_eq!(bounds.value.lower, (count >= 2).then_some(id(90)));
        assert_eq!(bounds.value.upper, expected.last().copied());
        // The KERML11-4 validation conflict does not change canonical ownership/domain.
        assert_eq!(q.owning_type(id(1)).value, None);
        assert!(q.featuring_types(id(1)).value.is_empty());
    }
}
