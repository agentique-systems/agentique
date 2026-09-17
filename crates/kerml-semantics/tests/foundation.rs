use agq_kerml::{classes as c, properties as p, views};
use agq_kerml_semantics::*;
use agq_kernel::provenance::Explanation;
use agq_kernel::{derived::*, metamodel::*, provenance::*, value::*, *};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[test]
fn working_reference_scopes_are_semantic_inputs_and_block_lexical_guesses() {
    let mut f = Fixture::new();
    f.create(A, c::TYPE);
    f.feature(id(30), A, FA);
    let snapshot = f.finish();
    let path = QualifiedName {
        absolute: false,
        segments: vec!["missing".into()],
    };
    let regular = queries(&snapshot).resolve_reference(FA, &path, c::TYPE);
    assert_eq!(regular.value, Resolution::Unresolved);
    let working = KerMlQueries::new(
        SemanticContext::for_working_snapshot(
            &snapshot,
            SemanticOptions::default(),
            BTreeSet::new(),
            BTreeSet::from([A]),
        )
        .unwrap(),
    );
    let result = working.resolve_reference(FA, &path, c::TYPE);
    assert_eq!(result.value, Resolution::Incomplete);
    assert_eq!(result.completeness, Completeness::Incomplete);
    assert_ne!(result.context, regular.context);
    assert_eq!(result.context.model_digest, regular.context.model_digest);
    assert!(matches!(
        SemanticContext::for_working_snapshot(
            &snapshot,
            SemanticOptions::default(),
            BTreeSet::new(),
            BTreeSet::from([id(999)])
        ),
        Err(ContextError::InvalidPendingScope(_))
    ));
}

#[test]
fn qualified_resolution_filters_private_members_and_retains_proofs() {
    let mut f = Fixture::new();
    f.create(A, c::NAMESPACE);
    f.create(B, c::NAMESPACE);
    f.create(FA, c::FEATURE);
    f.create(FB, c::FEATURE);
    f.create(FC, c::FEATURE);
    for (membership, owner, target) in [
        (id(30), A, B),
        (id(31), B, FA),
        (id(32), A, FB),
        (id(33), B, FC),
    ] {
        f.create(membership, c::OWNING_MEMBERSHIP);
        f.link(owner, p::ELEMENT_OWNED_RELATIONSHIP, membership);
        f.link(membership, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, target);
    }
    for (element, name) in [(B, "N"), (FA, "hidden")] {
        f.change.set(
            element,
            p::ELEMENT_DECLARED_NAME,
            SlotValue::Scalar(Value::String(name.into())),
            origin(),
        );
    }
    let registry = f.base.model().registry();
    let ValueKind::Enumeration(domain) = registry
        .property(p::MEMBERSHIP_VISIBILITY)
        .unwrap()
        .value_kind
    else {
        unreachable!()
    };
    let private = *registry
        .enumeration(domain)
        .unwrap()
        .literals
        .iter()
        .find(|(_, name)| name.as_str() == "private")
        .unwrap()
        .0;
    f.change.set(
        id(31),
        p::MEMBERSHIP_VISIBILITY,
        SlotValue::Scalar(Value::Enumeration(private)),
        origin(),
    );
    let snapshot = f.finish();
    let q = queries(&snapshot);
    let outside = q.resolve_reference(
        FB,
        &QualifiedName {
            absolute: false,
            segments: vec!["N".into(), "hidden".into()],
        },
        c::TYPE,
    );
    assert_eq!(outside.value, Resolution::Unresolved);
    let inside = q.resolve_reference(
        FC,
        &QualifiedName {
            absolute: false,
            segments: vec!["hidden".into()],
        },
        c::TYPE,
    );
    assert_eq!(inside.value, Resolution::Resolved(FA));
    assert!(
        inside
            .explanations
            .keys()
            .any(|c| c.query == QueryKind::ResolveReference)
    );
    assert_proofs(&inside);
    assert_proofs(&outside);
}

