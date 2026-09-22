//! Synthetic archive fixtures do not authorize any source-library publication.
use super::*;
use agq_kerml::{classes as c, properties as p};
use agq_kernel::{
    ChangeSet, LibraryId, MetaclassId, PropertyId, Snapshot,
    metamodel::ValueKind,
    provenance::DeclaredOrigin,
    value::{SlotValue, Value as KernelValue},
};
use std::sync::Arc;

struct Fixture {
    base: Snapshot,
    changes: ChangeSet,
    next: u128,
    owned: BTreeMap<ElementId, Vec<KernelValue>>,
    libraries: BTreeMap<ElementId, LibraryId>,
    classes: BTreeMap<ElementId, MetaclassId>,
}

impl Fixture {
    fn new() -> Self {
        let base = Snapshot::new(Arc::new(
            agq_kerml::registry_for_profile(BaselineProfile::OPERATIONAL_V9).unwrap(),
        ));
        Self {
            changes: base.change_set(),
            base,
            next: 1,
            owned: BTreeMap::new(),
            libraries: BTreeMap::new(),
            classes: BTreeMap::new(),
        }
    }

    fn create(&mut self, class: MetaclassId, library: LibraryId) -> ElementId {
        let id = ElementId::from_u128(self.next);
        self.next += 1;
        self.libraries.insert(id, library);
        self.classes.insert(id, class);
        let origin = DeclaredOrigin::StandardLibrary { library };
        self.changes.create(id, class, origin.clone());
        let registry = self.base.model().registry();
        for property in registry.effective_properties(class).unwrap() {
            if property.derived || property.multiplicity.lower == 0 {
                continue;
            }
            let value = match registry.storage_kind(property.value_kind).unwrap() {
                ValueKind::Boolean => KernelValue::Boolean(false),
                ValueKind::String => KernelValue::String(String::new()),
                ValueKind::Enumeration(domain) => KernelValue::Enumeration(
                    *registry
                        .enumeration(domain)
                        .unwrap()
                        .literals
                        .iter()
                        .find(|(_, name)| name.as_str() == "public")
                        .unwrap()
                        .0,
                ),
                ValueKind::Reference(_) => continue,
                other => panic!("unhandled fixture primitive: {other:?}"),
            };
            self.changes
                .set(id, property.id, SlotValue::Scalar(value), origin.clone());
        }
        id
    }

    fn value(&mut self, id: ElementId, property: PropertyId, value: KernelValue) {
        self.changes.set(
            id,
            property,
            SlotValue::Scalar(value),
            DeclaredOrigin::StandardLibrary {
                library: self.libraries[&id],
            },
        );
    }

    fn member(
        &mut self,
        owner: ElementId,
        name: &str,
        class: MetaclassId,
        membership_class: MetaclassId,
    ) -> ElementId {
        let library = self.libraries[&owner];
        let target = self.create(class, library);
        self.value(
            target,
            p::ELEMENT_DECLARED_NAME,
            KernelValue::String(name.into()),
        );
        let membership_class = if membership_class == c::FEATURE_MEMBERSHIP
            && !self
                .base
                .model()
                .registry()
                .is_subtype(self.classes[&owner], c::TYPE)
                .unwrap()
        {
            c::OWNING_MEMBERSHIP
        } else {
            membership_class
        };
        let membership = self.create(membership_class, library);
        self.owned
            .entry(owner)
            .or_default()
            .push(KernelValue::Reference(membership));
        self.changes.set(
            membership,
            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![KernelValue::Reference(target)]),
            DeclaredOrigin::StandardLibrary { library },
        );
        target
    }

    fn finish(mut self) -> Snapshot {
        for (owner, relationships) in self.owned {
            self.changes.set(
                owner,
                p::ELEMENT_OWNED_RELATIONSHIP,
                SlotValue::Ordered(relationships),
                DeclaredOrigin::StandardLibrary {
                    library: self.libraries[&owner],
                },
            );
        }
        self.base.apply(&self.changes).unwrap()
    }
}

