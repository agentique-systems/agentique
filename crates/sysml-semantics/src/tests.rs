//! Synthetic canonical fixtures deliberately bypass production acceptance only
//! through a private cfg(test) constructor. Real-cache integration is separate.
#[path = "producer_tests.rs"]
mod producer_tests;
#[path = "structural_query_tests.rs"]
mod structural_query_tests;
#[path = "transition_tests.rs"]
mod transition_tests;

#[test]
fn scalar_inverse_inventory_is_ownership_bounded() {
    // These are the scalar inverse populations used by the pinned combined
    // graph. Both have precise ownership reads in KerMLQueries::property;
    // the generic Incoming fallback is not reached by current library queries.
    for profile in [
        agq_kerml::BaselineProfile::PublishedKerMl10,
        agq_kerml::BaselineProfile::OPERATIONAL_V9,
    ] {
        let registry = agq_sysml::registry_for_profile(profile).unwrap();
        let inverses: BTreeSet<_> = registry
            .properties()
            .filter_map(|property| {
                registry
                    .inverse_storage(property.id)
                    .unwrap()
                    .map(|storage| (property.id, storage))
            })
            .collect();
        assert_eq!(
            inverses,
            BTreeSet::from([
                (
                    kp::ELEMENT_OWNING_RELATIONSHIP,
                    kp::RELATIONSHIP_OWNED_RELATED_ELEMENT
                ),
                (
                    kp::RELATIONSHIP_OWNING_RELATED_ELEMENT,
                    kp::ELEMENT_OWNED_RELATIONSHIP
                ),
            ]),
            "{profile:?}"
        );
    }
}

#[test]
fn mounted_dependency_cannot_substitute_a_weaker_producer_registry() {
    use agq_kerml_semantics::{
        ProducerClosedDependency, ProducerClosureCertificate, ProducerRegistry,
    };
    let snapshot = Fixture::new().base;
    let overlay = Arc::new(
        agq_kernel::derived::DerivationBuilder::new(snapshot)
            .build()
            .unwrap(),
    );
    let registry = ProducerRegistry::new([]).unwrap();
    let context = SemanticContext::for_overlay(
        &overlay,
        SemanticOptions {
            baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
            ..Default::default()
        },
        BTreeSet::new(),
    )
    .unwrap()
    .with_producer_registry_digest(registry.digest())
    .unwrap();
    let certificate = Arc::new(ProducerClosureCertificate::initial(&context, &registry).unwrap());
    let context = context.with_producer_closure(certificate).unwrap();
    let dependency = ProducerClosedDependency::new(overlay.clone(), &context, &registry).unwrap();
    let snapshot = dependency.project_snapshot();
    let context = dependency
        .project_context(&snapshot, &[], BTreeSet::new(), BTreeSet::new())
        .unwrap();
    let bindings = StandardSysmlBindings::unbound(SystemsLibraryIdentity::pinned([0; 32]));
    let expected = SysmlDependencyContract::checked_in_for_profile(
        &bindings,
        SysmlBaselineProfile::OperationalV2,
    )
    .unwrap();
    assert!(matches!(
        SysmlSemanticContext::for_closed_dependency(context, &expected, bindings),
        Err(SysmlContextError::KerMl(
            agq_kerml_semantics::ContextError::ProducerRegistryIdentityMismatch
        ))
    ));
}
use super::*;
use agq_kerml::{classes as kc, properties as kp};
use agq_kerml_semantics::{Completeness, KerMlQueries, SemanticContext, SemanticOptions};
use agq_kernel::{
    metamodel::ValueKind,
    provenance::DeclaredOrigin,
    value::{SlotValue, Value},
    *,
};
use agq_sysml::classes as sc;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