const A: ElementId = ElementId::from_u128(1);
const B: ElementId = ElementId::from_u128(2);
const C: ElementId = ElementId::from_u128(3);
const D: ElementId = ElementId::from_u128(4);
const FA: ElementId = ElementId::from_u128(11);
const FB: ElementId = ElementId::from_u128(12);
const FC: ElementId = ElementId::from_u128(13);
fn id(n: u128) -> ElementId {
    ElementId::from_u128(n)
}
fn origin() -> DeclaredOrigin {
    DeclaredOrigin::Authored { source: None }
}
fn reference(id: ElementId) -> SlotValue {
    SlotValue::Scalar(Value::Reference(id))
}
fn defaults(registry: &MetamodelRegistry, class: MetaclassId) -> Vec<(PropertyId, SlotValue)> {
    registry
        .effective_properties(class)
        .unwrap()
        .filter(|p| !p.derived && p.multiplicity.lower > 0)
        .filter_map(|p| {
            let value = match p.value_kind {
                ValueKind::Boolean => Value::Boolean(false),
                ValueKind::String => Value::String("fixture".into()),
                ValueKind::Enumeration(domain) => Value::Enumeration(
                    *registry
                        .enumeration(domain)
                        .unwrap()
                        .literals
                        .iter()
                        .find(|(_, name)| *name == "public")
                        .unwrap()
                        .0,
                ),
                ValueKind::Reference(_) => return None,
                _ => panic!("unhandled required fixture domain"),
            };
            Some((p.id, SlotValue::Scalar(value)))
        })
        .collect()
}
struct Fixture {
    base: Snapshot,
    change: ChangeSet,
    links: BTreeMap<(ElementId, PropertyId), Vec<Value>>,
}
impl Fixture {
    fn new() -> Self {
        let base = Snapshot::new(Arc::new(agq_kerml::registry().unwrap()));
        let change = base.change_set();
        Self {
            base,
            change,
            links: BTreeMap::new(),
        }
    }
    fn create(&mut self, id: ElementId, class: MetaclassId) {
        self.change.create(id, class, origin());
        for (p, value) in defaults(self.base.model().registry(), class) {
            self.change.set(id, p, value, origin());
        }
        self.change.set(
            id,
            p::ELEMENT_ELEMENT_ID,
            SlotValue::Scalar(Value::String(id.to_string())),
            origin(),
        );
    }
    fn link(&mut self, source: ElementId, property: PropertyId, target: ElementId) {
        self.links
            .entry((source, property))
            .or_default()
            .push(Value::Reference(target));
    }
    fn relation(
        &mut self,
        id: ElementId,
        class: MetaclassId,
        source: ElementId,
        target: ElementId,
    ) {
        self.create(id, class);
        let registry = self.base.model().registry();
        let specific = registry
            .resolve_property(class, p::SPECIALIZATION_SPECIFIC)
            .unwrap()
            .unwrap()
            .id;
        let general = registry
            .resolve_property(class, p::SPECIALIZATION_GENERAL)
            .unwrap()
            .unwrap()
            .id;
        self.change.set(id, specific, reference(source), origin());
        self.change.set(id, general, reference(target), origin());
        self.link(source, p::ELEMENT_OWNED_RELATIONSHIP, id);
    }
    fn feature(&mut self, membership: ElementId, owner: ElementId, feature: ElementId) {
        self.create(feature, c::FEATURE);
        self.create(membership, c::FEATURE_MEMBERSHIP);
        self.link(owner, p::ELEMENT_OWNED_RELATIONSHIP, membership);
        self.link(membership, p::RELATIONSHIP_OWNED_RELATED_ELEMENT, feature);
    }
    fn finish(mut self) -> Snapshot {
        for ((id, p), values) in self.links {
            self.change.set(id, p, SlotValue::Ordered(values), origin());
        }
        self.base.apply(&self.change).unwrap()
    }
}
fn queries(snapshot: &Snapshot) -> KerMlQueries<'_> {
    KerMlQueries::new(
        SemanticContext::for_snapshot(snapshot, SemanticOptions::default(), BTreeSet::new())
            .unwrap(),
    )
}
fn diamond() -> Fixture {
    let mut f = Fixture::new();
    for ty in [A, B, C, D] {
        f.create(ty, c::TYPE);
    }
    for (r, s, g) in [(20, B, A), (21, C, A), (22, D, B), (23, D, C)] {
        f.relation(id(r), c::SPECIALIZATION, s, g);
    }
    f
}
fn assert_proofs<T>(r: &QueryResult<T>) {
    for proofs in r.explanations.values() {
        for proof in proofs {
            for premise in &proof.premises {
                match premise {
                    Evidence::Fact(f) => {
                        assert!(r.fact_origins.contains_key(f), "missing fact {f:?}")
                    }
                    Evidence::Conclusion(c) => {
                        assert!(r.explanations.contains_key(c), "missing conclusion {c:?}")
                    }
                    Evidence::Search(s) => {
                        assert!(r.search_dependencies.contains(s), "missing search {s:?}")
                    }
                }
            }
        }
    }
}

