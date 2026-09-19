use agq_kerml::*;
use agq_kernel::metamodel::*;
use std::collections::BTreeSet;

fn key(id: DescriptorId) -> String {
    match id {
        DescriptorId::Property(id) => id.to_string(),
        DescriptorId::Association(id) => id.to_string(),
        _ => panic!("only reviewed properties and associations may disappear"),
    }
}

#[test]
fn exact_reviewed_diff_leaves_all_other_descriptors_and_provenance_unchanged() {
    let raw = published_descriptors();
    let operational = operational_descriptors(OperationalErrataProfile::ReviewedV1).unwrap();
    let manifest: serde_json::Value = serde_json::from_str(OPERATIONAL_ERRATA_MANIFEST).unwrap();
    assert_eq!(manifest["profile_id"], BaselineProfile::OPERATIONAL.id());
    assert_eq!(
        manifest["published_profile_id"],
        BaselineProfile::PublishedKerMl10.id()
    );
    let expected: BTreeSet<_> = manifest["entries"][0]["descriptors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["descriptor_id"].as_str().unwrap().to_owned())
        .collect();
    let removed: BTreeSet<_> = raw
        .sources
        .keys()
        .filter(|id| !operational.sources.contains_key(id))
        .map(|id| key(*id))
        .collect();
    assert_eq!(removed, expected);
    for row in manifest["entries"][0]["descriptors"].as_array().unwrap() {
        let (_, source) = raw
            .sources
            .iter()
            .find(|(id, _)| {
                matches!(id, DescriptorId::Property(_) | DescriptorId::Association(_))
                    && key(**id) == row["descriptor_id"].as_str().unwrap()
            })
            .unwrap();
        assert_eq!(source.external_id, row["external_id"]);
        assert_eq!(
            source.byte_range,
            [
                row["byte_range"][0].as_u64().unwrap() as usize,
                row["byte_range"][1].as_u64().unwrap() as usize
            ]
        );
        assert_eq!(source.artifact_uri, manifest["entries"][0]["artifact_uri"]);
        assert_eq!(source.sha256, manifest["entries"][0]["artifact_sha256"]);
    }
    assert_eq!(raw.models, operational.models);
    assert_eq!(raw.classes, operational.classes);
    assert_eq!(raw.enumerations, operational.enumerations);
    assert_eq!(raw.primitives, operational.primitives);
    assert_eq!(raw.reviews, operational.reviews);
    assert_eq!(raw.associations.len() - operational.associations.len(), 2);
    assert_eq!(raw.properties.len() - operational.properties.len(), 4);
    for a in &operational.associations {
        assert!(raw.associations.contains(a));
    }
    for p in &operational.properties {
        assert!(raw.properties.contains(p));
    }
    for (id, source) in &operational.sources {
        assert_eq!(raw.sources.get(id), Some(source));
    }
    assert_eq!(raw.sources, published_descriptors().sources);
    registry_for_profile(BaselineProfile::OPERATIONAL).unwrap();
    println!(
        "published baseline: {} associations, {} properties",
        raw.associations.len(),
        raw.properties.len()
    );
    println!(
        "Agentique operational errata profile: {} associations, {} properties; exact reviewed deletion closure = {}",
        operational.associations.len(),
        operational.properties.len(),
        removed.len()
    );
}

#[test]
fn transform_rejects_any_unreviewed_source_graph_including_hashes_and_duplicates() {
    let mut cases = vec![];
    let mut hash = published_descriptors();
    hash.sources.values_mut().next().unwrap().sha256 = "another artifact".into();
    cases.push(hash);
    let mut other_version = published_descriptors();
    for source in other_version.sources.values_mut() {
        if source.specification == "KerML" {
            source.version = "1.1".into();
        }
    }
    cases.push(other_version);
    let mut dangling = published_descriptors();
    dangling.properties[0]
        .subsets
        .insert(properties::A_PARTICIPANT_FEATURE_INTERACTION_PARTICIPANT_FEATURE);
    cases.push(dangling);
    let mut duplicate = published_descriptors();
    duplicate.properties.push(duplicate.properties[0].clone());
    cases.push(duplicate);
    let mut missing = published_descriptors();
    missing.properties.pop();
    cases.push(missing);
    let mut changed = published_descriptors();
    changed.classes[0].name = "changed".into();
    cases.push(changed);
    for input in cases {
        assert!(matches!(
            apply_operational_errata(&input, OperationalErrataProfile::ReviewedV1),
            Err(ProfileError::UnreviewedGraph)
        ));
    }
}
