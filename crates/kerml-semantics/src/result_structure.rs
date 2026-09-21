//! Canonical implied result structure. Raw result identities are never replaced.
use crate::*;
use agq_kerml::{BaselineProfile, classes as c, properties as p};
use agq_kernel::{
    DerivationKey, ElementId, MetaclassId, OutputKey, PropertyId, RuleId, Snapshot,
    derived::{DerivationBuilder, DerivationError, DerivedOverlay},
    provenance::{Dependency, Explanation as KernelExplanation, FactKey},
    value::{SlotValue, Value},
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

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
    use super::Graph;

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
            attachments: BTreeMap::new(),
            unattached_contextual: BTreeSet::new(),
        };
        let generated = graph.create(
            graph.key("evidence-regression", id(1), "feature", &[]),
            c::FEATURE,
            &BTreeSet::from([projected]),
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
    dependencies: BTreeSet<Dependency>,
}

struct Graph<'a> {
    model: &'a agq_kernel::ModelView,
    profile: BaselineProfile,
    records: BTreeMap<ElementId, ImpliedRecord>,
    attachments: BTreeMap<ElementId, Vec<ElementId>>,
    unattached_contextual: BTreeSet<ElementId>,
}
impl<'a> Graph<'a> {
    fn initial_value_context(&mut self, that: ElementId, start: ElementId) -> ElementId {
        let inputs = [that, start];
        let deps = BTreeSet::from([FactKey::Element(that), FactKey::Element(start)]);
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
        dependencies: &BTreeSet<FactKey>,
    ) -> ElementId {
        let id = key.element_id();
        if self.records.contains_key(&id) {
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
                .is_legal(class, property)
                .expect("KerML descriptor")
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
        self.records.insert(
            id,
            ImpliedRecord {
                key,
                class,
                slots,
                dependencies: dependencies
                    .iter()
                    .copied()
                    .flat_map(|fact| {
                        // Association navigation is a read projection, not a
                        // separately declared slot. Retain its canonical links
                        // as kernel evidence instead of inventing a slot fact.
                        if let FactKey::Property { element, property } = fact
                            && let Some(slot) = self.model.navigation_slot(element, property)
                            && let agq_kernel::provenance::Origin::AssociationOccurrences(links) =
                                slot.origin()
                        {
                            return links
                                .iter()
                                .copied()
                                .map(|link| {
                                    Dependency::Declared(FactKey::AssociationOccurrence(link))
                                })
                                .collect::<Vec<_>>();
                        }
                        vec![Dependency::Declared(fact)]
                    })
                    .collect(),
            },
        );
        id
    }
    fn set(&mut self, id: ElementId, property: PropertyId, value: Value) {
        self.records
            .get_mut(&id)
            .expect("implied record")
            .slots
            .insert(property, SlotValue::Scalar(value));
    }
    fn own(&mut self, owner: ElementId, relationship: ElementId) {
        if let Some(record) = self.records.get_mut(&owner) {
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
            let values = self.attachments.entry(owner).or_default();
            if !values.contains(&relationship) {
                values.push(relationship);
            }
        }
    }
    fn membership(
        &mut self,
        owner: ElementId,
        target: ElementId,
        class: MetaclassId,
        key: DerivationKey,
        deps: &BTreeSet<FactKey>,
    ) {
        let member = self.create(key, class, deps);
        self.records
            .get_mut(&member)
            .expect("membership")
            .slots
            .insert(
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
        deps: &BTreeSet<FactKey>,
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
        deps: &BTreeSet<FactKey>,
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
        self.records
            .get_mut(&relationship)
            .expect("target relationship")
            .slots
            .insert(
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
        deps: &BTreeSet<FactKey>,
    ) -> ElementId {
        let rule = role.constraint();
        let key = self.key(rule, owner, "binding", &endpoints);
        let binding = self.create(key, c::BINDING_CONNECTOR, deps);
        let membership = if matches!(
            role,
            ImpliedBindingRole::ExpressionResult | ImpliedBindingRole::FunctionResult
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
        let mut builder = DerivationBuilder::new(snapshot.clone());
        for record in self.records.into_values() {
            builder.element(record.key, record.class, record.slots, record.dependencies);
        }
        for (owner, additions) in self.attachments {
            builder.extend_ordered_references(
                owner,
                p::ELEMENT_OWNED_RELATIONSHIP,
                additions,
                KernelExplanation {
                    rule: rule_id(self.profile, "structural-result-ownership/1"),
                    dependencies: BTreeSet::new(),
                },
            );
        }
        builder.build()
    }
}

/// An immutable canonical overlay and the completeness of its producers.
/// An incomplete derivation is never an accepted semantic publication.
pub struct ResultStructure {
    pub overlay: DerivedOverlay,
    pub production: QueryResult<Vec<ElementId>>,
    pub contextual_results: Vec<ContextualResult>,
}

/// A bounded producer plan over one immutable semantic context. This is a
/// proposed derivation, not canonical storage or an accepted publication.
/// Construction inputs can be audited without inventing a valid Snapshot.
pub struct ResultStructurePlan<'m> {
    graph: Graph<'m>,
    /// Exact input identity, including unresolved construction obligations.
    pub context: SemanticContextId,
    pub production: QueryResult<Vec<ElementId>>,
    pub contextual_results: Vec<ContextualResult>,
}
impl ResultStructurePlan<'_> {
    /// Stable implied identities, useful for independent batch comparisons.
    pub fn planned_elements(&self) -> impl Iterator<Item = ElementId> + '_ {
        self.graph.records.keys().copied()
    }
    /// Merge disjoint subject batches from the identical context. Duplicate
    /// facts must agree exactly, including provenance. Conflicting ownership
    /// sequences are rejected before mutation; a batch order cannot pick one.
    pub fn merge(&mut self, other: Self) -> Result<(), DerivationError> {
        if self.context != other.context || !std::ptr::eq(self.graph.model, other.graph.model) {
            return Err(DerivationError::InputContextMismatch);
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
        for (&owner, additions) in &other.graph.attachments {
            if self
                .graph
                .attachments
                .get(&owner)
                .is_some_and(|existing| existing != additions)
            {
                return Err(DerivationError::InvalidCollectionExtension {
                    element: owner,
                    property: p::ELEMENT_OWNED_RELATIONSHIP,
                });
            }
        }
        self.graph.records.extend(other.graph.records);
        self.graph.attachments.extend(other.graph.attachments);
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
}

impl<'m> KerMlQueries<'m> {
    fn required_structural_result<T>(
        &self,
        out: &mut QueryResult<T>,
        expression: ElementId,
    ) -> Option<ElementId> {
        let result = self.expression_result(out, expression);
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
        let profile = self.context().options.baseline_profile;
        let mut graph = Graph {
            model: self.model(),
            profile,
            records: BTreeMap::new(),
            attachments: BTreeMap::new(),
            unattached_contextual: BTreeSet::new(),
        };
        let mut production = self.result(vec![]);
        let mut contextual_results = vec![];
        for subject in subjects {
            if self.model().element(subject).is_none() {
                production.problem(
                    Completeness::Invalid,
                    "KQ_PRODUCER_SUBJECT",
                    subject,
                    "Missing producer subject",
                );
                continue;
            }
            if self.is(subject, c::FEATURE_REFERENCE_EXPRESSION) {
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
                            &proof.positive_dependencies,
                        ));
                    }
                }
                production.merge(proof);
            }
            if self.is(subject, c::EXPRESSION) || self.is(subject, c::FUNCTION) {
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
                                    &proof.positive_dependencies,
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
                                &proof.positive_dependencies,
                            ));
                        }
                    }
                }
                proof.merge(members);
                production.merge(proof);
            }
            if self.is(subject, c::FEATURE) {
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
                                    &proof.positive_dependencies,
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
                                &proof.positive_dependencies,
                            );
                        }
                        if nondefault {
                            let domains = self.featuring_types(subject);
                            if initial && profile == BaselineProfile::OPERATIONAL_V7 {
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
                                        let context = graph.initial_value_context(*that, *start);
                                        production.value.push(graph.binding(
                                            subject,
                                            [subject, contextual.expect("value chain")],
                                            Some(context),
                                            ImpliedBindingRole::FeatureValue,
                                            &proof.positive_dependencies,
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
                                    &proof.positive_dependencies,
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
                proof.merge(owned);
                production.merge(proof);
            }
            if self.is(subject, c::INDEX_EXPRESSION) || self.is(subject, c::SELECT_EXPRESSION) {
                let mut proof = self.result(());
                if let Some(argument) = self.argument_expression(&mut proof, subject) {
                    let raw = self.required_structural_result(&mut proof, argument);
                    let result = self.required_structural_result(&mut proof, subject);
                    if let (Some(raw), Some(result)) = (raw, result) {
                        let index = self.is(subject, c::INDEX_EXPRESSION);
                        let applies = if index {
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
                                    &proof.positive_dependencies,
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
                                &proof.positive_dependencies,
                            );
                        }
                    }
                }
                production.merge(proof);
            }
        }
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
            context: self.context().clone(),
            production,
            contextual_results,
        }
    }
}