#[test]
fn chains_diamonds_alternative_proofs_and_determinism() {
    let snapshot = diamond().finish();
    let q = queries(&snapshot);
    let r = q.all_specializations(D);
    assert_eq!(r.value, vec![A, B, C]);
    assert_eq!(r.completeness, Completeness::Complete);
    let proofs = &r.explanations[&Conclusion {
        query: QueryKind::AllSpecializations,
        subject: D,
        value: A,
    }];
    assert_eq!(proofs.len(), 2);
    assert!(
        proofs
            .iter()
            .all(|p| p.rule == Rule::TransitiveSpecialization)
    );
    assert_eq!(r, q.all_specializations(D));
    assert_proofs(&r);
}
#[test]
fn typed_upcasts_include_typing_subsetting_and_redefinition() {
    let mut f = Fixture::new();
    f.create(A, c::TYPE);
    f.create(FA, c::FEATURE);
    f.create(FB, c::FEATURE);
    f.create(FC, c::FEATURE);
    f.relation(id(20), c::FEATURE_TYPING, FA, A);
    f.relation(id(21), c::SUBSETTING, FB, FA);
    f.relation(id(22), c::REDEFINITION, FC, FB);
    let s = f.finish();
    let q = queries(&s);
    assert_eq!(q.direct_feature_types(FA).value, vec![A]);
    assert_eq!(q.direct_specializations(FA).value, vec![A]);
    assert_eq!(q.subsetted_features(FC).value, vec![FB]);
    assert_eq!(q.redefined_features(FC).value, vec![FB]);
    assert_eq!(q.all_specializations(FC).value, vec![A, FA, FB]);
    let r = q.direct_specializations(FC);
    assert!(r.positive_dependencies.contains(&FactKey::Property {
        element: id(22),
        property: p::REDEFINITION_REDEFINED_FEATURE
    }));
    assert_proofs(&r);
}
#[test]
fn ownership_membership_order_and_no_copying() {
    let mut f = diamond();
    f.feature(id(30), A, FA);
    f.feature(id(31), B, FB);
    let s = f.finish();
    let before: Vec<_> = s.model().elements().cloned().collect();
    let q = queries(&s);
    assert_eq!(q.owner(FA).value, Some(A));
    assert_eq!(q.owning_relationship(FA).value, Some(id(30)));
    assert_eq!(q.owning_related_element(id(30)).value, Some(A));
    assert_eq!(q.memberships(A).value, vec![id(30)]);
    assert_eq!(q.member(id(30)).value, Some(FA));
    assert_eq!(q.direct_features(D).value, vec![]);
    let r = q.effective_features(D);
    assert_eq!(r.value, vec![FA, FB]);
    assert_eq!(r.completeness, Completeness::Complete);
    assert_proofs(&r);
    assert_eq!(
        r.explanations[&Conclusion {
            query: QueryKind::InheritedFeature,
            subject: D,
            value: FA
        }]
            .len(),
        2
    );
    assert_eq!(s.model().elements().cloned().collect::<Vec<_>>(), before);
    assert_eq!(
        views::Feature::try_new(FA, s.model())
            .unwrap()
            .owning_relationship()
            .unwrap(),
        Some(id(30))
    );
    assert!(
        s.model()
            .element(FA)
            .unwrap()
            .slot(p::ELEMENT_OWNING_RELATIONSHIP)
            .is_none()
    );
}
#[test]
fn redefinitions_on_intermediate_types_and_diamonds_suppress_transitively() {
    let mut f = diamond();
    f.feature(id(30), A, FA);
    f.feature(id(31), B, FB);
    f.feature(id(32), D, FC);
    f.relation(id(40), c::REDEFINITION, FB, FA);
    f.relation(id(41), c::REDEFINITION, FC, FB);
    let s = f.finish();
    let q = queries(&s);
    let r = q.effective_features(D);
    assert_eq!(r.value, vec![FC]);
    assert_eq!(r.completeness, Completeness::Complete);
    assert!(
        r.explanations
            .keys()
            .any(|c| c.query == QueryKind::SuppressedFeature)
    );
    assert_proofs(&r);
}
#[test]
fn sibling_redefinitions_of_a_common_base_follow_second_removal_condition() {
    let mut f = diamond();
    f.feature(id(30), A, FA);
    f.feature(id(31), B, FB);
    f.feature(id(32), D, FC);
    f.relation(id(40), c::REDEFINITION, FB, FA);
    f.relation(id(41), c::REDEFINITION, FC, FA);
    let s = f.finish();
    let r = queries(&s).effective_features(D);
    assert_eq!(r.value, vec![FC]);
    assert_proofs(&r);
}
#[test]
fn subsetting_does_not_remove_an_inherited_feature() {
    let mut f = diamond();
    f.feature(id(30), A, FA);
    f.feature(id(31), D, FB);
    f.relation(id(40), c::SUBSETTING, FB, FA);
    let s = f.finish();
    assert_eq!(queries(&s).effective_features(D).value, vec![FA, FB]);
}
#[test]
fn private_membership_is_direct_but_not_inherited() {
    let mut f = diamond();
    f.feature(id(30), A, FA);
    let ValueKind::Enumeration(domain) = f
        .base
        .model()
        .registry()
        .property(p::MEMBERSHIP_VISIBILITY)
        .unwrap()
        .value_kind
    else {
        panic!()
    };
    let literal = *f
        .base
        .model()
        .registry()
        .enumeration(domain)
        .unwrap()
        .literals
        .iter()
        .find(|(_, n)| *n == "private")
        .unwrap()
        .0;
    f.change.set(
        id(30),
        p::MEMBERSHIP_VISIBILITY,
        SlotValue::Scalar(Value::Enumeration(literal)),
        origin(),
    );
    let s = f.finish();
    let q = queries(&s);
    assert_eq!(q.direct_features(A).value, vec![FA]);
    assert!(q.effective_features(D).value.is_empty());
}
#[test]
fn specialization_cycles_are_finite_but_cyclic_inheritance_is_explicitly_incomplete() {
    let mut f = diamond();
    f.relation(id(25), c::SPECIALIZATION, A, B);
    let s = f.finish();
    let q = queries(&s);
    let r = q.all_specializations(D);
    assert_eq!(r.value, vec![A, B, C]);
    assert_eq!(r.completeness, Completeness::Complete);
    assert_proofs(&r);
    assert_eq!(
        q.effective_features(D).completeness,
        Completeness::Incomplete
    );
}
#[test]
fn ownership_cycles_are_invalid_even_beyond_the_query_root() {
    let mut f = Fixture::new();
    for t in [A, B, C] {
        f.create(t, c::TYPE);
    }
    for (r, owner, member) in [(30, A, B), (31, B, A), (32, B, C)] {
        f.create(id(r), c::OWNING_MEMBERSHIP);
        f.link(owner, p::ELEMENT_OWNED_RELATIONSHIP, id(r));
        f.link(id(r), p::RELATIONSHIP_OWNED_RELATED_ELEMENT, member);
    }
    let s = f.finish();
    let r = queries(&s).owner(C);
    assert_eq!(r.value, None);
    assert_eq!(r.completeness, Completeness::Invalid);
    assert!(r.diagnostics.iter().any(|d| d.code == "KQ_OWNERSHIP_CYCLE"));
}
#[test]
fn malformed_memberships_and_bad_query_subjects_are_not_empty_successes() {
    let mut f = Fixture::new();
    f.create(A, c::TYPE);
    f.create(id(30), c::FEATURE_MEMBERSHIP);
    f.link(A, p::ELEMENT_OWNED_RELATIONSHIP, id(30));
    let s = f.finish();
    let q = queries(&s);
    let r = q.direct_features(A);
    assert_eq!(r.completeness, Completeness::Invalid);
    assert!(r.value.is_empty());
    assert_eq!(
        q.direct_features(id(999)).completeness,
        Completeness::Invalid
    );
    assert_eq!(
        q.direct_feature_types(A).completeness,
        Completeness::Invalid
    );
    assert!(
        r.search_dependencies
            .contains(&SearchDependency::PropertySet {
                element: id(30),
                property: p::RELATIONSHIP_OWNED_RELATED_ELEMENT
            })
    );
}
#[test]
fn empty_lookup_depends_on_namespace_and_nonmatching_names() {
    let mut f = Fixture::new();
    f.create(A, c::TYPE);
    f.feature(id(30), A, FA);
    f.change.set(
        FA,
        p::ELEMENT_DECLARED_NAME,
        SlotValue::Scalar(Value::String("before".into())),
        origin(),
    );
    let s = f.finish();
    let r = queries(&s).lookup_declared_member(A, "after");
    assert_eq!(r.completeness, Completeness::Complete);
    assert!(r.value.is_empty());
    assert!(
        r.search_dependencies
            .contains(&SearchDependency::NamespaceMembers { namespace: A })
    );
    assert!(r.positive_dependencies.contains(&FactKey::Property {
        element: FA,
        property: p::ELEMENT_DECLARED_NAME
    }));
    let mut change = s.change_set();
    change.set(
        FA,
        p::ELEMENT_DECLARED_NAME,
        SlotValue::Scalar(Value::String("after".into())),
        origin(),
    );
    let next = s.apply(&change).unwrap();
    assert_eq!(
        queries(&next).lookup_declared_member(A, "after").value,
        vec![FA]
    );
    assert!(
        queries(&s)
            .lookup_declared_member(A, "after")
            .value
            .is_empty()
    );
}
#[test]
fn negative_incoming_dependency_catches_creation_and_retargeting() {
    let s = diamond().finish();
    let old = queries(&s).direct_specializations(A);
    assert!(old.value.is_empty());
    assert!(
        old.search_dependencies
            .contains(&SearchDependency::Incoming { target: A })
    );
    let mut change = s.change_set();
    change.set(id(20), p::SPECIALIZATION_SPECIFIC, reference(A), origin());
    let next = s.apply(&change).unwrap();
    assert_eq!(queries(&next).direct_specializations(A).value, vec![A]);
    assert_ne!(queries(&next).context(), queries(&s).context());
}
#[test]
fn context_binds_options_library_content_and_overlay_content() {
    let s = diamond().finish();
    let context =
        SemanticContext::for_snapshot(&s, SemanticOptions::default(), BTreeSet::new()).unwrap();
    let options = SemanticContext::for_snapshot(
        &s,
        SemanticOptions {
            exclude_implied: true,
        },
        BTreeSet::new(),
    )
    .unwrap();
    let pins = SemanticContext::for_snapshot(
        &s,
        SemanticOptions::default(),
        BTreeSet::from([LibraryPin {
            name: "test".into(),
            sha256: [1; 32],
        }]),
    )
    .unwrap();
    assert_ne!(context.id(), options.id());
    assert_ne!(context.id(), pins.id());
    let make = |value| {
        let mut b = DerivationBuilder::new(s.clone());
        b.property(
            A,
            p::TYPE_IS_CONJUGATED,
            SlotValue::Scalar(Value::Boolean(value)),
            Explanation {
                rule: RuleId::from_u128(99),
                dependencies: BTreeSet::from([Dependency::Declared(FactKey::Element(A))]),
            },
        );
        b.build().unwrap()
    };
    let a = make(false);
    let b = make(true);
    let ca = SemanticContext::for_overlay(&a, SemanticOptions::default(), BTreeSet::new()).unwrap();
    let cb = SemanticContext::for_overlay(&b, SemanticOptions::default(), BTreeSet::new()).unwrap();
    assert_eq!(ca.id().revision, cb.id().revision);
    assert_ne!(ca.id(), cb.id());
}
#[test]
fn implied_specializations_are_an_answer_affecting_option() {
    let mut f = diamond();
    f.change.set(
        id(22),
        p::RELATIONSHIP_IS_IMPLIED,
        SlotValue::Scalar(Value::Boolean(true)),
        origin(),
    );
    let s = f.finish();
    let q = KerMlQueries::new(
        SemanticContext::for_snapshot(
            &s,
            SemanticOptions {
                exclude_implied: true,
            },
            BTreeSet::new(),
        )
        .unwrap(),
    );
    assert_eq!(q.direct_specializations(D).value, vec![C]);
    assert_eq!(queries(&s).direct_specializations(D).value, vec![B, C]);
}
#[test]
fn large_sparse_graph_keeps_linear_proof_size_and_does_not_recurse() {
    const N: u128 = 1500;
    let mut f = Fixture::new();
    for n in 1..=N {
        f.create(id(n), c::TYPE);
    }
    for n in 2..=N {
        f.relation(id(10000 + n), c::SPECIALIZATION, id(n), id(n - 1));
    }
    f.feature(id(20001), id(1), id(20002));
    let s = f.finish();
    let q = queries(&s);
    let all = q.all_specializations(id(N));
    assert_eq!(all.value.len(), N as usize - 1);
    assert!(all.explanations.len() < N as usize * 3);
    let effective = q.effective_features(id(N));
    assert_eq!(effective.value, vec![id(20002)]);
    assert_eq!(effective.completeness, Completeness::Complete);
    assert!(effective.explanations.len() < N as usize * 6);
    assert_proofs(&effective);
}

