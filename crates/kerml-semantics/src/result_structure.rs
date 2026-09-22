//! Canonical implied result structure. Raw result identities are never replaced.
use crate::*;
use agq_kerml::{BaselineProfile, classes as c, properties as p};
use agq_kernel::{
    DerivationKey, ElementId, MetaclassId, OutputKey, PropertyId, RuleId, Snapshot,
    derived::{
        DerivationBuilder, DerivationError, DerivedOverlay, StructuralSearch, StructuralSearchPool,
    },
    provenance::{Dependency, Explanation as KernelExplanation, ExplanationPool, FactKey, Origin},
    value::{SlotValue, Value},
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

mod contributions;

/// Publication closes positive structural implications before choosing nearest
/// featuring contexts. The second stratum still runs all ordinary producers on
/// newly generated subjects and rejects any conflicting context reassignment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResultStructureStratum {
    Structural,
    /// Extension predicates whose negative antecedents require structural
    /// closure. KerML contextual bindings remain deferred in this stratum.
    StableProperties,
    ContextualBindings,
}

/// Semantic reason an implied BindingConnector exists. Its rule identity is
/// provenance, not an authored annotation or an owner-metaclass heuristic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImpliedBindingRole {
    FeatureReferenceResult,
    ExpressionResult,
    FunctionResult,
    FeatureValue,
    Invocation,
}
impl ImpliedBindingRole {
    pub const ALL: [Self; 5] = [
        Self::FeatureReferenceResult,
        Self::ExpressionResult,
        Self::FunctionResult,
        Self::FeatureValue,
        Self::Invocation,
    ];
    pub const fn constraint(self) -> &'static str {
        match self {
            Self::FeatureReferenceResult => "checkFeatureReferenceExpressionBindingConnector",
            Self::ExpressionResult => "checkExpressionResultBindingConnector",
            Self::FunctionResult => "checkFunctionResultBindingConnector",
            Self::FeatureValue => "checkFeatureValueBindingConnector",
            Self::Invocation => "checkInvocationExpressionBehaviorBindingConnector",
        }
    }
    pub fn rule_id(self, profile: BaselineProfile) -> RuleId {
        rule_id(profile, self.constraint())
    }
}

/// The five independently reviewed KERML11-145 antecedents and graph obligations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResultDomainRule {
    FeatureValuation,
    ExpressionResult,
    FunctionResult,
    IndexResult,
    SelectResult,
    FeatureValueBinding,
}
impl ResultDomainRule {
    pub const ALL: [Self; 5] = [
        Self::FeatureValuation,
        Self::ExpressionResult,
        Self::FunctionResult,
        Self::IndexResult,
        Self::SelectResult,
    ];
    pub const fn constraint(self) -> &'static str {
        match self {
            Self::FeatureValuation => "checkFeatureValuationSpecialization",
            Self::ExpressionResult => "checkExpressionResultBindingConnector",
            Self::FunctionResult => "checkFunctionResultBindingConnector",
            Self::IndexResult => "checkIndexExpressionResultSpecialization",
            Self::SelectResult => "checkSelectExpressionResultSpecialization",
            Self::FeatureValueBinding => "checkFeatureValueBindingConnector",
        }
    }
}

fn identity(parts: &[&[u8]]) -> u128 {
    let mut hash = Sha256::new();
    hash.update(b"agq-kerml-structural-output/1");
    for part in parts {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part);
    }
    u128::from_be_bytes(hash.finalize()[..16].try_into().expect("128-bit prefix"))
}

#[cfg(test)]
mod navigation_evidence_regression {
    use crate as agq_kerml_semantics;
    include!("../tests/common/result_fixture.rs");
    use super::{DerivationError, Graph, ResultStructurePlan};

    #[test]
    fn extension_property_changes_invalidate_reference_targets_and_require_complete_evidence() {
        let mut f = Fixture::new();
        f.create(1, c::FEATURE);
        f.create(2, c::CLASSIFIER);
        let snapshot = f.finish();
        let q = KerMlQueries::new(
            SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
        );
        let mut evidence = q.all_supertypes(id(2)).map(|_| ());
        assert_eq!(evidence.completeness, Completeness::Complete);
        let mut plan = q.plan_result_structure([]);
        let value = SlotValue::Set(BTreeSet::from([Value::Reference(id(2))]));
        plan.add_derived_property(
            id(1),
            p::FEATURE_TYPE,
            value.clone(),
            RuleId::from_u128(5),
            &evidence,
        )
        .unwrap();
        assert_eq!(plan.changed_population(), BTreeSet::from([id(1), id(2)]));

        evidence.completeness = Completeness::Incomplete;
        let mut pending = q.plan_result_structure([]);
        pending
            .add_derived_property(
                id(1),
                p::FEATURE_TYPE,
                value,
                RuleId::from_u128(5),
                &evidence,
            )
            .unwrap();
        assert!(pending.changed_population().is_empty());
        assert_eq!(pending.production.completeness, Completeness::Incomplete);
    }

    #[test]
    fn planned_outputs_share_proofs_across_batches_and_reuse_existing_evidence() {
        let mut f = Fixture::new();
        f.create(1, c::FEATURE);
        let snapshot = f.finish();
        let q = KerMlQueries::new(
            SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
        );
        fn make<'m>(
            q: &KerMlQueries<'m>,
            role: &str,
        ) -> (ResultStructurePlan<'m>, ElementId, DerivationKey) {
            let mut plan = q.plan_result_structure([]);
            let key = plan.graph.key("owned-cross-domain/1", id(1), role, &[]);
            let output = plan.graph.create(key, c::FEATURE, &BTreeSet::new());
            (plan, output, key)
        }
        let (mut first, a, key_a) = make(&q, "first");
        let (second, b, _) = make(&q, "second");
        first.merge(second).unwrap();
        let retained = first.graph.records[&a].explanation.clone();
        assert!(Arc::ptr_eq(&retained, &first.graph.records[&b].explanation));
        let result = first.materialize(&snapshot).unwrap();
        let Origin::Derived(stored) = result.overlay.model().element(a).unwrap().origin() else {
            panic!()
        };
        assert!(Arc::ptr_eq(&retained, stored));
        let q = KerMlQueries::new(
            SemanticContext::for_overlay(&result.overlay, Default::default(), BTreeSet::new())
                .unwrap(),
        );
        let (mut repeated, same_a, _) = make(&q, "first");
        assert_eq!(a, same_a);
        assert!(Arc::ptr_eq(stored, &repeated.graph.records[&a].explanation));
        repeated.materialize_on_overlay(&result.overlay).unwrap();

        // Evidence reuse never bypasses the class/rule/value collision checks.
        repeated = q.plan_result_structure([]);
        repeated
            .graph
            .create(key_a, c::CLASSIFIER, &BTreeSet::new());
        assert!(matches!(
            repeated.materialize_on_overlay(&result.overlay),
            Err(DerivationError::IdentityCollision(_))
        ));
    }

    #[test]
    fn independent_relationship_producers_merge_on_one_owner_without_reordering_chains() {
        let mut f = Fixture::new();
        f.create(1, c::FEATURE);
        f.create(2, c::FEATURE);
        f.create(3, c::CLASSIFIER);
        let snapshot = f.finish();
        let q = KerMlQueries::new(
            SemanticContext::for_snapshot(&snapshot, Default::default(), BTreeSet::new()).unwrap(),
        );
        let plan = |typing: bool, subsetting: bool| {
            let mut plan = q.plan_result_structure([]);
            let deps = BTreeSet::from([
                Dependency::Declared(FactKey::Element(id(1))),
                Dependency::Declared(FactKey::Element(id(2))),
                Dependency::Declared(FactKey::Element(id(3))),
            ]);
            if typing {
                plan.graph.cross_typing(id(1), id(1), id(3), &deps);
            }
            if subsetting {
                plan.graph
                    .subset(id(1), id(1), id(2), "owned-cross-domain/1", &deps);
            }
            plan
        };
        let together = plan(true, true).materialize(&snapshot).unwrap();
        for reverse in [false, true] {
            let mut separate = plan(reverse, !reverse);
            separate.merge(plan(!reverse, reverse)).unwrap();
            let merged = separate.materialize(&snapshot).unwrap();
            assert!(
                together
                    .overlay
                    .model()
                    .elements()
                    .eq(merged.overlay.model().elements())
            );
            assert!(together.overlay.facts().eq(merged.overlay.facts()));
            let owned = merged
                .overlay
                .model()
                .navigation_slot(id(1), p::ELEMENT_OWNED_RELATIONSHIP)
                .unwrap();
            assert_eq!(owned.value().values().count(), 2);
            assert!(
                snapshot
                    .model()
                    .navigation_slot(id(1), p::ELEMENT_OWNED_RELATIONSHIP)
                    .is_none()
            );
        }
    }

    #[test]
    fn projected_crossed_feature_dependencies_materialize_as_canonical_link_facts() {
        let profile = agq_kerml::BaselineProfile::OPERATIONAL_V7;
        let base = Snapshot::new(Arc::new(agq_kerml::registry_for_profile(profile).unwrap()));
        let mut f = Fixture {
            changes: base.change_set(),
            base,
            owned: BTreeMap::new(),
        };
        f.create(1, c::FEATURE);
        f.create(2, c::FEATURE);
        relation(
            &mut f,
            1,
            2,
            3,
            c::CROSS_SUBSETTING,
            p::CROSS_SUBSETTING_CROSSED_FEATURE,
        );
        let snapshot = f.finish();
        let projected = FactKey::Property {
            element: id(3),
            property: p::CROSS_SUBSETTING_CROSSED_FEATURE,
        };
        let q = KerMlQueries::new(
            SemanticContext::for_snapshot(
                &snapshot,
                SemanticOptions {
                    baseline_profile: profile,
                    ..Default::default()
                },
                Default::default(),
            )
            .unwrap(),
        );
        assert!(
            q.cross_feature(id(1))
                .positive_dependencies
                .contains(&projected)
        );
        assert!(
            snapshot
                .model()
                .element(id(3))
                .unwrap()
                .slot(p::CROSS_SUBSETTING_CROSSED_FEATURE)
                .is_none()
        );
        let Origin::AssociationOccurrences(links) = snapshot
            .model()
            .navigation_slot(id(3), p::CROSS_SUBSETTING_CROSSED_FEATURE)
            .unwrap()
            .origin()
        else {
            panic!("canonical association navigation");
        };
        let mut graph = Graph {
            model: snapshot.model(),
            profile,
            records: BTreeMap::new(),
            evidence_pool: ExplanationPool::default(),
            attachments: BTreeMap::new(),
            unattached_contextual: BTreeSet::new(),
            assignments: BTreeSet::new(),
            conflict: None,
            searches: BTreeMap::new(),
            search_pool: agq_kernel::derived::StructuralSearchPool::default(),
            direct_searches: BTreeSet::new(),
            touched_records: BTreeSet::new(),
            contributed_properties: BTreeMap::new(),
        };
        let mut proof = q.result(());
        q.fact(&mut proof, projected);
        let generated = graph.create(
            graph.key("evidence-regression", id(1), "feature", &[]),
            c::FEATURE,
            &proof.canonical_dependencies,
        );
        let overlay = graph.build(&snapshot).unwrap();
        let Origin::Derived(explanation) = overlay.model().element(generated).unwrap().origin()
        else {
            panic!("derived fact");
        };
        let mut expected: BTreeSet<_> = links
            .iter()
            .copied()
            .map(|link| Dependency::Declared(FactKey::AssociationOccurrence(link)))
            .collect();
        expected.insert(Dependency::Declared(FactKey::Element(id(1))));
        assert_eq!(explanation.dependencies, expected);
    }
}
fn rule_id(profile: BaselineProfile, rule: &str) -> RuleId {
    RuleId::from_u128(identity(&[
        profile.id().as_bytes(),
        &profile.errata_manifest_sha256().unwrap_or_default(),
        rule.as_bytes(),
        // Frozen v9 producer identity. Ordinary query-version changes must not
        // change the IDs of already reviewed contextual results and bindings.
        b"agq-kerml-query/16",
    ]))
}

