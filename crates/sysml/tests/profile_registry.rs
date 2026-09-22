use agq_kerml::BaselineProfile as P;

#[test]
fn full_sysml_extends_each_exact_kerml_profile_without_changing_old_class_contracts() {
    for profile in [
        P::PublishedKerMl10,
        P::OPERATIONAL_V1,
        P::OPERATIONAL_V2,
        P::OPERATIONAL_V3,
        P::OPERATIONAL_V4,
        P::OPERATIONAL_V5,
        P::OPERATIONAL_V6,
        P::OPERATIONAL_V7,
        P::OPERATIONAL_V8,
        P::OPERATIONAL_V9,
    ] {
        let base = agq_kerml::registry_for_profile(profile).unwrap();
        let extension = agq_sysml::registry_for_profile(profile).unwrap();
        extension.require_extension_of(&base).unwrap();
        assert_eq!(
            extension.classes().count(),
            base.classes().count() + agq_sysml::CLASS_IDS.len()
        );
        assert!(
            extension
                .is_subtype(agq_sysml::classes::PART_USAGE, agq_kerml::classes::FEATURE)
                .unwrap()
        );
    }
}

#[test]
fn legacy_sysml_registration_preserves_published_metadata() {
    let legacy = agq_sysml::registry().unwrap();
    let explicit = agq_sysml::registry_for_profile(P::PublishedKerMl10).unwrap();
    legacy.require_extension_of(&explicit).unwrap();
    explicit.require_extension_of(&legacy).unwrap();
    let operational = agq_sysml::registry_for_profile(P::OPERATIONAL_V9).unwrap();
    assert!(
        legacy
            .properties()
            .any(|property| operational.property(property.id).is_err())
    );
    // The language-neutral extension check is not profile authentication.
    // SemanticContext separately rejects excluded published descriptors.
}