#[test]
fn association_inverse_is_unique_atomic_and_rebuilt_after_edits() {
    let mut f = Fixture::new();
    f.create(A, c::TYPE);
    f.create(B, c::TYPE);
    f.feature(id(30), A, FA);
    let s = f.finish();
    let mut duplicate = s.change_set();
    duplicate.set(
        B,
        p::ELEMENT_OWNED_RELATIONSHIP,
        SlotValue::Ordered(vec![Value::Reference(id(30))]),
        origin(),
    );
    assert!(matches!(
        s.apply(&duplicate),
        Err(ModelError::InverseMultiplicity { .. })
    ));
    let mut inverse = s.change_set();
    inverse.set(
        id(30),
        p::RELATIONSHIP_OWNING_RELATED_ELEMENT,
        reference(B),
        origin(),
    );
    assert!(matches!(
        s.apply(&inverse),
        Err(ModelError::UnsupportedAssociationStorage(_))
    ));
    let mut moved = s.change_set();
    moved.clear(A, p::ELEMENT_OWNED_RELATIONSHIP);
    moved.set(
        B,
        p::ELEMENT_OWNED_RELATIONSHIP,
        SlotValue::Ordered(vec![Value::Reference(id(30))]),
        origin(),
    );
    let next = s.apply(&moved).unwrap();
    assert_eq!(queries(&s).owner(FA).value, Some(A));
    assert_eq!(queries(&next).owner(FA).value, Some(B));
    assert_eq!(
        views::FeatureMembership::try_new(id(30), next.model())
            .unwrap()
            .owning_related_element()
            .unwrap(),
        Some(B)
    );
    assert!(
        s.model()
            .element(id(30))
            .unwrap()
            .slot(p::RELATIONSHIP_OWNING_RELATED_ELEMENT)
            .is_none()
    );
}