fn id(n: u128) -> ElementId {
    ElementId::from_u128(n)
}
fn origin() -> DeclaredOrigin {
    DeclaredOrigin::Authored { source: None }
}
struct Fixture {
    base: Snapshot,
    changes: ChangeSet,
    owned: BTreeMap<ElementId, Vec<Value>>,
    origin: DeclaredOrigin,
}
impl Fixture {
    fn new() -> Self {
        let base = Snapshot::new(Arc::new(
            agq_sysml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V9).unwrap(),
        ));
        let changes = base.change_set();
        Self {
            base,
            changes,
            owned: BTreeMap::new(),
            origin: origin(),
        }
    }
    fn create(&mut self, n: u128, class: MetaclassId, name: &str) {
        self.changes.create(id(n), class, self.origin.clone());
        for property in self
            .base
            .model()
            .registry()
            .effective_properties(class)
            .unwrap()
        {
            if property.derived || property.multiplicity.lower == 0 {
                continue;
            }
            let value = match self
                .base
                .model()
                .registry()
                .storage_kind(property.value_kind)
                .unwrap()
            {
                ValueKind::Boolean => Value::Boolean(false),
                ValueKind::String => Value::String(name.into()),
                ValueKind::Enumeration(domain) => Value::Enumeration(
                    *self
                        .base
                        .model()
                        .registry()
                        .enumeration(domain)
                        .unwrap()
                        .literals
                        .iter()
                        .find(|(_, name)| name.as_str() == "public")
                        .or_else(|| {
                            self.base
                                .model()
                                .registry()
                                .enumeration(domain)
                                .unwrap()
                                .literals
                                .iter()
                                .next()
                        })
                        .unwrap()
                        .0,
                ),
                ValueKind::Reference(_) => continue,
                other => panic!("fixture domain {other:?}"),
            };
            self.changes.set(
                id(n),
                property.id,
                SlotValue::Scalar(value),
                self.origin.clone(),
            );
        }
        self.value(n, kp::ELEMENT_DECLARED_NAME, Value::String(name.into()));
    }
    fn value(&mut self, n: u128, property: PropertyId, value: Value) {
        if !self
            .base
            .model()
            .registry()
            .supports_slot_storage(property)
            .unwrap()
        {
            let descriptor = self.base.model().registry().property(property).unwrap();
            let Value::Reference(target) = value else {
                panic!("reference occurrence")
            };
            self.changes.link(
                AssociationOccurrenceId::new(),
                descriptor.association.unwrap(),
                BTreeMap::from([
                    (property, target),
                    (*descriptor.opposite_ends.first().unwrap(), id(n)),
                ]),
                BTreeMap::new(),
                self.origin.clone(),
            );
        } else {
            self.changes.set(
                id(n),
                property,
                SlotValue::Scalar(value),
                self.origin.clone(),
            );
        }
    }
    fn member(&mut self, owner: u128, member: u128, relation: u128, class: MetaclassId) {
        self.create(relation, class, "membership");
        self.owned
            .entry(id(owner))
            .or_default()
            .push(Value::Reference(id(relation)));
        self.changes.set(
            id(relation),
            kp::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(id(member))]),
            self.origin.clone(),
        );
    }
    fn relation(
        &mut self,
        specific: u128,
        general: u128,
        relation: u128,
        class: MetaclassId,
        target: PropertyId,
    ) {
        self.create(relation, class, "relationship");
        self.owned
            .entry(id(specific))
            .or_default()
            .push(Value::Reference(id(relation)));
        self.value(relation, target, Value::Reference(id(general)));
        let source = if class == kc::FEATURE_TYPING {
            kp::FEATURE_TYPING_TYPED_FEATURE
        } else if class == kc::REDEFINITION {
            kp::REDEFINITION_REDEFINING_FEATURE
        } else if class == kc::SUBCLASSIFICATION {
            kp::SUBCLASSIFICATION_SUBCLASSIFIER
        } else if class == kc::SUBSETTING {
            kp::SUBSETTING_SUBSETTING_FEATURE
        } else {
            kp::SPECIALIZATION_SPECIFIC
        };
        self.value(relation, source, Value::Reference(id(specific)));
    }
    fn finish(mut self) -> Snapshot {
        for (owner, relationships) in self.owned {
            self.changes.set(
                owner,
                kp::ELEMENT_OWNED_RELATIONSHIP,
                SlotValue::Ordered(relationships),
                self.origin.clone(),
            );
        }
        self.base.apply(&self.changes).unwrap()
    }
}
fn vertical() -> Fixture {
    let mut f = Fixture::new();
    f.create(10_000, kc::NAMESPACE, "temporaryRootName");
    f.changes.clear(id(10_000), kp::ELEMENT_DECLARED_NAME);
    f.create(1, sc::PART_DEFINITION, "Engine");
    f.create(2, sc::PART_DEFINITION, "Vehicle");
    f.create(3, sc::PART_USAGE, "engine");
    f.create(4, sc::PART_DEFINITION, "SportsCar");
    f.member(10_000, 1, 10_001, kc::OWNING_MEMBERSHIP);
    f.member(10_000, 2, 10_002, kc::OWNING_MEMBERSHIP);
    f.member(10_000, 4, 10_003, kc::OWNING_MEMBERSHIP);
    f.member(2, 3, 103, kc::FEATURE_MEMBERSHIP);
    f.relation(3, 1, 201, kc::FEATURE_TYPING, kp::FEATURE_TYPING_TYPE);
    f.relation(
        4,
        2,
        202,
        kc::SUBCLASSIFICATION,
        kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
    );
    f
}
fn q(snapshot: &Snapshot) -> SysmlQueries<'_> {
    SysmlQueries::new(crate::context::fixture_context(snapshot, BTreeSet::new()))
}