pub(crate) fn structural_rule_profile(rule: RuleId) -> Option<BaselineProfile> {
    use BaselineProfile as P;
    static PROFILES: std::sync::OnceLock<BTreeMap<RuleId, BaselineProfile>> =
        std::sync::OnceLock::new();
    PROFILES
        .get_or_init(|| {
            [
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
            ]
            .into_iter()
            .flat_map(|profile| {
                ImpliedBindingRole::ALL
                    .into_iter()
                    .map(ImpliedBindingRole::constraint)
                    .chain(
                        ResultDomainRule::ALL
                            .into_iter()
                            .map(ResultDomainRule::constraint),
                    )
                    .chain([
                        "structural-result-ownership/1",
                        "initial-feature-value-context/1",
                        "owned-cross-domain/1",
                        "checkFeatureCrossingSpecialization",
                        "checkFeatureFeatureMembershipTypeFeaturing",
                        "checkFeatureParameterRedefinition",
                        "checkFeatureEndRedefinition",
                        "checkFeatureResultRedefinition",
                        "validateInstantiationExpressionResult",
                        "checkInvocationExpressionSpecialization",
                        "checkInvocationExpressionBehaviorResultSpecialization",
                        "checkFeatureChainExpressionResultSpecialization",
                        "checkFeatureChainExpressionTargetRedefinition",
                        "checkFeatureChainExpressionSourceTargetRedefinition",
                    ])
                    .map(move |name| (rule_id(profile, name), profile))
            })
            .collect()
        })
        .get(&rule)
        .copied()
}

/// A contextual result is a separate canonical Feature; the terminal is unchanged.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextualResult {
    pub feature: ElementId,
    pub expression: ElementId,
    pub raw_result: ElementId,
    pub rule: ResultDomainRule,
}

#[derive(PartialEq, Eq)]
struct ImpliedRecord {
    key: DerivationKey,
    class: MetaclassId,
    slots: BTreeMap<PropertyId, SlotValue>,
    explanation: Arc<KernelExplanation>,
}