#[test]
fn overlay_proofs_retain_recursive_kernel_evidence() {
    let mut f = Fixture::new();
    f.create(A, c::TYPE);
    f.create(B, c::TYPE);
    let s = f.finish();
    let key = DerivationKey {
        rule: RuleId::from_u128(70),
        subject: B,
        output: OutputKey::from_u128(1),
    };
    let mut properties = defaults(s.model().registry(), c::SPECIALIZATION);
    properties.extend([
        (p::SPECIALIZATION_SPECIFIC, reference(B)),
        (p::SPECIALIZATION_GENERAL, reference(A)),
    ]);
    let mut builder = DerivationBuilder::new(s);
    builder.element(
        key,
        c::SPECIALIZATION,
        properties,
        BTreeSet::from([Dependency::Declared(FactKey::Element(A))]),
    );
    let overlay = builder.build().unwrap();
    let q = KerMlQueries::new(
        SemanticContext::for_overlay(&overlay, SemanticOptions::default(), BTreeSet::new())
            .unwrap(),
    );
    let r = q.direct_specializations(B);
    assert_eq!(r.value, vec![A]);
    assert_proofs(&r);
    assert!(matches!(
        r.fact_origins[&FactKey::Element(key.element_id())],
        Origin::Derived(_)
    ));
    assert!(r.positive_dependencies.contains(&FactKey::Element(A)));
    assert!(r.positive_dependencies.contains(&FactKey::Element(B)));
}