fn fixture() -> (
    CompletePublicationOverlay,
    Vec<ElementId>,
    LibrarySetIdentity,
) {
    let mut fixture = Fixture::new();
    let libraries = LibrarySetIdentity {
        artifacts: StandardLibraryArtifact::ALL
            .into_iter()
            .enumerate()
            .map(|(index, artifact)| (artifact, LibraryId::from_u128(100 + index as u128)))
            .collect(),
        pins: BTreeSet::from([LibraryPin {
            name: "private synthetic authored-publication fixture".into(),
            sha256: [0x42; 32],
        }]),
    };
    let roots_by_artifact: BTreeMap<_, _> = libraries
        .artifacts
        .iter()
        .map(|(&artifact, &library)| (artifact, fixture.create(c::NAMESPACE, library)))
        .collect();
    let mut paths: BTreeMap<(StandardLibraryArtifact, Vec<String>), ElementId> = BTreeMap::new();
    let mut result_owners = BTreeSet::new();
    for role in StandardRole::ALL {
        let artifact = role.library_artifact();
        let (segments, expected) = role.specification();
        let mut owner = roots_by_artifact[&artifact];
        for index in 0..segments.len() {
            let prefix: Vec<_> = segments[..=index].iter().map(|s| (*s).to_owned()).collect();
            let key = (artifact, prefix);
            if let Some(&id) = paths.get(&key) {
                owner = id;
                continue;
            }
            let class = if index + 1 == segments.len() {
                expected
            } else {
                match (role, index) {
                    (StandardRole::OccurrenceSnapshots | StandardRole::OccurrenceStartShot, 1) => {
                        c::CLASS
                    }
                    (StandardRole::ThingsThat, 1) => c::FEATURE,
                    (StandardRole::FeatureChainSourceTarget, 1) => c::FUNCTION,
                    (StandardRole::FeatureChainSourceTarget, 2) => c::FEATURE,
                    _ => c::LIBRARY_PACKAGE,
                }
            };
            let registry = fixture.base.model().registry();
            let is_feature = registry.is_subtype(class, c::FEATURE).unwrap();
            let needs_result = registry.is_subtype(class, c::FUNCTION).unwrap()
                || registry.is_subtype(class, c::EXPRESSION).unwrap();
            owner = fixture.member(
                owner,
                segments[index],
                class,
                if is_feature {
                    c::FEATURE_MEMBERSHIP
                } else {
                    c::OWNING_MEMBERSHIP
                },
            );
            paths.insert(key, owner);
            if needs_result {
                result_owners.insert(owner);
            }
        }
    }
    for owner in result_owners {
        fixture.member(owner, "result", c::FEATURE, c::RETURN_PARAMETER_MEMBERSHIP);
    }
    let occurrence = paths[&(
        StandardLibraryArtifact::Semantic,
        vec!["Occurrences".into(), "Occurrence".into()],
    )];
    let variable = fixture.member(
        occurrence,
        "variableFixture",
        c::FEATURE,
        c::FEATURE_MEMBERSHIP,
    );
    fixture.value(variable, p::FEATURE_IS_VARIABLE, KernelValue::Boolean(true));
    let snapshot = fixture.finish();
    let roots: Vec<_> = roots_by_artifact.values().copied().collect();
    let complete = CanonicalPublicationBuilder::new(&snapshot, &roots, &libraries)
        .build(16, |_| {})
        .expect("synthetic graph must pass real producer and capability closure");
    (complete, roots, libraries)
}

fn trusted_fixture(complete: &CompletePublicationOverlay) -> (Vec<u8>, AcceptedPublicationReceipt) {
    let mut graph = vec![];
    let overlay = complete.write_accepted_archive(&mut graph).unwrap();
    let identity = &overlay["identity"];
    let bindings = json!({
        "format":"agq-kerml-accepted-bindings/2",
        "operational_profile":identity["operational_profile"],
        "rule_set":identity["rule_set"],
        "semantic_publication_digest":identity["semantic_digest"],
        "library_set_identity":identity["library_set"],
        "binding_contract":identity["binding_contract"],
        "source_content_set":"private receipt-validation fixture",
    });
    let receipt = json!({
        "format":"agq-kerml-accepted-publication/1", "status":"accepted",
        "complete_overlay":overlay,
        "source_content_set":bindings["source_content_set"],
        "binding_manifest_sha256":json_digest(&bindings).unwrap(),
        "facade_metadata_sha256":json_digest(&json!({"source_map":[]})).unwrap(),
        "facade_metadata_bytes":serde_json::to_vec(&json!({"source_map":[]})).unwrap().len(),
    });
    let trusted = AcceptedPublicationReceipt::from_trusted_documents(
        &receipt.to_string(),
        &bindings.to_string(),
    )
    .unwrap();
    (graph, trusted)
}

fn decode(graph: &[u8]) -> DerivedOverlay {
    agq_kernel::archive::read_overlay(
        std::io::Cursor::new(graph),
        Arc::new(agq_kerml::registry_for_profile(BaselineProfile::OPERATIONAL_V9).unwrap()),
    )
    .unwrap()
}