struct Graph<'a> {
    model: &'a agq_kernel::ModelView,
    profile: BaselineProfile,
    records: BTreeMap<ElementId, ImpliedRecord>,
    evidence_pool: ExplanationPool,
    // New relationships on an existing owner are enumerated by their stable
    // semantic identity. Fresh chain ownership remains explicitly ordered in
    // its record; it never passes through this unordered contribution set.
    attachments: BTreeMap<ElementId, BTreeSet<ElementId>>,
    unattached_contextual: BTreeSet<ElementId>,
    assignments: BTreeSet<(ElementId, PropertyId)>,
    conflict: Option<FactKey>,
    searches: BTreeMap<ElementId, Arc<BTreeSet<StructuralSearch>>>,
    search_pool: StructuralSearchPool,
    direct_searches: BTreeSet<SearchDependency>,
    touched_records: BTreeSet<ElementId>,
    contributed_properties: BTreeMap<(ElementId, PropertyId), contributions::PropertyContribution>,
}
impl<'a> Graph<'a> {
    fn finish_subject(
        &mut self,
        production: &mut QueryResult<Vec<ElementId>>,
        aggregate: &mut QueryResult<Vec<ElementId>>,
    ) {
        production
            .search_dependencies
            .append(&mut self.direct_searches);
        let touched = std::mem::take(&mut self.touched_records);
        if !touched.is_empty() {
            let searches = self
                .search_pool
                .intern(crate::read_dependencies::structural_searches(production));
            for id in touched {
                self.merge_searches(id, searches.clone());
            }
        }
        aggregate.value.extend(production.value.iter().copied());
        let next = QueryResult::new(&production.context, vec![]);
        aggregate.merge(std::mem::replace(production, next));
    }
    fn merge_searches(&mut self, id: ElementId, searches: Arc<BTreeSet<StructuralSearch>>) {
        let merged = if let Some(previous) = self.searches.get(&id) {
            self.search_pool.union_shared(previous, searches)
        } else {
            self.search_pool.intern_shared(searches)
        };
        self.searches.insert(id, merged);
    }
    fn return_result(&mut self, expression: ElementId, deps: &BTreeSet<Dependency>) {
        let rule = "validateInstantiationExpressionResult";
        let result = self.create(
            self.key(rule, expression, "owned-result", &[]),
            c::FEATURE,
            deps,
        );
        let agq_kernel::metamodel::ValueKind::Enumeration(domain) = self
            .model
            .registry()
            .property(p::FEATURE_DIRECTION)
            .expect("direction")
            .value_kind
        else {
            unreachable!()
        };
        let literal = *self
            .model
            .registry()
            .enumeration(domain)
            .expect("direction domain")
            .literals
            .iter()
            .find(|(_, name)| name.as_str() == "out")
            .expect("output direction")
            .0;
        self.set(result, p::FEATURE_DIRECTION, Value::Enumeration(literal));
        self.membership(
            expression,
            result,
            c::RETURN_PARAMETER_MEMBERSHIP,
            self.key(rule, expression, "return-membership", &[]),
            deps,
        );
    }
    fn specialize(
        &mut self,
        source: ElementId,
        target: ElementId,
        rule: &str,
        deps: &BTreeSet<Dependency>,
    ) {
        let feature = self.records.get(&target).map_or_else(
            || {
                self.model.element(target).is_some_and(|r| {
                    self.model
                        .registry()
                        .is_subtype(r.metaclass(), c::FEATURE)
                        .unwrap_or(false)
                })
            },
            |r| {
                self.model
                    .registry()
                    .is_subtype(r.class, c::FEATURE)
                    .unwrap_or(false)
            },
        );
        let (class, endpoints) = if feature {
            (
                c::SUBSETTING,
                (
                    p::SUBSETTING_SUBSETTING_FEATURE,
                    p::SUBSETTING_SUBSETTED_FEATURE,
                ),
            )
        } else {
            (
                c::FEATURE_TYPING,
                (p::FEATURE_TYPING_TYPED_FEATURE, p::FEATURE_TYPING_TYPE),
            )
        };
        self.required_relationship(
            source,
            (source, target),
            class,
            endpoints,
            (rule, "specialization"),
            deps,
        );
    }
    fn expression_chain(
        &mut self,
        expression: ElementId,
        input: ElementId,
        source_target: Option<ElementId>,
        targets: (ElementId, ElementId),
        result: ElementId,
        deps: &BTreeSet<Dependency>,
    ) {
        let (actual, standard) = targets;
        let source_rule = "checkFeatureChainExpressionSourceTargetRedefinition";
        let source_target = source_target.unwrap_or_else(|| {
            let feature = self.create(
                self.key(source_rule, expression, "source-target", &[input]),
                c::FEATURE,
                deps,
            );
            self.membership(
                input,
                feature,
                c::FEATURE_MEMBERSHIP,
                self.key(
                    source_rule,
                    expression,
                    "source-target-membership",
                    &[input],
                ),
                deps,
            );
            feature
        });
        for (target, rule) in [
            (actual, source_rule),
            (standard, "checkFeatureChainExpressionTargetRedefinition"),
        ] {
            self.required_relationship(
                source_target,
                (source_target, target),
                c::REDEFINITION,
                (
                    p::REDEFINITION_REDEFINING_FEATURE,
                    p::REDEFINITION_REDEFINED_FEATURE,
                ),
                (rule, "target-redefinition"),
                deps,
            );
        }
        let rule = "checkFeatureChainExpressionResultSpecialization";
        let inputs = [input, source_target];
        let chain = self.create(
            self.key(rule, input, "result-chain", &inputs),
            c::FEATURE,
            deps,
        );
        for (role, target) in [("input", input), ("target", source_target)] {
            let chaining = self.create(
                self.key(rule, input, role, &inputs),
                c::FEATURE_CHAINING,
                deps,
            );
            self.set(
                chaining,
                p::FEATURE_CHAINING_CHAINING_FEATURE,
                Value::Reference(target),
            );
            self.own(chain, chaining);
        }
        self.unattached_contextual.insert(chain);
        self.subset(expression, result, chain, rule, deps);
    }
    fn crossing(
        &mut self,
        end: ElementId,
        first: ElementId,
        cross: ElementId,
        deps: &BTreeSet<Dependency>,
    ) -> ElementId {
        let rule = "checkFeatureCrossingSpecialization";
        let inputs = [first, cross];
        let relationship = self.create(
            self.key(rule, end, "cross-subsetting", &inputs),
            c::CROSS_SUBSETTING,
            deps,
        );
        // The ordered semantic endpoints determine chain identity. Neither the
        // caller's traversal order nor an element allocation counter participates.
        let chain = self.create(
            self.key(rule, first, "crossed-chain", &inputs),
            c::FEATURE,
            deps,
        );
        for (role, target) in [("first-chaining", first), ("second-chaining", cross)] {
            let chaining = self.create(
                self.key(rule, first, role, &inputs),
                c::FEATURE_CHAINING,
                deps,
            );
            self.set(
                chaining,
                p::FEATURE_CHAINING_CHAINING_FEATURE,
                Value::Reference(target),
            );
            self.own(chain, chaining);
        }
        self.set(
            relationship,
            p::CROSS_SUBSETTING_CROSSED_FEATURE,
            Value::Reference(chain),
        );
        self.set(
            relationship,
            p::CROSS_SUBSETTING_CROSSING_FEATURE,
            Value::Reference(end),
        );
        self.set_value(
            relationship,
            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(chain)]),
        );
        self.own(end, relationship);
        relationship
    }
    fn crossing_product(&mut self, ends: &[ElementId], deps: &BTreeSet<Dependency>) -> ElementId {
        let rule = "checkFeatureCrossingSpecialization";
        let mut first = ends[0];
        for i in 1..ends.len() {
            let inputs = &ends[..=i];
            let product = self.create(
                self.key(rule, ends[0], "end-product", inputs),
                c::FEATURE,
                deps,
            );
            let typing = self.create(
                self.key(rule, ends[0], "end-product-type", inputs),
                c::FEATURE_TYPING,
                deps,
            );
            self.set(
                typing,
                p::FEATURE_TYPING_TYPED_FEATURE,
                Value::Reference(product),
            );
            self.set(typing, p::FEATURE_TYPING_TYPE, Value::Reference(ends[i]));
            self.own(product, typing);
            let featuring = self.create(
                self.key(rule, ends[0], "end-product-domain", inputs),
                c::TYPE_FEATURING,
                deps,
            );
            self.set(
                featuring,
                p::TYPE_FEATURING_FEATURE_OF_TYPE,
                Value::Reference(product),
            );
            self.set(
                featuring,
                p::TYPE_FEATURING_FEATURING_TYPE,
                Value::Reference(first),
            );
            self.own(product, featuring);
            if i > 1 {
                self.membership(
                    product,
                    first,
                    c::OWNING_MEMBERSHIP,
                    self.key(rule, ends[0], "end-product-owner", inputs),
                    deps,
                );
            }
            first = product;
        }
        first
    }

    fn required_relationship(
        &mut self,
        subject: ElementId,
        participants: (ElementId, ElementId),
        class: MetaclassId,
        endpoints: (PropertyId, PropertyId),
        rule_role: (&str, &str),
        deps: &BTreeSet<Dependency>,
    ) -> ElementId {
        let (source, target) = participants;
        self.direct_searches.insert(SearchDependency::PropertySet {
            element: source,
            property: p::ELEMENT_OWNED_RELATIONSHIP,
        });
        // Required structure can already be authored. Preserve that relationship's
        // identity instead of adding a second realization of the same obligation.
        if let Some(owned) = self
            .model
            .navigation_slot(source, p::ELEMENT_OWNED_RELATIONSHIP)
        {
            for value in owned.value().values() {
                let Value::Reference(relationship) = value else {
                    continue;
                };
                self.direct_searches
                    .insert(SearchDependency::Element(*relationship));
                for property in [endpoints.0, endpoints.1] {
                    self.direct_searches.insert(SearchDependency::PropertySet {
                        element: *relationship,
                        property,
                    });
                }
                let Some(record) = self.model.element(*relationship) else {
                    continue;
                };
                let endpoint = |property| {
                    self.model
                        .navigation_slot(*relationship, property)
                        .and_then(|slot| {
                            slot.value().values().find_map(|value| match value {
                                Value::Reference(id) => Some(*id),
                                _ => None,
                            })
                        })
                };
                if record.metaclass() == class
                    && endpoint(endpoints.0) == Some(source)
                    && endpoint(endpoints.1) == Some(target)
                {
                    return *relationship;
                }
            }
        }
        let r = self.create(
            self.key(rule_role.0, subject, rule_role.1, &[source, target]),
            class,
            deps,
        );
        self.set(r, endpoints.0, Value::Reference(source));
        self.set(r, endpoints.1, Value::Reference(target));
        self.own(source, r);
        r
    }
    fn cross_featuring(
        &mut self,
        subject: ElementId,
        feature: ElementId,
        ty: ElementId,
        deps: &BTreeSet<Dependency>,
    ) {
        self.required_relationship(
            subject,
            (feature, ty),
            c::TYPE_FEATURING,
            (
                p::TYPE_FEATURING_FEATURE_OF_TYPE,
                p::TYPE_FEATURING_FEATURING_TYPE,
            ),
            ("owned-cross-domain/1", "featuring"),
            deps,
        );
    }
    fn variable_snapshot(
        &mut self,
        owner: ElementId,
        snapshots: ElementId,
        deps: &BTreeSet<Dependency>,
    ) -> ElementId {
        let rule = "checkFeatureFeatureMembershipTypeFeaturing";
        let domain = self.create(
            self.key(rule, owner, "snapshot-domain", &[snapshots]),
            c::FEATURE,
            deps,
        );
        self.membership(
            owner,
            domain,
            c::FEATURE_MEMBERSHIP,
            self.key(rule, owner, "snapshot-membership", &[snapshots]),
            deps,
        );
        self.required_relationship(
            owner,
            (domain, snapshots),
            c::REDEFINITION,
            (
                p::REDEFINITION_REDEFINING_FEATURE,
                p::REDEFINITION_REDEFINED_FEATURE,
            ),
            (rule, "snapshot-redefinition"),
            deps,
        );
        domain
    }

    fn variable_featuring(
        &mut self,
        feature: ElementId,
        domain: ElementId,
        deps: &BTreeSet<Dependency>,
    ) {
        self.required_relationship(
            feature,
            (feature, domain),
            c::TYPE_FEATURING,
            (
                p::TYPE_FEATURING_FEATURE_OF_TYPE,
                p::TYPE_FEATURING_FEATURING_TYPE,
            ),
            (
                "checkFeatureFeatureMembershipTypeFeaturing",
                "variable-featuring",
            ),
            deps,
        );
    }
    fn cross_typing(
        &mut self,
        subject: ElementId,
        feature: ElementId,
        ty: ElementId,
        deps: &BTreeSet<Dependency>,
    ) {
        self.required_relationship(
            subject,
            (feature, ty),
            c::FEATURE_TYPING,
            (p::FEATURE_TYPING_TYPED_FEATURE, p::FEATURE_TYPING_TYPE),
            ("owned-cross-domain/1", "typing"),
            deps,
        );
    }
    fn cross_domain(
        &mut self,
        cross: ElementId,
        domain: &OwnedCrossDomain,
        inherited_domains: &[ElementId],
        deps: &BTreeSet<Dependency>,
    ) -> Vec<ElementId> {
        if domain.factors.is_empty() {
            // A known empty population has no canonical opposite-end domain.
            // Do not fabricate factors to satisfy a validation-only cardinality
            // conflict. The conformance report retains that conflict separately.
            return vec![];
        }
        if let [factor] = domain.factors.as_slice() {
            for &ty in &factor.types {
                self.cross_featuring(cross, cross, ty, deps);
            }
            return factor.types.clone();
        }
        let mut types = vec![];
        for factor in &domain.factors {
            let ty = if let [ty] = factor.types.as_slice() {
                *ty
            } else {
                let ty = self.create(
                    self.key(
                        "owned-cross-domain/1",
                        cross,
                        "factor-intersection",
                        &[factor.end],
                    ),
                    c::TYPE,
                    deps,
                );
                for &target in &factor.types {
                    self.required_relationship(
                        cross,
                        (ty, target),
                        c::INTERSECTING,
                        (
                            p::INTERSECTING_TYPE_INTERSECTED,
                            p::INTERSECTING_INTERSECTING_TYPE,
                        ),
                        ("owned-cross-domain/1", "intersection"),
                        deps,
                    );
                }
                self.membership(
                    cross,
                    ty,
                    c::OWNING_MEMBERSHIP,
                    self.key(
                        "owned-cross-domain/1",
                        cross,
                        "intersection-owner",
                        &[factor.end],
                    ),
                    deps,
                );
                ty
            };
            types.push(ty);
        }
        let mut domain_type = types[0];
        let mut previous_product = None;
        for (i, &ty) in types.iter().enumerate().skip(1) {
            let product = self.create(
                self.key(
                    "owned-cross-domain/1",
                    cross,
                    "cartesian-product",
                    &[domain.factors[i].end],
                ),
                c::FEATURE,
                deps,
            );
            self.cross_typing(cross, product, ty, deps);
            self.cross_featuring(cross, product, domain_type, deps);
            if let Some(previous) = previous_product {
                self.membership(
                    product,
                    previous,
                    c::OWNING_MEMBERSHIP,
                    self.key(
                        "owned-cross-domain/1",
                        cross,
                        "cartesian-owner",
                        &[previous],
                    ),
                    deps,
                );
            }
            domain_type = product;
            previous_product = Some(product);
        }
        self.membership(
            cross,
            domain_type,
            c::OWNING_MEMBERSHIP,
            self.key("owned-cross-domain/1", cross, "cartesian-root-owner", &[]),
            deps,
        );
        self.cross_featuring(cross, cross, domain_type, deps);
        for &general in inherited_domains {
            if self
                .model
                .registry()
                .is_subtype(
                    self.model.element(general).expect("domain").metaclass(),
                    c::FEATURE,
                )
                .unwrap_or(false)
            {
                self.subset(cross, domain_type, general, "owned-cross-domain/1", deps);
            } else {
                self.cross_typing(cross, domain_type, general, deps);
            }
        }
        vec![domain_type]
    }
    fn initial_value_context(&mut self, that: ElementId, start: ElementId) -> ElementId {
        let inputs = [that, start];
        let deps = BTreeSet::from([
            Dependency::Declared(FactKey::Element(that)),
            Dependency::Declared(FactKey::Element(start)),
        ]);
        // This globally qualified chain has the same semantic context wherever
        // it is used. Its identity does not contain the requesting record ID.
        let chain = self.create(
            self.key("initial-feature-value-context/1", that, "chain", &inputs),
            c::FEATURE,
            &deps,
        );
        for (role, target) in [("first", that), ("last", start)] {
            let relation = self.create(
                self.key("initial-feature-value-context/1", that, role, &inputs),
                c::FEATURE_CHAINING,
                &deps,
            );
            self.set(
                relation,
                p::FEATURE_CHAINING_CHAINING_FEATURE,
                Value::Reference(target),
            );
            self.own(chain, relation);
        }
        chain
    }
    fn key(
        &self,
        rule: &str,
        subject: ElementId,
        role: &str,
        inputs: &[ElementId],
    ) -> DerivationKey {
        let bytes: Vec<_> = inputs
            .iter()
            .flat_map(|i| i.as_u128().to_be_bytes())
            .collect();
        DerivationKey {
            rule: rule_id(self.profile, rule),
            subject,
            output: OutputKey::from_u128(identity(&[role.as_bytes(), &bytes])),
        }
    }
    fn create(
        &mut self,
        key: DerivationKey,
        class: MetaclassId,
        dependencies: &BTreeSet<Dependency>,
    ) -> ElementId {
        let id = key.element_id();
        self.touched_records.insert(id);
        if let Some(existing) = self.records.get(&id) {
            if existing.key != key || existing.class != class {
                self.conflict = Some(FactKey::Element(id));
            }
            return id;
        }
        let registry = self.model.registry();
        let mut slots = BTreeMap::new();
        slots.insert(
            p::ELEMENT_ELEMENT_ID,
            SlotValue::Scalar(Value::String(id.to_string())),
        );
        for property in [
            p::ELEMENT_IS_IMPLIED_INCLUDED,
            p::RELATIONSHIP_IS_IMPLIED,
            p::TYPE_IS_ABSTRACT,
            p::TYPE_IS_SUFFICIENT,
            p::FEATURE_IS_COMPOSITE,
            p::FEATURE_IS_CONSTANT,
            p::FEATURE_IS_DERIVED,
            p::FEATURE_IS_END,
            p::FEATURE_IS_ORDERED,
            p::FEATURE_IS_PORTION,
            p::FEATURE_IS_VARIABLE,
        ] {
            if registry
                .resolve_property(class, property)
                .expect("KerML descriptor")
                .is_some_and(|property| !property.derived)
            {
                slots.insert(
                    property,
                    SlotValue::Scalar(Value::Boolean(
                        property == p::ELEMENT_IS_IMPLIED_INCLUDED
                            || property == p::RELATIONSHIP_IS_IMPLIED,
                    )),
                );
            }
        }
        if registry.is_subtype(class, c::FEATURE).expect("Feature") {
            slots.insert(
                p::FEATURE_IS_UNIQUE,
                SlotValue::Scalar(Value::Boolean(true)),
            );
        }
        if registry
            .is_subtype(class, c::MEMBERSHIP)
            .expect("Membership")
        {
            let agq_kernel::metamodel::ValueKind::Enumeration(domain) = registry
                .property(p::MEMBERSHIP_VISIBILITY)
                .expect("visibility")
                .value_kind
            else {
                unreachable!()
            };
            let literal = *registry
                .enumeration(domain)
                .expect("visibility domain")
                .literals
                .iter()
                .find(|(_, n)| n.as_str() == "public")
                .expect("public")
                .0;
            slots.insert(
                p::MEMBERSHIP_VISIBILITY,
                SlotValue::Scalar(Value::Enumeration(literal)),
            );
        }
        // Existing canonical facts keep their original proof. The materializer
        // still compares class, rule and every planned slot before reusing them.
        let explanation = if let Some(Origin::Derived(proof)) =
            self.model.element(id).map(|record| record.origin())
        {
            self.evidence_pool.intern_shared(proof.clone())
        } else {
            let mut normalized = self.producer_dependencies(dependencies);
            let subject = FactKey::Element(key.subject);
            normalized.insert(if self.model.declared_fact_origin(subject).is_some() {
                Dependency::Declared(subject)
            } else {
                Dependency::Derived(subject)
            });
            self.evidence_pool.intern(KernelExplanation {
                rule: key.rule,
                dependencies: normalized,
            })
        };
        self.records.insert(
            id,
            ImpliedRecord {
                key,
                class,
                slots,
                explanation,
            },
        );
        id
    }
    fn producer_dependencies(&self, dependencies: &BTreeSet<Dependency>) -> BTreeSet<Dependency> {
        let mut normalized = BTreeSet::new();
        for &dependency in dependencies {
            // Keep the actual prior contributors of an extensible ownership
            // collection. Depending on its aggregate key would let a later
            // relationship's ownership proof depend on its own antecedent.
            // Search reads still track the complete collection for invalidation.
            if let Dependency::Derived(FactKey::Property {
                element,
                property: p::ELEMENT_OWNED_RELATIONSHIP,
            }) = dependency
                && let Some(slot) = self
                    .model
                    .navigation_slot(element, p::ELEMENT_OWNED_RELATIONSHIP)
                && let Origin::Derived(proof) = slot.origin()
            {
                normalized.extend(proof.dependencies.iter().copied());
            } else {
                normalized.insert(dependency);
            }
        }
        normalized
    }
    fn set(&mut self, id: ElementId, property: PropertyId, value: Value) {
        self.set_value(id, property, SlotValue::Scalar(value));
    }
    fn set_value(&mut self, id: ElementId, property: PropertyId, value: SlotValue) {
        let slots = &mut self.records.get_mut(&id).expect("implied record").slots;
        if !self.assignments.insert((id, property)) && slots.get(&property) != Some(&value) {
            self.conflict = Some(FactKey::Property {
                element: id,
                property,
            });
        } else {
            slots.insert(property, value);
        }
    }
    fn own(&mut self, owner: ElementId, relationship: ElementId) {
        if self
            .model
            .navigation_slot(owner, p::ELEMENT_OWNED_RELATIONSHIP)
            .is_some_and(|s| {
                s.value()
                    .values()
                    .any(|v| *v == Value::Reference(relationship))
            })
        {
            return;
        }
        if self.model.element(owner).is_none()
            && let Some(record) = self.records.get_mut(&owner)
        {
            let SlotValue::Ordered(values) = record
                .slots
                .entry(p::ELEMENT_OWNED_RELATIONSHIP)
                .or_insert_with(|| SlotValue::Ordered(vec![]))
            else {
                unreachable!()
            };
            if !values.contains(&Value::Reference(relationship)) {
                values.push(Value::Reference(relationship));
            }
        } else {
            self.attachments
                .entry(owner)
                .or_default()
                .insert(relationship);
        }
    }
    fn membership(
        &mut self,
        owner: ElementId,
        target: ElementId,
        class: MetaclassId,
        key: DerivationKey,
        deps: &BTreeSet<Dependency>,
    ) {
        let member = self.create(key, class, deps);
        self.set_value(
            member,
            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(target)]),
        );
        self.own(owner, member);
    }
    fn contextual(
        &mut self,
        subject: ElementId,
        expression: ElementId,
        raw: ElementId,
        rule: ResultDomainRule,
        deps: &BTreeSet<Dependency>,
    ) -> ContextualResult {
        let name = rule.constraint();
        let inputs = [expression, raw];
        let key = self.key(name, subject, "contextual-result", &inputs);
        let is_new = !self.records.contains_key(&key.element_id());
        let feature = self.create(key, c::FEATURE, deps);
        if !self.profile.corrects_owned_cross_feature()
            || !matches!(
                rule,
                ResultDomainRule::FeatureValuation | ResultDomainRule::FeatureValueBinding
            )
        {
            self.membership(
                subject,
                feature,
                c::OWNING_MEMBERSHIP,
                self.key(name, subject, "contextual-membership", &inputs),
                deps,
            );
        } else if is_new {
            self.unattached_contextual.insert(feature);
        }
        for (role, target) in [("chain-expression", expression), ("chain-result", raw)] {
            let chain = self.create(
                self.key(name, subject, role, &inputs),
                c::FEATURE_CHAINING,
                deps,
            );
            self.set(
                chain,
                p::FEATURE_CHAINING_CHAINING_FEATURE,
                Value::Reference(target),
            );
            self.own(feature, chain);
        }
        ContextualResult {
            feature,
            expression,
            raw_result: raw,
            rule,
        }
    }
    fn subset(
        &mut self,
        subject: ElementId,
        specific: ElementId,
        general: ElementId,
        rule: &str,
        deps: &BTreeSet<Dependency>,
    ) {
        let relationship = self.create(
            self.key(rule, subject, "subsetting", &[specific, general]),
            c::SUBSETTING,
            deps,
        );
        self.set(
            relationship,
            p::SUBSETTING_SUBSETTING_FEATURE,
            Value::Reference(specific),
        );
        self.set(
            relationship,
            p::SUBSETTING_SUBSETTED_FEATURE,
            Value::Reference(general),
        );
        self.own(specific, relationship);
        self.own_unattached_contextual(relationship, general);
    }

    // A contextual target is semantic infrastructure of the relationship using
    // it, not an additional owned member of an end Feature. Preserve historical
    // v6 ownership; v7 fixes the producer defect exposed by KERML11-1 integration.
    fn own_unattached_contextual(&mut self, relationship: ElementId, target: ElementId) {
        if !self.unattached_contextual.remove(&target) {
            return;
        }
        self.set_value(
            relationship,
            p::RELATIONSHIP_OWNED_RELATED_ELEMENT,
            SlotValue::Ordered(vec![Value::Reference(target)]),
        );
    }
    fn binding(
        &mut self,
        owner: ElementId,
        endpoints: [ElementId; 2],
        domain: Option<ElementId>,
        role: ImpliedBindingRole,
        deps: &BTreeSet<Dependency>,
    ) -> ElementId {
        let rule = role.constraint();
        let key = self.key(rule, owner, "binding", &endpoints);
        let binding = self.create(key, c::BINDING_CONNECTOR, deps);
        let membership = if matches!(
            role,
            ImpliedBindingRole::ExpressionResult
                | ImpliedBindingRole::FunctionResult
                | ImpliedBindingRole::Invocation
        ) {
            c::FEATURE_MEMBERSHIP
        } else {
            c::OWNING_MEMBERSHIP
        };
        self.membership(
            owner,
            binding,
            membership,
            self.key(rule, owner, "binding-membership", &endpoints),
            deps,
        );
        for (end_role, member_role, reference_role, endpoint) in [
            (
                "first-end",
                "first-end-membership",
                "first-reference",
                endpoints[0],
            ),
            (
                "second-end",
                "second-end-membership",
                "second-reference",
                endpoints[1],
            ),
        ] {
            let end = self.create(
                self.key(rule, owner, end_role, &endpoints),
                c::FEATURE,
                deps,
            );
            self.set(end, p::FEATURE_IS_END, Value::Boolean(true));
            self.membership(
                binding,
                end,
                c::END_FEATURE_MEMBERSHIP,
                self.key(rule, owner, member_role, &endpoints),
                deps,
            );
            let reference = self.create(
                self.key(rule, owner, reference_role, &endpoints),
                c::REFERENCE_SUBSETTING,
                deps,
            );
            self.set(
                reference,
                p::REFERENCE_SUBSETTING_REFERENCED_FEATURE,
                Value::Reference(endpoint),
            );
            self.own(end, reference);
            self.own_unattached_contextual(reference, endpoint);
        }
        if let Some(domain) = domain {
            let featuring = self.create(
                self.key(rule, owner, "binding-featuring", &endpoints),
                c::TYPE_FEATURING,
                deps,
            );
            self.set(
                featuring,
                p::TYPE_FEATURING_FEATURE_OF_TYPE,
                Value::Reference(binding),
            );
            self.set(
                featuring,
                p::TYPE_FEATURING_FEATURING_TYPE,
                Value::Reference(domain),
            );
            self.own(binding, featuring);
        }
        binding
    }
    fn build(self, snapshot: &Snapshot) -> Result<DerivedOverlay, DerivationError> {
        self.build_with(DerivationBuilder::new(snapshot.clone()))
    }
    fn build_with(self, mut builder: DerivationBuilder) -> Result<DerivedOverlay, DerivationError> {
        self.enqueue(&mut builder)?;
        builder.build()
    }
    fn enqueue<Input>(self, builder: &mut DerivationBuilder<Input>) -> Result<(), DerivationError> {
        if let Some(fact) = self.conflict {
            return Err(DerivationError::DuplicateFact(fact));
        }
        contributions::enqueue_properties(self.model, self.contributed_properties, builder)?;
        let mut owner_orders: BTreeMap<_, Vec<_>> = self
            .records
            .iter()
            .filter_map(|(&id, r)| {
                r.slots.get(&p::ELEMENT_OWNED_RELATIONSHIP).map(|v| {
                    (
                        id,
                        v.values()
                            .filter_map(|v| {
                                if let Value::Reference(id) = v {
                                    Some(*id)
                                } else {
                                    None
                                }
                            })
                            .collect(),
                    )
                })
            })
            .collect();
        for (&owner, additions) in &self.attachments {
            let order = owner_orders.entry(owner).or_insert_with(|| {
                self.model
                    .navigation_slot(owner, p::ELEMENT_OWNED_RELATIONSHIP)
                    .into_iter()
                    .flat_map(|s| s.value().values())
                    .filter_map(|v| {
                        if let Value::Reference(id) = v {
                            Some(*id)
                        } else {
                            None
                        }
                    })
                    .collect()
            });
            order.extend(additions.iter().copied());
        }
        let mut occurrences = vec![];
        for record in self.records.into_values() {
            if let Some(existing) = self.model.element(record.key.element_id()) {
                if existing.metaclass() != record.class
                    || !matches!(existing.origin(),
                    agq_kernel::provenance::Origin::Derived(e) if e.rule == record.key.rule)
                {
                    return Err(DerivationError::IdentityCollision(existing.id()));
                }
                for (property, value) in &record.slots {
                    if self
                        .model
                        .navigation_slot(existing.id(), *property)
                        .map(|s| s.value())
                        != Some(value)
                    {
                        if std::env::var_os("AGQ_TRACE_DERIVATION_CONFLICT").is_some() {
                            eprintln!(
                                "publication-conflict key={:?} property={property} previous={:?} proposed={value:?} subject_name={:?}",
                                record.key,
                                self.model
                                    .navigation_slot(existing.id(), *property)
                                    .map(|slot| slot.value()),
                                self.model
                                    .navigation_slot(record.key.subject, p::ELEMENT_DECLARED_NAME)
                                    .map(|slot| slot.value()),
                            );
                        }
                        return Err(DerivationError::DuplicateFact(FactKey::Property {
                            element: existing.id(),
                            property: *property,
                        }));
                    }
                }
                continue;
            }
            // Retain the producer's actual bounded searches, including misses.
            // Reading this fact in a later producer transfers these dependencies
            // without turning every derived fact into a whole-model search.
            builder.searches_shared(
                FactKey::Element(record.key.element_id()),
                self.searches
                    .get(&record.key.element_id())
                    .cloned()
                    .unwrap_or_default(),
            );
            let mut slots = BTreeMap::new();
            for (property, value) in record.slots {
                let registry = self.model.registry();
                let descriptor = registry
                    .property(property)
                    .map_err(agq_kernel::ModelError::from)?;
                let derived_association = descriptor.derived
                    && descriptor.association.is_some_and(|a| {
                        registry
                            .supports_derived_occurrence_storage(a)
                            .unwrap_or(false)
                    });
                if registry
                    .supports_slot_storage(property)
                    .map_err(agq_kernel::ModelError::from)?
                    && !derived_association
                {
                    slots.insert(property, value);
                    continue;
                }
                let descriptor = registry
                    .property(property)
                    .map_err(agq_kernel::ModelError::from)?;
                let association = descriptor.association.ok_or(
                    agq_kernel::ModelError::UnsupportedAssociationStorage(property),
                )?;
                let opposite = *descriptor.opposite_ends.first().ok_or(
                    agq_kernel::ModelError::UnsupportedAssociationStorage(property),
                )?;
                for (position, value) in value.values().enumerate() {
                    let Value::Reference(target) = value else {
                        return Err(agq_kernel::ModelError::UnsupportedAssociationStorage(
                            property,
                        )
                        .into());
                    };
                    let ends =
                        BTreeMap::from([(property, *target), (opposite, record.key.element_id())]);
                    let mut positions = BTreeMap::new();
                    if descriptor.ordered && !matches!(descriptor.multiplicity.upper, Some(0 | 1)) {
                        positions.insert(property, position);
                    }
                    occurrences.push((record.key, association, ends, positions));
                }
            }
            builder.element_with_explanation(record.key, record.class, slots, record.explanation);
        }
        // An ordered inverse of an owned relationship is selected from the
        // canonical owner's ordered relationships. It must include precisely the
        // participants of that end, preserving the declared prefix. Never infer
        // semantic end order from allocation or producer traversal order.
        let mut groups = BTreeMap::<(ElementId, PropertyId), BTreeSet<ElementId>>::new();
        for (_, _, ends, _) in &occurrences {
            for (&end, &target) in ends {
                let p = self
                    .model
                    .registry()
                    .property(end)
                    .map_err(agq_kernel::ModelError::from)?;
                let context = ends[p.opposite_ends.first().expect("binary association")];
                groups.entry((context, end)).or_default().insert(target);
            }
        }
        for link in self.model.association_occurrences() {
            for (&end, &target) in link.ends() {
                let p = self
                    .model
                    .registry()
                    .property(end)
                    .map_err(agq_kernel::ModelError::from)?;
                let context = link.ends()[p.opposite_ends.first().expect("binary association")];
                if let Some(group) = groups.get_mut(&(context, end)) {
                    group.insert(target);
                }
            }
        }
        for (key, association, ends, mut positions) in occurrences {
            for (&end, &target) in &ends {
                let p = self
                    .model
                    .registry()
                    .property(end)
                    .map_err(agq_kernel::ModelError::from)?;
                if !p.ordered
                    || matches!(p.multiplicity.upper, Some(0 | 1))
                    || positions.contains_key(&end)
                {
                    continue;
                }
                let context = ends[p.opposite_ends.first().expect("binary association")];
                let targets = &groups[&(context, end)];
                let order: Vec<_> = owner_orders
                    .get(&context)
                    .into_iter()
                    .flatten()
                    .filter(|id| targets.contains(id))
                    .copied()
                    .collect();
                if order.iter().copied().collect::<BTreeSet<_>>() != *targets {
                    return Err(agq_kernel::ModelError::UnsupportedAssociationStorage(end).into());
                }
                positions.insert(
                    end,
                    order
                        .iter()
                        .position(|id| *id == target)
                        .expect("complete group"),
                );
            }
            builder.association_occurrence(key, association, ends, positions, BTreeSet::new());
        }
        for (owner, additions) in self.attachments {
            builder.extend_ordered_references(
                owner,
                p::ELEMENT_OWNED_RELATIONSHIP,
                additions.into_iter().collect(),
                KernelExplanation {
                    rule: rule_id(self.profile, "structural-result-ownership/1"),
                    dependencies: BTreeSet::new(),
                },
            );
        }
        Ok(())
    }
}