#[test]
fn unsupported_conjugation_is_not_a_complete_empty_result() {
    let mut f = Fixture::new();
    f.create(A, c::TYPE);
    let s = f.finish();
    let mut builder = DerivationBuilder::new(s);
    builder.property(
        A,
        p::TYPE_IS_CONJUGATED,
        SlotValue::Scalar(Value::Boolean(true)),
        Explanation {
            rule: RuleId::from_u128(99),
            dependencies: BTreeSet::from([Dependency::Declared(FactKey::Element(A))]),
        },
    );
    let overlay = builder.build().unwrap();
    let q = KerMlQueries::new(
        SemanticContext::for_overlay(&overlay, SemanticOptions::default(), BTreeSet::new())
            .unwrap(),
    );
    let r = q.effective_features(A);
    assert_eq!(r.completeness, Completeness::Incomplete);
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == "KQ_UNSUPPORTED_INHERITANCE")
    );
}

#[test]
fn nonowning_memberships_and_empty_namespace_lookup() {
    let mut f = Fixture::new();
    f.create(A, c::TYPE);
    f.create(B, c::TYPE);
    f.create(id(30), c::MEMBERSHIP);
    f.change
        .set(id(30), p::MEMBERSHIP_MEMBER_ELEMENT, reference(B), origin());
    f.change.set(
        id(30),
        p::MEMBERSHIP_MEMBER_NAME,
        SlotValue::Scalar(Value::String("alias".into())),
        origin(),
    );
    f.link(A, p::ELEMENT_OWNED_RELATIONSHIP, id(30));
    let s = f.finish();
    let q = queries(&s);
    assert_eq!(q.lookup_declared_member(A, "alias").value, vec![B]);
    assert_eq!(q.owner(B).value, None);
    let empty = q.lookup_declared_member(B, "missing");
    assert!(empty.value.is_empty());
    assert_eq!(empty.completeness, Completeness::Complete);
    assert!(
        empty
            .search_dependencies
            .contains(&SearchDependency::NamespaceMembers { namespace: B })
    );
    assert_proofs(&q.lookup_declared_member(A, "alias"));
}

