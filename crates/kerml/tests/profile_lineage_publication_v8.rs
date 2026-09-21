use agq_kerml::BaselineProfile as P;

#[test]
fn publication_interpretations_first_apply_in_v8_with_independent_manifest_identities() {
    for p in [
        P::PublishedKerMl10,
        P::OPERATIONAL_V1,
        P::OPERATIONAL_V2,
        P::OPERATIONAL_V3,
        P::OPERATIONAL_V4,
        P::OPERATIONAL_V5,
        P::OPERATIONAL_V6,
        P::OPERATIONAL_V7,
    ] {
        assert!(!p.corrects_owned_cross_domain());
        assert!(!p.corrects_import_collisions());
        assert_eq!(p.owned_cross_domain_manifest_sha256(), None);
        assert_eq!(p.import_collision_manifest_sha256(), None);
    }
    let v8 = P::OPERATIONAL_V8;
    assert_eq!(v8.id(), "agentique-kerml-1.0-operational/8");
    assert!(v8.corrects_owned_cross_domain());
    assert!(v8.corrects_import_collisions());
    assert_ne!(
        v8.owned_cross_domain_manifest_sha256(),
        v8.import_collision_manifest_sha256()
    );
    assert_ne!(
        v8.errata_manifest_sha256(),
        P::OPERATIONAL_V7.errata_manifest_sha256()
    );
    assert_eq!(
        v8.owned_cross_feature_manifest_sha256(),
        P::OPERATIONAL_V7.owned_cross_feature_manifest_sha256()
    );
    assert_eq!(
        v8.result_domain_manifest_sha256(),
        P::OPERATIONAL_V6.result_domain_manifest_sha256()
    );
    assert_eq!(
        v8.reference_binding_manifest_sha256(),
        P::OPERATIONAL_V6.reference_binding_manifest_sha256()
    );
    assert!(v8.accepts_correction_profile(P::OPERATIONAL_V3.id()));
    let v7 = agq_kerml::descriptors_for_profile(P::OPERATIONAL_V7).unwrap();
    let v8 = agq_kerml::descriptors_for_profile(v8).unwrap();
    assert_eq!(v7.classes, v8.classes);
    assert_eq!(v7.properties, v8.properties);
    assert_eq!(v7.associations, v8.associations);
    assert_eq!(P::OPERATIONAL, P::OPERATIONAL_V2);
}
