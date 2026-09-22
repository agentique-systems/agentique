use agq_kerml::BaselineProfile as P;

#[test]
fn operational_default_uses_accepted_v9_without_changing_legacy_or_explicit_selection() {
    assert_eq!(P::OPERATIONAL, P::OPERATIONAL_V9);
    assert_eq!(P::OPERATIONAL.id(), "agentique-kerml-1.0-operational/9");
    assert_eq!(P::default(), P::PublishedKerMl10);
    assert_eq!(P::OPERATIONAL_V2.id(), "agentique-kerml-1.0-operational/2");
    assert!(!P::OPERATIONAL_V2.corrects_multiplicity_context());
    assert!(P::OPERATIONAL.corrects_multiplicity_context());
    assert_eq!(
        P::OPERATIONAL.errata_manifest_sha256(),
        P::OPERATIONAL_V9.errata_manifest_sha256()
    );
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
        agq_kerml::registry_for_profile(profile).unwrap();
    }
}

#[test]
fn alias_activation_does_not_rewrite_the_reviewed_candidate_manifest() {
    // The frozen manifest records the default when v9 was reviewed. Publication
    // acceptance and alias activation have separate evidence and identities.
    let manifest: serde_json::Value =
        serde_json::from_str(agq_kerml::OPERATIONAL_PROFILE_V9_MANIFEST).unwrap();
    assert_eq!(manifest["default_profile"], P::OPERATIONAL_V2.id());
    assert_eq!(manifest["profile_id"], P::OPERATIONAL_V9.id());
    assert_ne!(P::OPERATIONAL, P::OPERATIONAL_V2);
}