#[test]
fn metamodel_mutation_is_rejected_even_with_same_release_id() {
    let mut descriptors = agq_kerml::descriptors();
    descriptors.models[0].version.patch += 1;
    let s = Snapshot::new(Arc::new(
        MetamodelRegistry::from_descriptors(descriptors).unwrap(),
    ));
    assert!(matches!(
        SemanticContext::for_snapshot(&s, SemanticOptions::default(), BTreeSet::new()),
        Err(ContextError::UnsupportedMetamodel)
    ));
}

#[test]
fn adding_redefinition_invalidates_effective_feature_evidence() {
    let mut f = diamond();
    f.feature(id(30), A, FA);
    f.feature(id(31), D, FB);
    let s = f.finish();
    let old = queries(&s).effective_features(D);
    assert_eq!(old.value, vec![FA, FB]);
    assert!(
        old.search_dependencies
            .contains(&SearchDependency::Incoming { target: FB })
    );
    let mut change = s.change_set();
    change.create(id(40), c::REDEFINITION, origin());
    for (p, v) in defaults(s.model().registry(), c::REDEFINITION) {
        change.set(id(40), p, v, origin());
    }
    change.set(
        id(40),
        p::REDEFINITION_REDEFINING_FEATURE,
        reference(FB),
        origin(),
    );
    change.set(
        id(40),
        p::REDEFINITION_REDEFINED_FEATURE,
        reference(FA),
        origin(),
    );
    change.set(
        FB,
        p::ELEMENT_OWNED_RELATIONSHIP,
        SlotValue::Ordered(vec![Value::Reference(id(40))]),
        origin(),
    );
    let next = s.apply(&change).unwrap();
    assert_eq!(queries(&next).effective_features(D).value, vec![FB]);
    assert_eq!(queries(&s).effective_features(D), old);
}