#[test]
fn programmatic_vertical_reuses_original_inherited_usage_without_allocating_records() {
    let snapshot = vertical().finish();
    let before = snapshot.model().len();
    let queries = q(&snapshot);
    let types = queries.current_part_definitions(id(3));
    assert_eq!(types.value(), &[id(1)]);
    assert_eq!(types.completeness(), Completeness::Complete, "{types:?}");
    assert_eq!(queries.direct_usage_types(id(3)).value(), &[id(1)]);
    assert_eq!(queries.direct_specializations(id(4)).value(), &[id(2)]);
    assert_eq!(
        queries
            .current_supertypes(id(4))
            .value()
            .iter()
            .copied()
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([id(2), id(4)])
    );
    assert!(queries.owned_usages(id(4)).value().is_empty());
    let inherited = queries.current_effective_usages(id(4));
    assert_eq!(inherited.value(), &[id(3)]);
    assert_eq!(
        inherited.completeness(),
        Completeness::Complete,
        "{inherited:?}"
    );
    assert!(!inherited.kerml.positive_dependencies.is_empty());
    assert_eq!(
        queries.current_qualified_name(id(3)).value(),
        &Some(QualifiedNamePath {
            segments: vec![
                BTreeSet::from(["Vehicle".to_owned()]),
                BTreeSet::from(["engine".to_owned()])
            ]
        })
    );
    assert_eq!(snapshot.model().len(), before);
    assert_eq!(
        queries.current_usage_types(id(3)).kerml,
        queries.kerml().feature_types(id(3))
    );
}

#[test]
fn retained_sysml_context_preserves_answers_and_outlives_original_snapshot() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<crate::RetainedSysmlContext>();
    let snapshot = vertical().finish();
    let original = q(&snapshot);
    let inherited = original.current_effective_usages(id(4));
    let unclosed = original.effective_usages(id(4));
    let absent = original.current_part_definitions(id(999));
    let context = original.context().clone();
    let retained = original.retain_context();
    drop(original);
    drop(snapshot);
    let queries = retained.queries();
    assert_eq!(queries.context(), &context);
    // Include every composed evidence/status field, even fields which have no
    // public PartialEq implementation. No identity normalization is performed.
    assert_eq!(format!("{:?}", queries.current_effective_usages(id(4))), format!("{inherited:?}"));
    assert_eq!(format!("{:?}", queries.effective_usages(id(4))), format!("{unclosed:?}"));
    assert_eq!(format!("{:?}", queries.current_part_definitions(id(999))), format!("{absent:?}"));
}