/// An immutable canonical overlay and the completeness of its producers.
/// An incomplete derivation is never an accepted semantic publication.
pub struct ResultStructure {
    pub overlay: DerivedOverlay,
    pub production: QueryResult<Vec<ElementId>>,
    pub contextual_results: Vec<ContextualResult>,
}

/// Unpublished derivation retaining every missing construction obligation.
/// It cannot be used as a strict overlay or a publication certificate.
pub struct ConstructionResultStructure {
    pub overlay: agq_kernel::derived::ConstructionOverlay,
    pub production: QueryResult<Vec<ElementId>>,
    pub contextual_results: Vec<ContextualResult>,
}

/// A bounded producer plan over one immutable semantic context. This is a
/// proposed derivation, not canonical storage or an accepted publication.
/// Construction inputs can be audited without inventing a valid Snapshot.
pub struct ResultStructurePlan<'m> {
    graph: Graph<'m>,
    pub(crate) producer_families_attempted: usize,
    pub(crate) producer_evaluations: Vec<(ElementId, ProducerFamilyId, Completeness)>,
    pub(crate) deferred_bindings: BTreeSet<ElementId>,
    /// Exact input identity, including unresolved construction obligations.
    pub context: SemanticContextId,
    pub production: QueryResult<Vec<ElementId>>,
    pub contextual_results: Vec<ContextualResult>,
}
impl ResultStructurePlan<'_> {
    /// Record one extension family's complete evaluation, including a false
    /// dynamic antecedent. The scheduler authenticates this against the registry.
    pub fn record_producer_evaluation(
        &mut self,
        subject: ElementId,
        family: ProducerFamilyId,
        completeness: Completeness,
    ) {
        self.producer_evaluations
            .push((subject, family, completeness));
    }

    /// Publication batches retain canonical per-fact evidence in the graph.
    /// Release the redundant aggregate query proof before the kernel allocates
    /// the merged overlay. This internal status is never returned as a query.
    pub(crate) fn discard_aggregate_proof(&mut self) {
        self.production.value.clear();
        self.production.explanations.clear();
        self.production.fact_origins.clear();
        self.production.declared_fact_origins.clear();
        self.production.canonical_dependencies.clear();
        self.production.positive_dependencies.clear();
        self.production.clear_search_dependencies();
        self.contextual_results.clear();
    }
    /// Stable implied identities, useful for independent batch comparisons.
    pub fn planned_elements(&self) -> impl Iterator<Item = ElementId> + '_ {
        self.graph.records.keys().copied()
    }
    /// An exact additive change boundary: existing records are checked and
    /// reused; only fresh records and ownership extensions can alter navigation.
    /// References include both participant and occurrence navigation endpoints.
    pub(crate) fn changed_population(&self) -> BTreeSet<ElementId> {
        let mut result: BTreeSet<_> = self.graph.attachments.keys().copied().collect();
        for (&(element, property), contribution) in &self.graph.contributed_properties {
            if self
                .graph
                .model
                .navigation_slot(element, property)
                .is_some()
            {
                continue;
            }
            result.insert(element);
            result.extend(contribution.value.values().filter_map(|value| {
                if let Value::Reference(target) = value {
                    Some(*target)
                } else {
                    None
                }
            }));
        }
        for (&id, record) in &self.graph.records {
            if self.graph.model.element(id).is_some() {
                continue;
            }
            result.insert(id);
            result.extend(
                record
                    .slots
                    .values()
                    .flat_map(|v| v.values())
                    .filter_map(|v| {
                        if let Value::Reference(id) = v {
                            Some(*id)
                        } else {
                            None
                        }
                    }),
            );
        }
        for additions in self.graph.attachments.values() {
            result.extend(additions);
        }
        result
    }
    pub(crate) fn existing_elements_reused(&self) -> usize {
        self.graph
            .records
            .keys()
            .filter(|id| self.graph.model.element(**id).is_some())
            .count()
    }
    /// Finish all graph reads and release the borrowed input before consuming
    /// the previous overlay. The worklist is the only caller and supplies the
    /// exact input checked here to `build_on_overlay` immediately afterward.
    pub(crate) fn prepare_on_overlay(
        self,
        input: &DerivedOverlay,
    ) -> Result<DerivationBuilder, DerivationError> {
        if input.base_revision() != self.context.revision
            || !std::ptr::eq(input.model(), self.graph.model)
        {
            return Err(DerivationError::InputContextMismatch);
        }
        let mut builder = DerivationBuilder::new(input.declared().clone());
        self.graph.enqueue(&mut builder)?;
        Ok(builder)
    }
    pub(crate) fn prepare_on_construction_overlay(
        self,
        input: &agq_kernel::derived::ConstructionOverlay,
    ) -> Result<agq_kernel::derived::ConstructionDerivationBuilder, DerivationError> {
        if input.base_revision() != self.context.revision
            || !std::ptr::eq(input.model(), self.graph.model)
        {
            return Err(DerivationError::InputContextMismatch);
        }
        let mut builder = agq_kernel::derived::ConstructionDerivationBuilder::for_construction(
            input.declared_shared().clone(),
        );
        self.graph.enqueue(&mut builder)?;
        Ok(builder)
    }
    /// Materialize only an unpublished construction overlay. Required endpoint
    /// deficits remain explicit and all other kernel invariants still apply.
    pub fn materialize_construction(
        self,
        candidate: Arc<agq_kernel::ConstructionView>,
    ) -> Result<ConstructionResultStructure, DerivationError> {
        if candidate.revision() != self.context.revision
            || !std::ptr::eq(candidate.model(), self.graph.model)
        {
            return Err(DerivationError::InputContextMismatch);
        }
        let mut builder =
            agq_kernel::derived::ConstructionDerivationBuilder::for_construction(candidate);
        self.graph.enqueue(&mut builder)?;
        Ok(ConstructionResultStructure {
            overlay: builder.build()?,
            production: self.production,
            contextual_results: self.contextual_results,
        })
    }
    /// Merge disjoint subject batches from the identical context. Duplicate
    /// facts must agree exactly, including provenance. Contributions to existing
    /// ownership collections merge by stable identity, preserving declared
    /// prefixes. Semantic sequences on fresh records must still agree exactly.
    pub fn merge(&mut self, other: Self) -> Result<(), DerivationError> {
        if self.context != other.context || !std::ptr::eq(self.graph.model, other.graph.model) {
            return Err(DerivationError::InputContextMismatch);
        }
        if let Some(fact) = self.graph.conflict.or(other.graph.conflict) {
            return Err(DerivationError::DuplicateFact(fact));
        }
        for (key, contribution) in other.graph.contributed_properties {
            contributions::merge_property(
                &mut self.graph.contributed_properties,
                key,
                contribution,
            )?;
        }
        for (&id, record) in &other.graph.records {
            if self
                .graph
                .records
                .get(&id)
                .is_some_and(|existing| existing != record)
            {
                return Err(DerivationError::DuplicateFact(FactKey::Element(id)));
            }
        }
        for (id, mut record) in other.graph.records {
            record.explanation = self.graph.evidence_pool.intern_shared(record.explanation);
            self.graph.records.insert(id, record);
        }
        for (id, searches) in other.graph.searches {
            self.graph.merge_searches(id, searches);
        }
        self.graph
            .direct_searches
            .extend(other.graph.direct_searches);
        self.producer_families_attempted += other.producer_families_attempted;
        self.producer_evaluations.extend(other.producer_evaluations);
        self.deferred_bindings.extend(other.deferred_bindings);
        self.graph.assignments.extend(other.graph.assignments);
        for (owner, additions) in other.graph.attachments {
            self.graph
                .attachments
                .entry(owner)
                .or_default()
                .extend(additions);
        }
        self.graph
            .unattached_contextual
            .extend(other.graph.unattached_contextual);
        self.production
            .value
            .extend(other.production.value.iter().copied());
        self.production.merge(other.production);
        self.production.value.sort();
        self.production.value.dedup();
        self.contextual_results.extend(other.contextual_results);
        self.contextual_results.sort_by_key(|r| r.feature);
        self.contextual_results.dedup();
        Ok(())
    }
    /// Materialization requires the identical validated Snapshot; a plan over
    /// an incomplete construction cannot be converted into an overlay here.
    pub fn materialize(self, snapshot: &Snapshot) -> Result<ResultStructure, DerivationError> {
        if snapshot.revision() != self.context.revision
            || !std::ptr::eq(snapshot.model(), self.graph.model)
        {
            return Err(DerivationError::InputContextMismatch);
        }
        Ok(ResultStructure {
            overlay: self.graph.build(snapshot)?,
            production: self.production,
            contextual_results: self.contextual_results,
        })
    }
    /// Add a producer stage to its exact input overlay. The kernel retains all
    /// earlier declared and derived facts and validates the combined evidence DAG.
    pub fn materialize_on_overlay(
        self,
        input: &DerivedOverlay,
    ) -> Result<ResultStructure, DerivationError> {
        if input.base_revision() != self.context.revision
            || !std::ptr::eq(input.model(), self.graph.model)
        {
            return Err(DerivationError::InputContextMismatch);
        }
        Ok(ResultStructure {
            overlay: self
                .graph
                .build_with(DerivationBuilder::from_overlay(input.clone()))?,
            production: self.production,
            contextual_results: self.contextual_results,
        })
    }
}

