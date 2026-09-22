use agq_kerml::BaselineProfile as P;

#[test]
fn multiplicity_context_authority_is_added_only_in_v9() {
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
    ] {
        assert!(!profile.corrects_multiplicity_context());
        assert_eq!(profile.multiplicity_context_manifest_sha256(), None);
        assert_eq!(profile.cross_multiplicity_context_manifest_sha256(), None);
    }
    let v9 = P::OPERATIONAL_V9;
    assert_eq!(v9.id(), "agentique-kerml-1.0-operational/9");
    assert!(v9.corrects_multiplicity_context());
    assert!(v9.supports_publication_producers());
    assert!(v9.corrects_owned_cross_domain());
    assert!(v9.corrects_import_collisions());
    assert!(v9.accepts_correction_profile(P::OPERATIONAL_V3.id()));
    assert_ne!(
        v9.multiplicity_context_manifest_sha256(),
        v9.cross_multiplicity_context_manifest_sha256()
    );
    assert_ne!(
        v9.errata_manifest_sha256(),
        P::OPERATIONAL_V8.errata_manifest_sha256()
    );
    assert_eq!(
        v9.owned_cross_domain_manifest_sha256(),
        P::OPERATIONAL_V8.owned_cross_domain_manifest_sha256()
    );
    assert_eq!(
        v9.import_collision_manifest_sha256(),
        P::OPERATIONAL_V8.import_collision_manifest_sha256()
    );
    let old = agq_kerml::descriptors_for_profile(P::OPERATIONAL_V8).unwrap();
    let new = agq_kerml::descriptors_for_profile(v9).unwrap();
    assert_eq!(old.classes, new.classes);
    assert_eq!(old.properties, new.properties);
    assert_eq!(old.associations, new.associations);
    assert_eq!(P::OPERATIONAL, P::OPERATIONAL_V2);
}