#[test]
fn attribute_and_item_projections_keep_valid_non_sysml_classifier_targets() {
    let mut f = Fixture::new();
    f.create(1, kc::DATA_TYPE, "OrdinaryKerMlData");
    f.create(2, kc::STRUCTURE, "OrdinaryKerMlStructure");
    f.create(3, sc::ATTRIBUTE_USAGE, "arbitraryAttribute");
    f.create(4, sc::ITEM_USAGE, "arbitraryItem");
    f.create(5, sc::ATTRIBUTE_USAGE, "untypedAttribute");
    f.relation(3, 1, 101, kc::FEATURE_TYPING, kp::FEATURE_TYPING_TYPE);
    f.relation(4, 2, 102, kc::FEATURE_TYPING, kp::FEATURE_TYPING_TYPE);
    let snapshot = f.finish();
    let queries = q(&snapshot);
    for (subject, expected) in [(3, 1), (4, 2)] {
        assert_eq!(
            queries.current_usage_types(id(subject)).value(),
            &[id(expected)]
        );
    }
    assert_eq!(
        queries.current_attribute_definitions(id(3)).value(),
        &[id(1)]
    );
    assert_eq!(queries.current_item_definitions(id(4)).value(), &[id(2)]);
    assert!(queries.direct_usage_types(id(5)).value().is_empty());
    assert_eq!(
        queries.direct_usage_types(id(5)).completeness(),
        Completeness::Complete
    );
    assert_eq!(
        queries.effective_usage_types(id(5)).completeness(),
        Completeness::Incomplete
    );
}

#[test]
fn invalid_typed_endpoints_are_reported_and_retained_as_rejected_evidence() {
    let mut f = Fixture::new();
    f.create(1, kc::DATA_TYPE, "notAPartDefinition");
    f.create(2, sc::PART_USAGE, "aPart");
    f.relation(2, 1, 101, kc::FEATURE_TYPING, kp::FEATURE_TYPING_TYPE);
    let snapshot = f.finish();
    let answer = q(&snapshot).current_part_definitions(id(2));
    assert!(answer.value().is_empty());
    assert_eq!(answer.rejected_targets, BTreeSet::from([id(1)]));
    assert_eq!(answer.completeness(), Completeness::Invalid);
    assert!(
        answer
            .observations
            .contains_key(&provenance::FactKey::Element(id(1)))
    );
}

#[test]
fn pending_specialization_survives_the_usage_filter() {
    let snapshot = vertical().finish();
    let queries = SysmlQueries::new(crate::context::fixture_context(
        &snapshot,
        BTreeSet::from([id(4)]),
    ));
    let answer = queries.current_effective_usages(id(4));
    assert_eq!(answer.value(), &[id(3)]);
    assert_eq!(answer.completeness(), Completeness::Incomplete);
    assert!(
        answer
            .kerml
            .diagnostics
            .iter()
            .any(|d| d.code == "KQ_PENDING_INHERITANCE")
    );
}

#[test]
fn owned_redefinition_suppresses_inherited_usage_by_original_identity() {
    let mut f = vertical();
    f.create(5, sc::PART_USAGE, "replacement");
    f.member(4, 5, 105, kc::FEATURE_MEMBERSHIP);
    f.relation(
        5,
        3,
        205,
        kc::REDEFINITION,
        kp::REDEFINITION_REDEFINED_FEATURE,
    );
    let snapshot = f.finish();
    let queries = q(&snapshot);
    assert_eq!(queries.current_effective_usages(id(4)).value(), &[id(5)]);
    assert_eq!(queries.redefined_features(id(5)).value(), &[id(3)]);
    assert_eq!(queries.current_usage_types(id(5)).value(), &[id(1)]);
    assert!(snapshot.model().element(id(3)).is_some());
}