impl<'m> KerMlQueries<'m> {
    fn required_structural_result<T>(
        &self,
        out: &mut QueryResult<T>,
        expression: ElementId,
    ) -> Option<ElementId> {
        let result = self.expression_result(out, expression);
        if self
            .context()
            .options
            .baseline_profile
            .supports_publication_producers()
            && self.is(expression, c::INSTANTIATION_EXPRESSION)
            && let Some(result) = result
        {
            let owner = self.owning_type(result);
            let owned = owner.value == Some(expression);
            out.merge(owner);
            if !owned {
                out.problem(
                    Completeness::Incomplete,
                    "KQ_OWNED_INSTANTIATION_RESULT",
                    expression,
                    "Instantiation result ownership has not been produced",
                );
                return None;
            }
        }
        if result.is_none() {
            out.problem(
                Completeness::Incomplete,
                "KQ_REQUIRED_RESULT",
                expression,
                "A unique canonical result is required for structural implication",
            );
        }
        result
    }
    /// Construct structural result implications with profile-qualified provenance.
    pub fn derive_result_structure(
        &self,
        snapshot: &Snapshot,
    ) -> Result<ResultStructure, DerivationError> {
        if snapshot.revision() != self.context().revision
            || !std::ptr::eq(snapshot.model(), self.model())
        {
            return Err(DerivationError::InputContextMismatch);
        }
        self.plan_result_structure(self.model().elements().map(|r| r.id()))
            .materialize(snapshot)
    }

