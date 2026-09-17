use crate::{contract::claim, *};
use agq_kerml::{TypedView, ViewError, classes as c, properties as p, views};
use agq_kernel::{
    ElementId, MetaclassId, ModelView, PropertyId,
    provenance::{Dependency, FactKey, Origin},
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// An immutable evaluator; per-invocation traversal state is always discardable.
pub struct KerMlQueries<'m> {
    pub(crate) context: SemanticContext<'m>,
}

impl<'m> KerMlQueries<'m> {
    pub fn new(context: SemanticContext<'m>) -> Self {
        Self { context }
    }
    pub fn context(&self) -> &SemanticContextId {
        self.context.id()
    }
    pub(crate) fn model(&self) -> &'m ModelView {
        self.context.model
    }
    pub(crate) fn result<T>(&self, value: T) -> QueryResult<T> {
        QueryResult::new(self.context(), value)
    }

    pub(crate) fn checked<V: TypedView<'m>, T>(
        &self,
        out: &mut QueryResult<T>,
        id: ElementId,
    ) -> Option<V> {
        out.search_dependencies
            .insert(SearchDependency::Element(id));
        self.fact(out, FactKey::Element(id));
        self.accept(out, id, V::try_new(id, self.model()))
    }
    pub(crate) fn accept<T, U>(
        &self,
        out: &mut QueryResult<T>,
        id: ElementId,
        read: Result<U, ViewError>,
    ) -> Option<U> {
        match read {
            Ok(value) => Some(value),
            Err(error) => {
                let status = if matches!(
                    error,
                    ViewError::NotComputed { .. } | ViewError::UnsupportedAssociationStorage(_)
                ) {
                    Completeness::Incomplete
                } else {
                    Completeness::Invalid
                };
                out.problem(status, "KQ_EVIDENCE", id, format!("{error:?}"));
                None
            }
        }
    }
    /// Read the effective fact identity, including inverse association storage.
    pub(crate) fn property<T>(
        &self,
        out: &mut QueryResult<T>,
        element: ElementId,
        property: PropertyId,
    ) -> Vec<Evidence> {
        let Some(record) = self.model().element(element) else {
            return vec![];
        };
        let Some(descriptor) = self
            .model()
            .registry()
            .resolve_property(record.metaclass(), property)
            .ok()
            .flatten()
        else {
            return vec![];
        };
        let property = descriptor.id;
        let search = SearchDependency::PropertySet { element, property };
        out.search_dependencies.insert(search.clone());
        let mut evidence = vec![Evidence::Search(search)];
        if let Some(storage) = self
            .model()
            .registry()
            .inverse_storage(property)
            .ok()
            .flatten()
        {
            let search = SearchDependency::Incoming { target: element };
            out.search_dependencies.insert(search.clone());
            evidence.push(Evidence::Search(search));
            for incoming in self
                .model()
                .incoming(element)
                .filter(|r| r.property == storage)
            {
                let fact = FactKey::Property {
                    element: incoming.source,
                    property: storage,
                };
                self.fact(out, fact);
                evidence.push(Evidence::Fact(fact));
            }
        } else if record.slot(property).is_some() {
            let fact = FactKey::Property { element, property };
            self.fact(out, fact);
            evidence.push(Evidence::Fact(fact));
        }
        evidence
    }
    /// Iterative expansion preserves the kernel's declared/derived distinction.
    pub(crate) fn fact<T>(&self, out: &mut QueryResult<T>, key: FactKey) {
        let mut queue = vec![key];
        while let Some(key) = queue.pop() {
            if out.positive_dependencies.contains(&key) {
                continue;
            }
            let origin = match key {
                FactKey::Element(id) => self.model().element(id).map(|e| e.origin()),
                FactKey::Property { element, property } => self
                    .model()
                    .element(element)
                    .and_then(|e| e.slot(property))
                    .map(|s| s.origin()),
            };
            if let Some(origin) = origin {
                out.positive_dependencies.insert(key);
                out.fact_origins.insert(key, origin.clone());
                if let Origin::Derived(explanation) = origin {
                    queue.extend(explanation.dependencies.iter().map(|d| match d {
                        Dependency::Declared(f) | Dependency::Derived(f) => *f,
                    }));
                }
            }
        }
    }
    pub(crate) fn is(&self, id: ElementId, class: MetaclassId) -> bool {
        self.model().element(id).is_some_and(|e| {
            self.model()
                .registry()
                .is_subtype(e.metaclass(), class)
                .unwrap_or(false)
        })
    }

    /// Ordered directly owned relationship identities, preserving canonical order.
    pub fn owned_relationships(&self, element: ElementId) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        let Some(view) = self.checked::<views::Element, _>(&mut out, element) else {
            return out;
        };
        let evidence = self.property(&mut out, element, p::ELEMENT_OWNED_RELATIONSHIP);
        if let Some(values) = self.accept(&mut out, element, view.owned_relationship()) {
            for target in values.into_iter().flat_map(|v| v.iter()) {
                self.fact(&mut out, FactKey::Element(target));
                out.value.push(target);
                out.prove(
                    QueryKind::OwnedRelationships,
                    element,
                    target,
                    Rule::StoredRelationship,
                    evidence.clone(),
                );
            }
        }
        out
    }
    pub fn owning_relationship(&self, element: ElementId) -> QueryResult<Option<ElementId>> {
        let mut out = self.result(None);
        let Some(view) = self.checked::<views::Element, _>(&mut out, element) else {
            return out;
        };
        let evidence = self.property(&mut out, element, p::ELEMENT_OWNING_RELATIONSHIP);
        out.value = self
            .accept(&mut out, element, view.owning_relationship())
            .flatten();
        if let Some(target) = out.value {
            out.prove(
                QueryKind::OwningRelationship,
                element,
                target,
                Rule::InverseAssociation,
                evidence,
            );
        }
        out
    }
    pub fn owning_related_element(
        &self,
        relationship: ElementId,
    ) -> QueryResult<Option<ElementId>> {
        let mut out = self.result(None);
        let Some(view) = self.checked::<views::Relationship, _>(&mut out, relationship) else {
            return out;
        };
        let evidence = self.property(
            &mut out,
            relationship,
            p::RELATIONSHIP_OWNING_RELATED_ELEMENT,
        );
        out.value = self
            .accept(&mut out, relationship, view.owning_related_element())
            .flatten();
        if let Some(target) = out.value {
            out.prove(
                QueryKind::OwningRelatedElement,
                relationship,
                target,
                Rule::InverseAssociation,
                evidence,
            );
        }
        out
    }
    /// Element::owner = owningRelationship.owningRelatedElement. Illegal ownership
    /// loops are diagnosed, including loops reached beyond the initial element.
    pub fn owner(&self, element: ElementId) -> QueryResult<Option<ElementId>> {
        let mut out = self.result(None);
        let mut seen = BTreeSet::new();
        let mut current = element;
        loop {
            if !seen.insert(current) {
                out.problem(
                    Completeness::Invalid,
                    "KQ_OWNERSHIP_CYCLE",
                    current,
                    "cyclic KerML ownership",
                );
                out.value = None;
                break;
            }
            let relationship = self.owning_relationship(current);
            let id = relationship.value;
            out.merge(relationship);
            let Some(id) = id else { break };
            let owner = self.owning_related_element(id);
            let target = owner.value;
            out.merge(owner);
            let Some(target) = target else {
                if self.is(id, c::MEMBERSHIP) {
                    out.problem(
                        Completeness::Invalid,
                        "KQ_MEMBERSHIP_NAMESPACE",
                        id,
                        "membership requires an owning namespace",
                    );
                }
                break;
            };
            if self.is(id, c::MEMBERSHIP) && !self.is(target, c::NAMESPACE) {
                out.problem(
                    Completeness::Invalid,
                    "KQ_MEMBERSHIP_NAMESPACE",
                    id,
                    "membership owner must be a namespace",
                );
                out.value = None;
                break;
            }
            out.prove(
                QueryKind::Owner,
                current,
                target,
                Rule::ElementOwner,
                [
                    claim(QueryKind::OwningRelationship, current, id),
                    claim(QueryKind::OwningRelatedElement, id, target),
                ],
            );
            if current == element {
                out.value = Some(target);
            }
            current = target;
        }
        out
    }

    /// Direct owned memberships only; this is not Namespace::membership (imports/inheritance).
    pub fn memberships(&self, namespace: ElementId) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        if self
            .checked::<views::Namespace, _>(&mut out, namespace)
            .is_none()
        {
            return out;
        }
        out.search_dependencies
            .insert(SearchDependency::NamespaceMembers { namespace });
        let owned = self.owned_relationships(namespace);
        for &id in &owned.value {
            if self.is(id, c::MEMBERSHIP) {
                out.value.push(id);
                out.prove(
                    QueryKind::Memberships,
                    namespace,
                    id,
                    Rule::OwnedMembership,
                    [
                        claim(QueryKind::OwnedRelationships, namespace, id),
                        Evidence::Fact(FactKey::Element(id)),
                    ],
                );
            }
        }
        out.merge(owned);
        out
    }
    /// Member endpoint; owning membership endpoints are derived from canonical ownership links.
    pub fn member(&self, membership: ElementId) -> QueryResult<Option<ElementId>> {
        let mut out = self.result(None);
        let Some(view) = self.checked::<views::Membership, _>(&mut out, membership) else {
            return out;
        };
        let evidence;
        if self.is(membership, c::OWNING_MEMBERSHIP) {
            evidence = self.property(&mut out, membership, p::RELATIONSHIP_OWNED_RELATED_ELEMENT);
            let values = self.accept(&mut out, membership, view.owned_related_element());
            let ids: Vec<_> = values
                .flatten()
                .into_iter()
                .flat_map(|v| v.iter())
                .collect();
            if ids.len() != 1 {
                out.problem(
                    Completeness::Invalid,
                    "KQ_MEMBERSHIP_ARITY",
                    membership,
                    "owning membership requires exactly one owned member",
                );
            } else {
                out.value = Some(ids[0]);
            }
        } else {
            evidence = self.property(&mut out, membership, p::MEMBERSHIP_MEMBER_ELEMENT);
            out.value = self.accept(&mut out, membership, view.member_element());
        }
        if let Some(id) = out.value {
            if self.is(membership, c::FEATURE_MEMBERSHIP) && !self.is(id, c::FEATURE) {
                out.problem(
                    Completeness::Invalid,
                    "KQ_MEMBER_TYPE",
                    membership,
                    "FeatureMembership must own a Feature",
                );
                out.value = None;
            } else {
                self.fact(&mut out, FactKey::Element(id));
                out.prove(
                    QueryKind::Member,
                    membership,
                    id,
                    Rule::MembershipEndpoint,
                    evidence,
                );
            }
        }
        out
    }
    /// Exact declared full-name lookup among directly owned memberships. No imports,
    /// inherited names, short-name resolution, shadowing or lexical scope is implied.
    pub fn lookup_declared_member(
        &self,
        namespace: ElementId,
        name: &str,
    ) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        let memberships = self.memberships(namespace);
        for &id in &memberships.value {
            let member = self.member(id);
            if let Some(target) = member.value {
                let (actual, mut evidence) = if self.is(id, c::OWNING_MEMBERSHIP) {
                    let evidence = self.property(&mut out, target, p::ELEMENT_DECLARED_NAME);
                    let view =
                        views::Element::try_new(target, self.model()).expect("validated endpoint");
                    (
                        self.accept(&mut out, target, view.declared_name())
                            .flatten(),
                        evidence,
                    )
                } else {
                    let evidence = self.property(&mut out, id, p::MEMBERSHIP_MEMBER_NAME);
                    let view =
                        views::Membership::try_new(id, self.model()).expect("checked membership");
                    (
                        self.accept(&mut out, id, view.member_name()).flatten(),
                        evidence,
                    )
                };
                if actual == Some(name) {
                    out.value.push(target);
                    evidence.extend([
                        claim(QueryKind::Memberships, namespace, id),
                        claim(QueryKind::Member, id, target),
                    ]);
                    out.prove(
                        QueryKind::LookupDeclaredMember,
                        namespace,
                        target,
                        Rule::DeclaredMemberName,
                        evidence,
                    );
                }
            }
            out.merge(member);
        }
        out.merge(memberships);
        out.value.sort();
        out.value.dedup();
        out
    }
    pub fn direct_features(&self, ty: ElementId) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        if self.checked::<views::Type, _>(&mut out, ty).is_none() {
            return out;
        }
        let memberships = self.memberships(ty);
        for &id in &memberships.value {
            if !self.is(id, c::FEATURE_MEMBERSHIP) {
                continue;
            }
            let member = self.member(id);
            if let Some(feature) = member.value {
                out.value.push(feature);
                out.prove(
                    QueryKind::DirectFeatures,
                    ty,
                    feature,
                    Rule::OwnedFeature,
                    [
                        claim(QueryKind::Memberships, ty, id),
                        claim(QueryKind::Member, id, feature),
                    ],
                );
            }
            out.merge(member);
        }
        out.merge(memberships);
        out
    }

    /// All explicit Specialization relationships with this specific endpoint,
    /// including FeatureTyping/Subsetting/Redefinition through generated upcasts.
    pub fn direct_specializations(&self, specific: ElementId) -> QueryResult<Vec<ElementId>> {
        self.targets(specific, QueryKind::DirectSpecializations)
    }
    /// Direct FeatureTyping endpoints only; not the complete derived Feature::type.
    pub fn direct_feature_types(&self, feature: ElementId) -> QueryResult<Vec<ElementId>> {
        self.targets(feature, QueryKind::DirectFeatureTypes)
    }
    pub fn subsetted_features(&self, feature: ElementId) -> QueryResult<Vec<ElementId>> {
        self.targets(feature, QueryKind::SubsettedFeatures)
    }
    pub fn redefined_features(&self, feature: ElementId) -> QueryResult<Vec<ElementId>> {
        self.targets(feature, QueryKind::RedefinedFeatures)
    }

    fn targets(&self, source: ElementId, kind: QueryKind) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        let (class, source_property, target_property, rule) = match kind {
            QueryKind::DirectSpecializations => (
                c::SPECIALIZATION,
                p::SPECIALIZATION_SPECIFIC,
                p::SPECIALIZATION_GENERAL,
                Rule::Specialization,
            ),
            QueryKind::DirectFeatureTypes => (
                c::FEATURE_TYPING,
                p::FEATURE_TYPING_TYPED_FEATURE,
                p::FEATURE_TYPING_TYPE,
                Rule::FeatureTyping,
            ),
            QueryKind::SubsettedFeatures => (
                c::SUBSETTING,
                p::SUBSETTING_SUBSETTING_FEATURE,
                p::SUBSETTING_SUBSETTED_FEATURE,
                Rule::Subsetting,
            ),
            QueryKind::RedefinedFeatures => (
                c::REDEFINITION,
                p::REDEFINITION_REDEFINING_FEATURE,
                p::REDEFINITION_REDEFINED_FEATURE,
                Rule::Redefinition,
            ),
            _ => unreachable!(),
        };
        if kind == QueryKind::DirectSpecializations {
            if self.checked::<views::Type, _>(&mut out, source).is_none() {
                return out;
            }
        } else if self
            .checked::<views::Feature, _>(&mut out, source)
            .is_none()
        {
            return out;
        }
        out.search_dependencies
            .insert(SearchDependency::Incoming { target: source });
        // Uses the kernel incoming index, not a population scan per queried type.
        let candidates: BTreeSet<_> = self
            .model()
            .incoming(source)
            .filter(|r| self.is(r.source, class))
            .map(|r| r.source)
            .collect();
        for id in candidates {
            let Some(view) = self.checked::<views::Specialization, _>(&mut out, id) else {
                continue;
            };
            let mut evidence = self.property(&mut out, id, source_property);
            if self.accept(&mut out, id, view.specific()) != Some(source) {
                continue;
            }
            evidence.extend(self.property(&mut out, id, p::RELATIONSHIP_IS_IMPLIED));
            let implied = self.accept(&mut out, id, view.is_implied());
            if kind == QueryKind::DirectSpecializations
                && self.context.id.options.exclude_implied
                && implied == Some(true)
            {
                continue;
            }
            evidence.extend(self.property(&mut out, id, target_property));
            if let Some(target) = self.accept(&mut out, id, view.general()) {
                self.fact(&mut out, FactKey::Element(target));
                evidence.extend([
                    Evidence::Fact(FactKey::Element(id)),
                    Evidence::Fact(FactKey::Element(source)),
                    Evidence::Fact(FactKey::Element(target)),
                ]);
                out.value.push(target);
                out.prove(kind, source, target, rule, evidence);
            }
        }
        out.value.sort();
        out.value.dedup();
        out
    }
    /// Non-reflexive reachability over explicit specialization edges. Cyclic graphs
    /// terminate; specialization cycles alone are not a KerML validation error.
    pub fn all_specializations(&self, specific: ElementId) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        let mut queue = VecDeque::from([specific]);
        let mut depths = BTreeMap::from([(specific, 0usize)]);
        while let Some(current) = queue.pop_front() {
            let direct = self.direct_specializations(current);
            let depth = depths[&current];
            for &general in &direct.value {
                if let std::collections::btree_map::Entry::Vacant(entry) = depths.entry(general) {
                    entry.insert(depth + 1);
                    queue.push_back(general);
                    out.value.push(general);
                }
                if general != specific && depths[&general] > depths[&current] {
                    let mut premises =
                        vec![claim(QueryKind::DirectSpecializations, current, general)];
                    if current != specific {
                        premises.push(claim(QueryKind::AllSpecializations, specific, current));
                    }
                    out.prove(
                        QueryKind::AllSpecializations,
                        specific,
                        general,
                        Rule::TransitiveSpecialization,
                        premises,
                    );
                }
            }
            out.merge(direct);
        }
        out.value.sort();
        out
    }
}