#[test]
fn effective_answers_expose_missing_base_rules_and_derived_may_time_vary() {
    let mut f = vertical();
    f.value(3, kp::FEATURE_IS_COMPOSITE, Value::Boolean(true));
    let snapshot = f.finish();
    let queries = q(&snapshot);
    let answer = queries.effective_usage_types(id(3));
    assert_eq!(answer.value(), &[id(1)]);
    assert_eq!(answer.completeness(), Completeness::Incomplete);
    for rule in [
        PendingSysmlRule::StandardGeneralization(StandardSysmlRole::Parts),
        PendingSysmlRule::MayTimeVary,
        PendingSysmlRule::CompositeItemSubsettingAuthorityGap,
        PendingSysmlRule::ProducerClosure,
    ] {
        assert!(
            answer.pending.contains(&(id(3), rule)),
            "{:?}",
            answer.pending
        );
    }
    assert!(matches!(
        snapshot
            .model()
            .property_state(id(3), agq_sysml::properties::USAGE_MAY_TIME_VARY),
        Ok(derived::PropertyState::NotComputed)
    ));
    assert_eq!(
        queries.direct_usage_types(id(3)).completeness(),
        Completeness::Complete
    );
}

pub(crate) fn library_fixture(class: MetaclassId, duplicated: bool) -> Snapshot {
    let mut f = Fixture::new();
    f.origin = DeclaredOrigin::StandardLibrary {
        library: SystemsLibraryIdentity::LIBRARY,
    };
    f.create(1, kc::PACKAGE, "root");
    f.create(2, kc::LIBRARY_PACKAGE, "Parts");
    f.create(3, class, "Part");
    f.member(1, 2, 102, kc::OWNING_MEMBERSHIP);
    f.member(2, 3, 103, kc::OWNING_MEMBERSHIP);
    if duplicated {
        f.create(4, class, "Part");
        f.member(2, 4, 104, kc::OWNING_MEMBERSHIP);
    }
    f.finish()
}
fn validate_part(snapshot: &Snapshot) -> Result<StandardSysmlBindings, SysmlBindingError> {
    let queries = KerMlQueries::new(
        SemanticContext::for_snapshot(
            snapshot,
            SemanticOptions {
                baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
                exclude_implied: true,
            },
            BTreeSet::new(),
        )
        .unwrap(),
    );
    StandardSysmlBindings::validate(
        snapshot.model(),
        &queries,
        SystemsLibraryIdentity::pinned([7; 32]),
        &[id(1)],
        [StandardSysmlRole::Part],
    )
}

#[test]
fn bindings_require_unique_public_path_exact_metaclass_and_source_library() {
    let good = library_fixture(sc::PART_DEFINITION, false);
    assert_eq!(
        validate_part(&good).unwrap().get(StandardSysmlRole::Part),
        Some(id(3))
    );
    assert!(matches!(
        validate_part(&library_fixture(sc::ITEM_DEFINITION, false)),
        Err(SysmlBindingError::WrongMetaclass(..))
    ));
    assert!(matches!(
        validate_part(&library_fixture(sc::PART_DEFINITION, true)),
        Err(SysmlBindingError::Ambiguous(..))
    ));
    let mut changes = good.change_set();
    changes.set(
        id(3),
        kp::ELEMENT_DECLARED_NAME,
        SlotValue::Scalar(Value::String("Part".into())),
        origin(),
    );
    assert!(matches!(
        validate_part(&good.apply(&changes).unwrap()),
        Err(SysmlBindingError::WrongLibrary(..))
    ));
    let mut changes = good.change_set();
    let property = good
        .model()
        .registry()
        .property(kp::MEMBERSHIP_VISIBILITY)
        .unwrap();
    let ValueKind::Enumeration(domain) = property.value_kind else {
        panic!("visibility enum")
    };
    let literal = *good
        .model()
        .registry()
        .enumeration(domain)
        .unwrap()
        .literals
        .iter()
        .find(|(_, name)| name.as_str() == "private")
        .unwrap()
        .0;
    changes.set(
        id(103),
        kp::MEMBERSHIP_VISIBILITY,
        SlotValue::Scalar(Value::Enumeration(literal)),
        DeclaredOrigin::StandardLibrary {
            library: SystemsLibraryIdentity::LIBRARY,
        },
    );
    assert!(matches!(
        validate_part(&good.apply(&changes).unwrap()),
        Err(SysmlBindingError::Inaccessible(..))
    ));
}