#[test]
fn accepted_graph_roundtrip_retains_context_and_protected_dependency() {
    let (complete, roots, libraries) = fixture();
    let historical = context_identity(complete.context());
    assert!(historical.get("semantic_extensions").is_none());
    let mut composed = complete.context().clone();
    composed
        .semantic_extensions
        .insert("fixture-language/1", [7; 32]);
    let mut encoded = context_identity(&composed);
    assert_eq!(
        encoded["semantic_extensions"]["fixture-language/1"],
        json!(vec![7_u8; 32])
    );
    encoded
        .as_object_mut()
        .unwrap()
        .remove("semantic_extensions");
    assert_eq!(
        encoded, historical,
        "empty extension preserves historical receipt encoding"
    );
    let (graph, receipt) = trusted_fixture(&complete);
    let restored =
        CompletePublicationOverlay::restore_accepted(decode(&graph), &roots, &libraries, &receipt)
            .unwrap();
    assert_eq!(restored.context(), complete.context());
    assert_eq!(restored.checked_items(), complete.checked_items());
    assert!(restored.restored_from_receipt());
    assert!(!complete.restored_from_receipt());
    assert!(restored.stages().is_empty());
    assert_eq!(restored.counters(), &PublicationCounters::default());
    let mut reencoded = vec![];
    assert_eq!(
        restored.write_accepted_archive(&mut reencoded).unwrap(),
        receipt.receipt["complete_overlay"]
    );
    assert_eq!(graph, reencoded);
    let project = Snapshot::with_immutable_dependency(Arc::new(restored.overlay().clone()));
    assert!(
        restored
            .project_context(&project, roots[0], BTreeSet::new(), BTreeSet::new())
            .is_ok()
    );
    let other = Snapshot::with_immutable_dependency(Arc::new(complete.overlay().clone()));
    assert!(matches!(
        restored.project_context(&other, roots[0], BTreeSet::new(), BTreeSet::new()),
        Err(ContextError::PublicationDependencyMismatch)
    ));
}

#[test]
fn composed_candidate_context_keeps_dependency_obligations_and_asymmetric_roots() {
    let (complete, roots, _) = fixture();
    let dependency = Arc::new(complete.overlay().clone());
    let project = Snapshot::with_immutable_dependency_in_registry(
        dependency.clone(),
        Arc::new(agq_sysml::registry_for_profile(BaselineProfile::OPERATIONAL_V9).unwrap()),
    )
    .unwrap();
    let first = ElementId::from_u128(900001);
    let second = ElementId::from_u128(900002);
    let definition = ElementId::from_u128(900003);
    let mut changes = project.change_set();
    let authored = DeclaredOrigin::Authored { source: None };
    changes
        .create(first, c::NAMESPACE, authored.clone())
        .create(second, c::NAMESPACE, authored.clone())
        .create(definition, agq_sysml::classes::PART_DEFINITION, authored);
    let candidate = project.preview(&changes).unwrap();
    assert!(Arc::ptr_eq(
        candidate.immutable_dependency().unwrap(),
        &dependency
    ));
    assert!(!candidate.obligations().is_empty());
    assert!(project.apply(&changes).is_err());
    let context = complete
        .project_construction_context(
            &candidate,
            &[first, second],
            BTreeSet::new(),
            BTreeSet::from([first]),
        )
        .unwrap();
    assert_eq!(
        context.id().publication_dependency_digest,
        Some(complete.context().model_digest)
    );
    assert_eq!(
        context.id().construction_obligations.len(),
        candidate.obligations().len()
    );
    for &library_root in &roots {
        assert!(!context.id().available_roots[&library_root].contains(&first));
        assert!(context.id().available_roots[&first].contains(&library_root));
    }
    assert!(context.id().available_roots[&first].contains(&second));
    let query = KerMlQueries::new(context);
    assert!(std::ptr::eq(query.model(), candidate.model()));
    let missing = query.lookup_path(
        first,
        &QualifiedName {
            absolute: false,
            segments: vec!["not_yet_known".into()],
        },
    );
    assert_eq!(
        missing.map(|hits| hits.len()).completeness,
        Completeness::Incomplete
    );
    let unattached = Snapshot::new(Arc::new(
        agq_sysml::registry_for_profile(BaselineProfile::OPERATIONAL_V9).unwrap(),
    ));
    let other = unattached.preview(&unattached.change_set()).unwrap();
    assert!(matches!(
        complete.project_construction_context(&other, &[], BTreeSet::new(), BTreeSet::new()),
        Err(ContextError::PublicationDependencyMismatch)
    ));
    assert!(matches!(
        complete.project_construction_context(
            &candidate,
            &[first],
            BTreeSet::new(),
            BTreeSet::from([ElementId::from_u128(99_999_999)])
        ),
        Err(ContextError::InvalidPendingScope(_))
    ));
}