#[test]
fn creation_order_does_not_change_content_identity_or_proofs() {
    let build = |reverse| {
        let mut f = Fixture::new();
        for ty in if reverse {
            vec![C, B, A]
        } else {
            vec![A, B, C]
        } {
            f.create(ty, c::TYPE);
        }
        for (r, s, g) in if reverse {
            vec![(21, C, B), (20, B, A)]
        } else {
            vec![(20, B, A), (21, C, B)]
        } {
            f.relation(id(r), c::SPECIALIZATION, s, g);
        }
        f.finish()
    };
    let a = build(false);
    let b = build(true);
    let qa = queries(&a);
    let qb = queries(&b);
    assert_ne!(qa.context().revision, qb.context().revision);
    assert_eq!(qa.context().model_digest, qb.context().model_digest);
    let ra = qa.all_specializations(C);
    let rb = qb.all_specializations(C);
    assert_eq!(ra.value, rb.value);
    assert_eq!(ra.explanations, rb.explanations);
    assert_eq!(ra.positive_dependencies, rb.positive_dependencies);
    assert_eq!(ra.search_dependencies, rb.search_dependencies);
}

#[test]
fn feature_aliases_do_not_silently_bypass_inheritance_filtering() {
    let mut f = Fixture::new();
    f.create(A, c::TYPE);
    f.create(FA, c::FEATURE);
    f.create(id(30), c::MEMBERSHIP);
    f.change.set(
        id(30),
        p::MEMBERSHIP_MEMBER_ELEMENT,
        reference(FA),
        origin(),
    );
    f.link(A, p::ELEMENT_OWNED_RELATIONSHIP, id(30));
    let s = f.finish();
    let r = queries(&s).effective_features(A);
    assert_eq!(r.completeness, Completeness::Incomplete);
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == "KQ_NONFEATURE_MEMBERSHIP")
    );
}