#[test]
fn unbound_systems_identity_cannot_change_the_pinned_kpar() {
    let mut identity = SystemsLibraryIdentity::pinned([0; 32]);
    identity.artifact_sha256.push('x');
    let bindings = StandardSysmlBindings::unbound(identity);
    assert_eq!(
        SysmlDependencyContract::checked_in(&bindings).unwrap_err(),
        SysmlContextError::IdentityMismatch("Systems Library pin")
    );
}

#[test]
fn actual_base_edge_discharges_only_its_specific_implication() {
    for with_base in [false, true] {
        let mut f = Fixture::new();
        f.origin = DeclaredOrigin::StandardLibrary {
            library: SystemsLibraryIdentity::LIBRARY,
        };
        f.create(1, kc::PACKAGE, "root");
        f.create(2, kc::LIBRARY_PACKAGE, "Parts");
        f.create(3, sc::PART_DEFINITION, "Part");
        f.member(1, 2, 102, kc::OWNING_MEMBERSHIP);
        f.member(2, 3, 103, kc::OWNING_MEMBERSHIP);
        f.origin = origin();
        f.create(4, sc::PART_DEFINITION, "AuthoredVehicle");
        if with_base {
            f.relation(
                4,
                3,
                201,
                kc::SUBCLASSIFICATION,
                kp::SUBCLASSIFICATION_SUPERCLASSIFIER,
            );
        }
        let snapshot = f.finish();
        let mut context = crate::context::fixture_context(&snapshot, BTreeSet::new());
        let kerml = KerMlQueries::new(context.kerml.fork());
        let bindings = StandardSysmlBindings::validate(
            snapshot.model(),
            &kerml,
            SystemsLibraryIdentity::pinned([8; 32]),
            &[id(1)],
            [StandardSysmlRole::Part],
        )
        .unwrap();
        context.id.dependencies = SysmlDependencyContract::checked_in(&bindings).unwrap();
        context.bindings = bindings;
        let answer = SysmlQueries::new(context).effective_usages(id(4));
        assert_eq!(
            answer.pending.contains(&(
                id(4),
                PendingSysmlRule::StandardGeneralization(StandardSysmlRole::Part)
            )),
            !with_base
        );
        assert!(
            answer
                .pending
                .contains(&(id(4), PendingSysmlRule::ProducerClosure))
        );
        assert_eq!(answer.completeness(), Completeness::Incomplete);
    }
}

#[test]
fn explicit_variant_names_are_independent_of_structural_typing_closure() {
    let mut f = Fixture::new();
    f.create(1, sc::PART_DEFINITION, "Choices");
    f.value(
        1,
        agq_sysml::properties::DEFINITION_IS_VARIATION,
        Value::Boolean(true),
    );
    f.create(2, sc::PART_USAGE, "aVariant");
    f.member(1, 2, 102, sc::VARIANT_MEMBERSHIP);
    let snapshot = f.finish();
    let query = q(&snapshot);
    let current = query.current_names(id(2));
    assert_eq!(current.completeness(), Completeness::Complete);
    assert_eq!(
        current.value(),
        &agq_kerml_semantics::EffectiveNames::Determinate(BTreeSet::from(["aVariant".into()]))
    );
    let answer = query.effective_names(id(2));
    assert_eq!(answer.completeness(), Completeness::Incomplete);
    assert!(
        !answer
            .pending
            .contains(&(id(2), PendingSysmlRule::Variation))
    );
    assert!(
        answer
            .pending
            .contains(&(id(2), PendingSysmlRule::ProducerClosure))
    );
    // A known name cannot hide the missing canonical variant typing.
    assert!(
        query
            .effective_usage_types(id(2))
            .pending
            .contains(&(id(2), PendingSysmlRule::Variation))
    );
    assert_eq!(
        query.current_names(id(1)).completeness(),
        Completeness::Complete
    );
    assert!(answer.kerml.positive_dependencies.contains(
        &agq_kernel::provenance::FactKey::Property {
            element: id(2),
            property: kp::ELEMENT_DECLARED_NAME,
        }
    ));
}