#[test]
fn restoration_rejects_graph_profile_rules_descriptor_library_and_source_tampering() {
    let (complete, roots, libraries) = fixture();
    let (graph, receipt) = trusted_fixture(&complete);
    let decoded = decode(&graph);
    let mut changes = decoded.declared().change_set();
    changes.set(
        roots[0],
        p::ELEMENT_DECLARED_NAME,
        SlotValue::Scalar(KernelValue::String("tampered-root".into())),
        DeclaredOrigin::StandardLibrary {
            library: libraries.artifacts[&StandardLibraryArtifact::Semantic],
        },
    );
    let changed = decoded.declared().apply(&changes).unwrap();
    let changed = agq_kernel::derived::DerivationBuilder::new(changed)
        .build()
        .unwrap();
    assert!(matches!(
        CompletePublicationOverlay::restore_accepted(changed, &roots, &libraries, &receipt),
        Err(PublicationRestoreError::Mismatch("kernel archive"))
    ));

    for field in [
        "operational_profile",
        "rule_set",
        "descriptor_sha256",
        "library_set",
        "authority",
    ] {
        let mut bad = AcceptedPublicationReceipt {
            receipt: receipt.receipt.clone(),
            bindings: receipt.bindings.clone(),
        };
        bad.receipt["complete_overlay"]["identity"][field] = json!("wrong identity");
        assert!(
            matches!(
                CompletePublicationOverlay::restore_accepted(
                    decode(&graph),
                    &roots,
                    &libraries,
                    &bad
                ),
                Err(PublicationRestoreError::Mismatch("semantic context"))
            ),
            "{field}"
        );
    }
    let mut wrong_libraries = libraries.clone();
    wrong_libraries.pins.insert(LibraryPin {
        name: "different-source".into(),
        sha256: [7; 32],
    });
    assert!(
        CompletePublicationOverlay::restore_accepted(
            decode(&graph),
            &roots,
            &wrong_libraries,
            &receipt
        )
        .is_err()
    );
    assert!(
        receipt
            .verify_facade_metadata(&json!({"source_map":["tampered"]}))
            .is_err()
    );
    let mut wrong_source = receipt.receipt.clone();
    wrong_source["source_content_set"] = json!("different bytes");
    assert!(
        AcceptedPublicationReceipt::from_trusted_documents(
            &wrong_source.to_string(),
            &receipt.bindings.to_string()
        )
        .is_err()
    );
    let mut stale_bindings = receipt.bindings.clone();
    stale_bindings["extra_stale_role"] = json!(true);
    assert!(
        AcceptedPublicationReceipt::from_trusted_documents(
            &receipt.receipt.to_string(),
            &stale_bindings.to_string()
        )
        .is_err()
    );
}

#[test]
fn success_labels_cannot_establish_trusted_acceptance() {
    assert!(matches!(
        AcceptedPublicationReceipt::from_trusted_documents(
            r#"{"format":"agq-kerml-accepted-publication/1","status":"accepted"}"#,
            r#"{"format":"agq-kerml-accepted-bindings/2","operational_profile":"operational-v9"}"#,
        ),
        Err(PublicationRestoreError::NotAccepted | PublicationRestoreError::Mismatch(_))
    ));
}

#[test]
fn restoration_requires_each_capability_family_once_and_all_standard_roles() {
    let (complete, roots, libraries) = fixture();
    let (graph, receipt) = trusted_fixture(&complete);
    for case in 0..3 {
        let mut bad = AcceptedPublicationReceipt {
            receipt: receipt.receipt.clone(),
            bindings: receipt.bindings.clone(),
        };
        let entries = bad.receipt["complete_overlay"]["checked"]
            .as_array_mut()
            .unwrap();
        match case {
            0 => {
                entries.pop();
            }
            1 => {
                entries[1] = entries[0].clone();
            }
            _ => {
                let bindings = entries
                    .iter_mut()
                    .find(|entry| entry["family"] == "StandardBindings")
                    .unwrap();
                bindings["count"] = json!(1);
            }
        }
        assert!(
            matches!(
                CompletePublicationOverlay::restore_accepted(
                    decode(&graph),
                    &roots,
                    &libraries,
                    &bad
                ),
                Err(PublicationRestoreError::Mismatch(
                    "complete capability population"
                ))
            ),
            "case {case}"
        );
    }
}