    /// Execute all result producers for the supplied subjects. The scope is
    /// explicit so a corpus audit can release query evidence between batches.
    /// No rule is skipped merely because an unrelated reference is unresolved.
    pub fn plan_result_structure(
        &self,
        subjects: impl IntoIterator<Item = ElementId>,
    ) -> ResultStructurePlan<'m> {
        self.plan_result_structure_in_stratum(subjects, ResultStructureStratum::ContextualBindings)
    }
    pub(crate) fn plan_result_structure_in_stratum(
        &self,
        subjects: impl IntoIterator<Item = ElementId>,
        stratum: ResultStructureStratum,
    ) -> ResultStructurePlan<'m> {
        let mut plan = self.plan_result_structure_subjects([], stratum);
        for subject in subjects.into_iter().collect::<BTreeSet<_>>() {
            let part = self.plan_result_structure_subjects([subject], stratum);
            // Preserve the established API's deferred collision behavior.
            if let Err(error) = plan.merge(part) {
                plan.graph.conflict = Some(match error {
                    DerivationError::DuplicateFact(fact) => fact,
                    _ => FactKey::Element(subject),
                });
            }
        }
        plan
    }
    /// Original shared-graph batch planner, retained as an independent test
    /// oracle for the optimized per-subject extraction and plan merge.
    pub(crate) fn plan_result_structure_reference(
        &self,
        subjects: impl IntoIterator<Item = ElementId>,
        stratum: ResultStructureStratum,
    ) -> ResultStructurePlan<'m> {
        self.plan_result_structure_subjects(subjects, stratum)
    }
    fn plan_result_structure_subjects(
        &self,
        subjects: impl IntoIterator<Item = ElementId>,
        stratum: ResultStructureStratum,
    ) -> ResultStructurePlan<'m> {
        let profile = self.context().options.baseline_profile;
        let mut graph = Graph {
            model: self.model(),
            profile,
            records: BTreeMap::new(),
            evidence_pool: ExplanationPool::default(),
            attachments: BTreeMap::new(),
            unattached_contextual: BTreeSet::new(),
            assignments: BTreeSet::new(),
            conflict: None,
            searches: BTreeMap::new(),
            search_pool: StructuralSearchPool::default(),
            direct_searches: BTreeSet::new(),
            touched_records: BTreeSet::new(),
            contributed_properties: BTreeMap::new(),
        };
        let mut aggregate = self.result(vec![]);
        let mut contextual_results = vec![];
        let mut producer_families_attempted = 0;
        let mut producer_evaluations = vec![];
        let mut deferred_bindings = BTreeSet::new();
        for subject in subjects.into_iter().collect::<BTreeSet<_>>() {
            let mut production = self.result(vec![]);
            if self.model().element(subject).is_none() {
                production.problem(
                    Completeness::Invalid,
                    "KQ_PRODUCER_SUBJECT",
                    subject,
                    "Missing producer subject",
                );
                graph.finish_subject(&mut production, &mut aggregate);
                continue;
            }
            if profile.supports_publication_producers()
                && self.is(subject, c::INSTANTIATION_EXPRESSION)
            {
                producer_families_attempted += 1;
                let mut proof = self.result(());
                let members = self.memberships(subject);
                let has_owned_result = members
                    .value
                    .iter()
                    .any(|&m| self.is(m, c::RETURN_PARAMETER_MEMBERSHIP));
                proof.merge(members);
                if !has_owned_result {
                    if proof.completeness == Completeness::Complete {
                        graph.return_result(subject, &proof.canonical_dependencies);
                        proof.problem(Completeness::Incomplete, "KQ_RESULT_STAGE", subject,
                            "Owned result was staged; dependent producers require the next overlay stage");
                    }
                    producer_evaluations.push((
                        subject,
                        ProducerFamily::OwnedInstantiationResult.id(),
                        proof.completeness,
                    ));
                    production.merge(proof);
                    graph.finish_subject(&mut production, &mut aggregate);
                    continue;
                }
                producer_evaluations.push((
                    subject,
                    ProducerFamily::OwnedInstantiationResult.id(),
                    proof.completeness,
                ));
            }
            if profile.supports_publication_producers() && self.is(subject, c::FEATURE) {
                producer_families_attempted += 2;
                let redefinitions = self.implied_redefinitions(subject);
                if redefinitions.completeness == Completeness::Complete {
                    for &target in &redefinitions.value {
                        if let Some(proofs) = redefinitions.explanations.get(&Conclusion {
                            query: QueryKind::RedefinedFeatures,
                            subject,
                            value: target,
                        }) {
                            for proof in proofs {
                                let rule = match proof.rule {
                                    Rule::ParameterRedefinition => {
                                        "checkFeatureParameterRedefinition"
                                    }
                                    Rule::EndRedefinition => "checkFeatureEndRedefinition",
                                    Rule::ResultRedefinition => "checkFeatureResultRedefinition",
                                    _ => continue,
                                };
                                graph.required_relationship(
                                    subject,
                                    (subject, target),
                                    c::REDEFINITION,
                                    (
                                        p::REDEFINITION_REDEFINING_FEATURE,
                                        p::REDEFINITION_REDEFINED_FEATURE,
                                    ),
                                    (rule, "positional-redefinition"),
                                    &redefinitions.canonical_dependencies,
                                );
                            }
                        }
                    }
                }
                producer_evaluations.push((
                    subject,
                    ProducerFamily::PositionalRedefinition.id(),
                    redefinitions.completeness,
                ));
                production.merge(redefinitions);
                let mut proof = self.result(());
                if matches!(
                    self.read_value(&mut proof, subject, p::FEATURE_IS_VARIABLE),
                    Some(Value::Boolean(true))
                ) {
                    let owner = self.owning_type(subject);
                    if let Some(owner_id) = owner.value {
                        let snapshots = self.standard_role(StandardRole::OccurrenceSnapshots);
                        let occurrence = self.standard_role(StandardRole::Occurrence);
                        if let (Some(snapshots_id), Some(occurrence_id)) =
                            (snapshots.value, occurrence.value)
                        {
                            let mut domain = (owner_id == occurrence_id).then_some(snapshots_id);
                            if domain.is_none() {
                                let mut scope = self.result(());
                                let features = self.effective_features(owner_id);
                                let mut candidates = vec![];
                                for &feature in &features.value {
                                    self.read_value(&mut scope, feature, p::FEATURE_IS_VARIABLE);
                                    let redefined = self.all_redefined_features(feature);
                                    if redefined.value.contains(&snapshots_id) {
                                        let domains = self.featuring_types(feature);
                                        if domains.value.contains(&owner_id) {
                                            candidates.push(feature);
                                        }
                                        scope.merge(domains);
                                    }
                                    scope.merge(redefined);
                                }
                                scope.merge(features);
                                let mut scope_deps = scope.canonical_dependencies.clone();
                                scope_deps.extend(snapshots.canonical_dependencies.iter().copied());
                                scope_deps
                                    .extend(occurrence.canonical_dependencies.iter().copied());
                                proof.merge(scope);
                                match candidates.as_slice() {
                                    [candidate] => domain = Some(*candidate),
                                    [] if proof.completeness == Completeness::Complete => {
                                        domain = Some(graph.variable_snapshot(owner_id, snapshots_id, &scope_deps));
                                    }
                                    [] => {},
                                    _ => proof.problem(Completeness::Incomplete, "KQ_SNAPSHOT_DOMAIN_AMBIGUOUS", subject,
                                        "Multiple effective snapshot redefinitions require a unique featuring domain"),
                                }
                            }
                            if let Some(domain) = domain
                                && proof.completeness == Completeness::Complete
                            {
                                graph.variable_featuring(
                                    subject,
                                    domain,
                                    &proof.canonical_dependencies,
                                );
                            }
                        }
                        proof.merge(snapshots);
                        proof.merge(occurrence);
                    } else {
                        proof.problem(
                            Completeness::Incomplete,
                            "KQ_VARIABLE_OWNER",
                            subject,
                            "A variable Feature requires its owning Type",
                        );
                    }
                    proof.merge(owner);
                }
                producer_evaluations.push((
                    subject,
                    ProducerFamily::VariableFeaturing.id(),
                    proof.completeness,
                ));
                production.merge(proof);
            }
            if profile.corrects_owned_cross_domain() && self.is(subject, c::FEATURE) {
                producer_families_attempted += 1;
                let mut proof = self.owned_cross_feature(subject);
                if let Some(cross) = proof.value {
                    let existing = self.owned_cross_subsetting(subject);
                    let has_crossing = existing.value.is_some();
                    proof.merge(existing);
                    if has_crossing {
                        let actual = self.cross_feature(subject);
                        if actual.value != Some(cross)
                            && actual.completeness == Completeness::Complete
                        {
                            proof.problem(Completeness::Invalid, "KQ_CROSSING_CONFLICT", subject,
                                "The canonical crossed chain conflicts with the selected owned cross Feature");
                        }
                        proof.merge(actual);
                    } else {
                        let owner = self.owning_type(subject);
                        if let Some(owner) = owner.value {
                            let ends = self.structural_end_features(owner);
                            let other: Vec<_> = ends
                                .value
                                .iter()
                                .copied()
                                .filter(|e| *e != subject)
                                .collect();
                            proof.merge(ends);
                            if let [first] = other.as_slice() {
                                if proof.completeness == Completeness::Complete {
                                    graph.crossing(
                                        subject,
                                        *first,
                                        cross,
                                        &proof.canonical_dependencies,
                                    );
                                }
                            } else if other.len() > 1
                                && proof.completeness == Completeness::Complete
                            {
                                let first =
                                    graph.crossing_product(&other, &proof.canonical_dependencies);
                                let relationship = graph.crossing(
                                    subject,
                                    first,
                                    cross,
                                    &proof.canonical_dependencies,
                                );
                                let SlotValue::Ordered(owned) = graph
                                    .records
                                    .get_mut(&relationship)
                                    .expect("cross-subsetting")
                                    .slots
                                    .get_mut(&p::RELATIONSHIP_OWNED_RELATED_ELEMENT)
                                    .expect("owned chain")
                                else {
                                    unreachable!()
                                };
                                owned.push(Value::Reference(first));
                            }
                        }
                        proof.merge(owner);
                    }
                }
                producer_evaluations.push((
                    subject,
                    ProducerFamily::OwnedCrossing.id(),
                    proof.completeness,
                ));
                production.merge(proof);
            }
            if profile.corrects_owned_cross_domain() && self.is(subject, c::FEATURE) {
                producer_families_attempted += 1;
                let mut proof = self.owned_cross_feature_domain(subject);
                if let Some(domain) = proof.value.clone() {
                    let mut inherited_domains = vec![];
                    for &inherited in &domain.inherited_cross_features {
                        let featuring = self.featuring_types(inherited);
                        inherited_domains.extend(featuring.value.iter().copied());
                        proof.merge(featuring);
                    }
                    let types = self.feature_types(domain.owning_end);
                    let required_types = types.value.clone();
                    proof.merge(types);
                    if proof.completeness == Completeness::Complete {
                        graph.cross_domain(
                            subject,
                            &domain,
                            &inherited_domains,
                            &proof.canonical_dependencies,
                        );
                        for ty in required_types {
                            graph.cross_typing(subject, subject, ty, &proof.canonical_dependencies);
                        }
                        for &inherited in &domain.inherited_cross_features {
                            graph.subset(
                                subject,
                                subject,
                                inherited,
                                "owned-cross-domain/1",
                                &proof.canonical_dependencies,
                            );
                        }
                    }
                }
                producer_evaluations.push((
                    subject,
                    ProducerFamily::CrossDomain.id(),
                    proof.completeness,
                ));
                production.merge(proof);
            }
            if profile.supports_publication_producers()
                && self.is(subject, c::INVOCATION_EXPRESSION)
            {
                producer_families_attempted += 1;
                let mut proof = self.result(());
                let instantiated = self.instantiated_type(subject);
                if let Some(target) = instantiated.value {
                    let function = if self.is(target, c::FEATURE) {
                        let types = self.feature_types(target);
                        let function = types.value.iter().any(|&ty| self.is(ty, c::FUNCTION));
                        proof.merge(types);
                        function
                    } else {
                        self.is(target, c::FUNCTION)
                    };
                    proof.merge(instantiated);
                    if proof.completeness == Completeness::Complete {
                        graph.specialize(
                            subject,
                            target,
                            "checkInvocationExpressionSpecialization",
                            &proof.canonical_dependencies,
                        );
                    }
                    if !function
                        && let Some(raw) = self.required_structural_result(&mut proof, subject)
                        && proof.completeness == Completeness::Complete
                    {
                        graph.specialize(
                            raw,
                            target,
                            "checkInvocationExpressionBehaviorResultSpecialization",
                            &proof.canonical_dependencies,
                        );
                        production.value.push(graph.binding(
                            subject,
                            [subject, raw],
                            Some(subject),
                            ImpliedBindingRole::Invocation,
                            &proof.canonical_dependencies,
                        ));
                    }
                } else {
                    proof.merge(instantiated);
                }
                producer_evaluations.push((
                    subject,
                    ProducerFamily::Invocation.id(),
                    proof.completeness,
                ));
                production.merge(proof);
            }
            if profile.supports_publication_producers()
                && self.is(subject, c::FEATURE_CHAIN_EXPRESSION)
            {
                producer_families_attempted += 1;
                let mut proof = self.result(());
                let input = self.first_input(subject);
                let source_target = self.source_target_feature(subject);
                let target = self.reference_referent(subject);
                let standard = self.standard_role(StandardRole::FeatureChainSourceTarget);
                let values = (
                    input.value,
                    source_target.value,
                    target.value,
                    standard.value,
                );
                proof.merge(input);
                proof.merge(source_target);
                proof.merge(target);
                proof.merge(standard);
                let raw = self.required_structural_result(&mut proof, subject);
                if let (Some(input), source_target, Some(target), Some(standard), Some(raw)) =
                    (values.0, values.1, values.2, values.3, raw)
                {
                    if proof.completeness == Completeness::Complete {
                        graph.expression_chain(
                            subject,
                            input,
                            source_target,
                            (target, standard),
                            raw,
                            &proof.canonical_dependencies,
                        );
                    }
                } else {
                    proof.problem(Completeness::Incomplete, "KQ_FEATURE_CHAIN_STRUCTURE", subject,
                        "The chain expression requires a source input, target Feature and owned result");
                }
                producer_evaluations.push((
                    subject,
                    ProducerFamily::FeatureChainExpression.id(),
                    proof.completeness,
                ));
                production.merge(proof);
            }
            if self.is(subject, c::FEATURE_REFERENCE_EXPRESSION) {
                producer_families_attempted += 1;
                if stratum == ResultStructureStratum::Structural {
                    deferred_bindings.insert(subject);
                } else {
                    let mut proof = self.result(());
                    let referent = self.reference_referent(subject);
                    let target = referent.value;
                    proof.merge(referent);
                    let raw = self.required_structural_result(&mut proof, subject);
                    if let (Some(target), Some(raw)) = (target, raw) {
                        let context = self.reference_binding_context(subject, target, raw);
                        let domain = context.value;
                        proof.merge(context);
                        if proof.completeness == Completeness::Complete {
                            production.value.push(graph.binding(
                                subject,
                                [target, raw],
                                domain,
                                ImpliedBindingRole::FeatureReferenceResult,
                                &proof.canonical_dependencies,
                            ));
                        }
                    }
                    producer_evaluations.push((
                        subject,
                        ProducerFamily::FeatureReferenceExpression.id(),
                        proof.completeness,
                    ));
                    production.merge(proof);
                }
            }
            if self.is(subject, c::EXPRESSION) || self.is(subject, c::FUNCTION) {
                producer_families_attempted += 1;
                let mut proof = self.result(());
                let members = self.memberships(subject);
                for &membership in &members.value {
                    if !self.is(membership, c::RESULT_EXPRESSION_MEMBERSHIP) {
                        continue;
                    }
                    let member = self.member(membership);
                    let nested = member.value;
                    proof.merge(member);
                    if let Some(nested) = nested {
                        let outer = self.required_structural_result(&mut proof, subject);
                        let inner = self.required_structural_result(&mut proof, nested);
                        if let (Some(outer), Some(inner)) = (outer, inner)
                            && proof.completeness == Completeness::Complete
                        {
                            let (rule, role) = if self.is(subject, c::FUNCTION) {
                                (
                                    ResultDomainRule::FunctionResult,
                                    ImpliedBindingRole::FunctionResult,
                                )
                            } else {
                                (
                                    ResultDomainRule::ExpressionResult,
                                    ImpliedBindingRole::ExpressionResult,
                                )
                            };
                            let target = if profile.corrects_result_domains() {
                                let contextual = graph.contextual(
                                    subject,
                                    nested,
                                    inner,
                                    rule,
                                    &proof.canonical_dependencies,
                                );
                                let id = contextual.feature;
                                contextual_results.push(contextual);
                                id
                            } else {
                                inner
                            };
                            production.value.push(graph.binding(
                                subject,
                                [outer, target],
                                Some(subject),
                                role,
                                &proof.canonical_dependencies,
                            ));
                        }
                    }
                }
                proof.merge(members);
                producer_evaluations.push((
                    subject,
                    ProducerFamily::ExpressionResult.id(),
                    proof.completeness,
                ));
                production.merge(proof);
            }
            if self.is(subject, c::FEATURE) {
                producer_families_attempted += 1;
                let mut proof = self.result(());
                let owned = self.owned_relationships(subject);
                let undirected = self
                    .read_value(&mut proof, subject, p::FEATURE_DIRECTION)
                    .is_none();
                let all_implied = owned
                    .value
                    .iter()
                    .filter(|r| self.is(**r, c::SPECIALIZATION))
                    .all(|r| {
                        matches!(
                            self.read_value(&mut proof, *r, p::RELATIONSHIP_IS_IMPLIED),
                            Some(Value::Boolean(true))
                        )
                    });
                for &value in &owned.value {
                    if !self.is(value, c::FEATURE_VALUE) {
                        continue;
                    }
                    let nondefault = matches!(
                        self.read_value(&mut proof, value, p::FEATURE_VALUE_IS_DEFAULT),
                        Some(Value::Boolean(false))
                    );
                    let initial = matches!(
                        self.read_value(&mut proof, value, p::FEATURE_VALUE_IS_INITIAL),
                        Some(Value::Boolean(true))
                    );
                    let member = self.member(value);
                    let expression = member.value;
                    proof.merge(member);
                    if let Some(expression) = expression
                        && let Some(raw) = self.required_structural_result(&mut proof, expression)
                        && proof.completeness == Completeness::Complete
                    {
                        let valuation = undirected && all_implied;
                        let rule = if valuation {
                            ResultDomainRule::FeatureValuation
                        } else {
                            ResultDomainRule::FeatureValueBinding
                        };
                        let contextual =
                            if nondefault || (valuation && profile.corrects_result_domains()) {
                                let contextual = graph.contextual(
                                    subject,
                                    expression,
                                    raw,
                                    rule,
                                    &proof.canonical_dependencies,
                                );
                                let id = contextual.feature;
                                contextual_results.push(contextual);
                                Some(id)
                            } else {
                                None
                            };
                        if valuation {
                            let target = if profile.corrects_result_domains() {
                                contextual.expect("contextual valuation")
                            } else {
                                raw
                            };
                            graph.subset(
                                subject,
                                subject,
                                target,
                                ResultDomainRule::FeatureValuation.constraint(),
                                &proof.canonical_dependencies,
                            );
                        }
                        if nondefault {
                            if stratum == ResultStructureStratum::Structural {
                                // The contextual chain and valuation Subsetting
                                // above are structural prerequisites, not delayed.
                                deferred_bindings.insert(subject);
                            } else {
                                let domains = self.featuring_types(subject);
                                if initial && profile.supports_publication_producers() {
                                    let that = self.standard_role(StandardRole::ThingsThat);
                                    let start =
                                        self.standard_role(StandardRole::OccurrenceStartShot);
                                    let anchors = (that.value, start.value);
                                    proof.merge(that);
                                    proof.merge(start);
                                    if let (Some(that), Some(start)) = anchors {
                                        let context = graph.initial_value_context(that, start);
                                        production.value.push(graph.binding(
                                            subject,
                                            [subject, contextual.expect("value chain")],
                                            Some(context),
                                            ImpliedBindingRole::FeatureValue,
                                            &proof.canonical_dependencies,
                                        ));
                                    }
                                } else if initial && profile == BaselineProfile::OPERATIONAL_V7 {
                                    let mut targets = vec![];
                                    for segments in [
                                        ["Base", "things", "that"],
                                        ["Occurrences", "Occurrence", "startShot"],
                                    ] {
                                        let resolved = self.resolve_reference(
                                            subject,
                                            &QualifiedName {
                                                absolute: true,
                                                segments: segments
                                                    .into_iter()
                                                    .map(str::to_owned)
                                                    .collect(),
                                            },
                                            c::FEATURE,
                                        );
                                        if let Resolution::Resolved(target) = resolved.value {
                                            targets.push(target);
                                        }
                                        proof.merge(resolved);
                                    }
                                    if let [that, start] = targets.as_slice() {
                                        if proof.completeness == Completeness::Complete {
                                            let context =
                                                graph.initial_value_context(*that, *start);
                                            production.value.push(graph.binding(
                                                subject,
                                                [subject, contextual.expect("value chain")],
                                                Some(context),
                                                ImpliedBindingRole::FeatureValue,
                                                &proof.canonical_dependencies,
                                            ));
                                        }
                                    } else {
                                        proof.problem(Completeness::Incomplete,"KQ_INITIAL_VALUE_CONTEXT",value,"Initial binding requires exact canonical Base::things::that and Occurrences::Occurrence::startShot");
                                    }
                                } else if initial {
                                    proof.problem(
                                        Completeness::Incomplete,
                                        "KQ_INITIAL_VALUE_CONTEXT",
                                        value,
                                        "Initial binding requires canonical that.startShot context",
                                    );
                                } else if domains.value.len() <= 1
                                    && domains.completeness == Completeness::Complete
                                {
                                    production.value.push(graph.binding(
                                        subject,
                                        [subject, contextual.expect("value chain")],
                                        domains.value.first().copied(),
                                        ImpliedBindingRole::FeatureValue,
                                        &proof.canonical_dependencies,
                                    ));
                                } else {
                                    proof.problem(
                                        Completeness::Incomplete,
                                        "KQ_VALUE_CONTEXT",
                                        value,
                                        "Value binding requires complete featuring domains",
                                    );
                                }
                                proof.merge(domains);
                            }
                        }
                    }
                }
                proof.merge(owned);
                producer_evaluations.push((
                    subject,
                    ProducerFamily::FeatureValue.id(),
                    proof.completeness,
                ));
                production.merge(proof);
            }
            if self.is(subject, c::INDEX_EXPRESSION) || self.is(subject, c::SELECT_EXPRESSION) {
                producer_families_attempted += 1;
                let mut proof = self.result(());
                if let Some(argument) = self.argument_expression(&mut proof, subject) {
                    let raw = self.required_structural_result(&mut proof, argument);
                    let result = self.required_structural_result(&mut proof, subject);
                    if let (Some(raw), Some(result)) = (raw, result) {
                        let index = self.is(subject, c::INDEX_EXPRESSION);
                        let applies = if index && profile.supports_publication_producers() {
                            let array = self.standard_role(StandardRole::CollectionsArray);
                            let mut applies = false;
                            if let Some(array) = array.value {
                                let supers = self.all_supertypes(raw);
                                applies = !supers.value.contains(&array);
                                proof.merge(supers);
                            }
                            proof.merge(array);
                            applies
                        } else if index {
                            // Historical profiles retain their original path lookup.
                            // The exact Array guard is bound through ordinary global resolution.
                            let resolution = self.resolve_reference(
                                subject,
                                &QualifiedName {
                                    absolute: true,
                                    segments: vec!["Collections".into(), "Array".into()],
                                },
                                c::DATA_TYPE,
                            );
                            let mut applies = false;
                            if let Resolution::Resolved(candidate) = resolution.value {
                                let supers = self.all_supertypes(raw);
                                applies = !supers.value.contains(&candidate);
                                proof.merge(supers);
                            } else {
                                proof.problem(
                                    Completeness::Incomplete,
                                    "KQ_INDEX_ARRAY_GUARD",
                                    subject,
                                    "Requires a unique canonical Collections::Array",
                                );
                            }
                            proof.merge(resolution);
                            applies
                        } else {
                            true
                        };
                        if applies && proof.completeness == Completeness::Complete {
                            let rule = if index {
                                ResultDomainRule::IndexResult
                            } else {
                                ResultDomainRule::SelectResult
                            };
                            let target = if profile.corrects_result_domains() {
                                let contextual = graph.contextual(
                                    subject,
                                    argument,
                                    raw,
                                    rule,
                                    &proof.canonical_dependencies,
                                );
                                let id = contextual.feature;
                                contextual_results.push(contextual);
                                id
                            } else {
                                raw
                            };
                            graph.subset(
                                subject,
                                result,
                                target,
                                rule.constraint(),
                                &proof.canonical_dependencies,
                            );
                        }
                    }
                }
                producer_evaluations.push((
                    subject,
                    ProducerFamily::IndexSelectResult.id(),
                    proof.completeness,
                ));
                production.merge(proof);
            }
            graph.finish_subject(&mut production, &mut aggregate);
        }
        let mut production = aggregate;
        // These lists enumerate produced identities, not semantic ownership.
        // V7 normalizes them exactly as merge does so caller/batch order cannot
        // change the complete producer answer. Graph ownership and historical
        // profile output sequences are untouched.
        if profile.corrects_owned_cross_feature() {
            production.value.sort();
            production.value.dedup();
            contextual_results.sort_by_key(|result| result.feature);
            contextual_results.dedup();
        }
        ResultStructurePlan {
            graph,
            producer_families_attempted,
            producer_evaluations,
            deferred_bindings,
            context: self.context().clone(),
            production,
            contextual_results,
        }
    }
}