#[test]
fn distinct_query_model_is_rejected_before_binding() {
    let first = library_fixture(sc::PART_DEFINITION, false);
    let second = library_fixture(sc::PART_DEFINITION, false);
    let queries = q(&first);
    let error = StandardSysmlBindings::validate(
        second.model(),
        queries.kerml(),
        SystemsLibraryIdentity::pinned([0; 32]),
        &[id(1)],
        [StandardSysmlRole::Part],
    )
    .unwrap_err();
    assert_eq!(error, SysmlBindingError::QueryModelMismatch);
}

#[test]
fn binding_uniqueness_requires_closed_path_populations_on_validation_and_attachment() {
    let mut f = Fixture::new();
    f.origin = DeclaredOrigin::StandardLibrary {
        library: SystemsLibraryIdentity::LIBRARY,
    };
    f.create(1, kc::PACKAGE, "root");
    f.create(2, kc::LIBRARY_PACKAGE, "Parts");
    f.create(3, sc::PART_DEFINITION, "Part");
    f.create(4, kc::PACKAGE, "currentlyEmptyRoot");
    f.create(5, kc::PACKAGE, "unsearchedRoot");
    f.member(1, 2, 102, kc::OWNING_MEMBERSHIP);
    f.member(2, 3, 103, kc::OWNING_MEMBERSHIP);
    let snapshot = f.finish();
    let queries = |pending| {
        KerMlQueries::new(
            SemanticContext::for_project_snapshot(
                &snapshot,
                SemanticOptions {
                    baseline_profile: agq_kerml::BaselineProfile::OPERATIONAL_V9,
                    exclude_implied: true,
                },
                BTreeSet::new(),
                BTreeSet::new(),
                pending,
            )
            .unwrap(),
        )
    };
    let validate = |queries: &KerMlQueries<'_>| {
        StandardSysmlBindings::validate(
            snapshot.model(),
            queries,
            SystemsLibraryIdentity::pinned([7; 32]),
            &[id(1), id(4)],
            [StandardSysmlRole::Part],
        )
    };
    let closed = queries(BTreeSet::new());
    let bindings = validate(&closed).unwrap();
    assert_eq!(bindings.get(StandardSysmlRole::Part), Some(id(3)));
    assert!(bindings.valid_for(closed.context()));
    for scope in [id(1), id(2), id(4)] {
        let pending = queries(BTreeSet::from([scope]));
        // The underlying graph is identical, but the declaration-population
        // contract no longer establishes a unique owned qualified path.
        assert_eq!(
            pending.context().model_digest,
            closed.context().model_digest
        );
        assert_eq!(
            validate(&pending),
            Err(SysmlBindingError::Incomplete(StandardSysmlRole::Part)),
            "pending path scope {scope}"
        );
        assert!(!bindings.valid_for(pending.context()));
    }
    // Pending members of the terminal Part definition and an unsearched root
    // cannot add another declaration along the already complete Parts::Part path.
    let unrelated = queries(BTreeSet::from([id(3), id(5)]));
    assert!(validate(&unrelated).is_ok());
    assert!(bindings.valid_for(unrelated.context()));
}
