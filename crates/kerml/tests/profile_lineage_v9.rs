use agq_kerml::BaselineProfile as P;
use std::collections::BTreeSet;

#[test]
fn seven_immutable_profiles_keep_their_earliest_correction_boundaries() {
    let mut identities = BTreeSet::new();
    let mut manifests = BTreeSet::new();
    for (index, profile) in [
        P::PublishedKerMl10,
        P::OPERATIONAL_V1,
        P::OPERATIONAL_V2,
        P::OPERATIONAL_V3,
        P::OPERATIONAL_V4,
        P::OPERATIONAL_V5,
        P::OPERATIONAL_V6,
    ]
    .into_iter()
    .enumerate()
    {
        identities.insert(profile.id());
        assert_eq!(profile.corrects_redefinition_resolution(), index >= 2);
        assert_eq!(profile.corrects_library_content(), index >= 3);
        assert_eq!(profile.corrects_redefinition_end_conformance(), index >= 4);
        assert_eq!(profile.corrects_formal_constraint_targets(), index >= 5);
        assert_eq!(profile.corrects_result_domains(), index >= 6);
        assert_eq!(profile.corrects_reference_binding(), index >= 6);
        assert_eq!(
            profile.result_domain_manifest_sha256().is_some(),
            index >= 6
        );
        assert_eq!(
            profile.reference_binding_manifest_sha256().is_some(),
            index >= 6
        );
        assert_eq!(profile.errata_manifest_sha256().is_some(), index >= 1);
        if let Some(digest) = profile.errata_manifest_sha256() {
            manifests.insert(digest);
        }
        agq_kerml::registry_for_profile(profile).unwrap();
    }
    assert_eq!(identities.len(), 7);
    assert_eq!(manifests.len(), 6);
    assert_ne!(
        P::OPERATIONAL_V6.result_domain_manifest_sha256(),
        P::OPERATIONAL_V6.reference_binding_manifest_sha256()
    );
}
