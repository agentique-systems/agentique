use agq_kerml::BaselineProfile;
use agq_kerml_semantics::{ContextError, SemanticContext, SemanticOptions};
use agq_kernel::Snapshot;
use std::sync::Arc;

#[test]
fn contexts_bind_profile_artifact_effective_graph_and_errata_content() {
    let published = Snapshot::new(Arc::new(agq_kerml::registry().unwrap()));
    let operational = Snapshot::new(Arc::new(
        agq_kerml::registry_for_profile(BaselineProfile::OPERATIONAL).unwrap(),
    ));
    let raw =
        SemanticContext::for_snapshot(&published, Default::default(), Default::default()).unwrap();
    let options = SemanticOptions {
        baseline_profile: BaselineProfile::OPERATIONAL,
        ..Default::default()
    };
    let effective =
        SemanticContext::for_snapshot(&operational, options.clone(), Default::default()).unwrap();
    assert_eq!(raw.id().metamodel_version, effective.id().metamodel_version);
    assert_eq!(raw.id().model_digest, effective.id().model_digest);
    assert_ne!(raw.id().descriptor_digest, effective.id().descriptor_digest);
    assert_ne!(
        raw.id().baseline_profile_id,
        effective.id().baseline_profile_id
    );
    assert!(raw.id().errata_manifest_digest.is_none());
    assert!(effective.id().errata_manifest_digest.is_some());
    assert!(matches!(
        SemanticContext::for_snapshot(&published, options, Default::default()),
        Err(ContextError::UnsupportedMetamodel)
    ));
    assert!(matches!(
        SemanticContext::for_snapshot(&operational, Default::default(), Default::default()),
        Err(ContextError::UnsupportedMetamodel)
    ));
    let v1 = SemanticContext::for_snapshot(
        &operational,
        SemanticOptions {
            baseline_profile: BaselineProfile::OPERATIONAL_V1,
            ..Default::default()
        },
        Default::default(),
    )
    .unwrap();
    assert_eq!(v1.id().descriptor_digest, effective.id().descriptor_digest);
    assert_eq!(v1.id().model_digest, effective.id().model_digest);
    assert_ne!(v1.id(), effective.id());
    assert_ne!(
        v1.id().errata_manifest_digest,
        effective.id().errata_manifest_digest
    );
    let mut changed_review = effective.id().clone();
    changed_review.errata_manifest_digest = Some([0; 32]);
    assert_ne!(&changed_review, effective.id());
    let v3 = SemanticContext::for_snapshot(
        &operational,
        SemanticOptions {
            baseline_profile: BaselineProfile::OPERATIONAL_V3,
            ..Default::default()
        },
        Default::default(),
    )
    .unwrap();
    assert_eq!(v3.id().descriptor_digest, effective.id().descriptor_digest);
    assert_ne!(
        v3.id().errata_manifest_digest,
        effective.id().errata_manifest_digest
    );
    assert_ne!(v3.id(), v1.id());
    assert_ne!(v3.id(), raw.id());
}

#[test]
fn operational_context_cannot_smuggle_deleted_descriptors_back_as_an_extension() {
    use agq_kernel::metamodel::MetamodelRegistry;
    let raw = agq_kerml::published_descriptors();
    let mut set = agq_kerml::descriptors_for_profile(BaselineProfile::OPERATIONAL).unwrap();
    for association in raw.associations {
        if !set.associations.iter().any(|a| a.id == association.id) {
            set.associations.push(association);
        }
    }
    for property in raw.properties {
        if !set.properties.iter().any(|p| p.id == property.id) {
            set.properties.push(property);
        }
    }
    // Deliberately omit the raw provenance, so checking sources alone is insufficient.
    let snapshot = Snapshot::new(Arc::new(MetamodelRegistry::from_descriptors(set).unwrap()));
    let options = SemanticOptions {
        baseline_profile: BaselineProfile::OPERATIONAL,
        ..Default::default()
    };
    assert!(matches!(
        SemanticContext::for_snapshot(&snapshot, options, Default::default()),
        Err(ContextError::UnsupportedMetamodel)
    ));
}
