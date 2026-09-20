use agq_kerml::{BaselineProfile, classes as c, properties as p};
use agq_kerml_semantics::Completeness;
use agq_kernel::value::Value;
use agq_standard_libraries::VerifiedLibrarySet;

#[test]
fn every_pinned_end_membership_constructs_an_end_without_retyping_referents() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root).unwrap();
    let draft = agq_kerml_text::library::lower_declarations_with_profile(
        &sources,
        BaselineProfile::OPERATIONAL_V5,
    )
    .unwrap();
    let q = draft.queries(&sources).unwrap();
    let model = draft.candidate().model();
    let mut count = 0;
    for membership in model.instances(c::END_FEATURE_MEMBERSHIP, true).unwrap() {
        let member = q.member(membership.id());
        assert_eq!(member.completeness, Completeness::Complete);
        let feature = member.value.unwrap();
        assert!(
            matches!(
                model
                    .navigation_slot(feature, p::FEATURE_IS_END)
                    .unwrap()
                    .value()
                    .values()
                    .next(),
                Some(Value::Boolean(true))
            ),
            "{}",
            membership.id()
        );
        let validation = q.validate_local_structure(membership.id());
        assert_eq!(
            validation.completeness,
            Completeness::Complete,
            "{:?}",
            validation.diagnostics
        );
        assert!(
            validation
                .value
                .contains(&"validateEndFeatureMembershipIsEnd")
        );
        count += 1;
    }
    assert!(count > 0);
    println!("EndFeatureMembership instances checked: {count}; end findings: 0");
    for rule in agq_kerml_semantics::FormalConstraintId::ALL {
        let target = q.formal_constraint_target(rule);
        assert_eq!(
            target.completeness,
            Completeness::Complete,
            "{rule:?}: {:?}",
            target.diagnostics
        );
        assert!(target.value.is_some());
    }
}
