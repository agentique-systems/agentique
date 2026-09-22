use crate::{contract::claim, *};
use agq_kerml::{TypedView, ViewError, classes as c, properties as p, views};
use agq_kernel::{
    ElementId, MetaclassId, ModelView, PropertyId,
    provenance::{Dependency, FactKey, Origin},
    value::Value,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Arc, Mutex};

#[cfg(test)]
#[path = "../tests/unit/shared_query_searches.rs"]
mod shared_search_tests;

/// An immutable evaluator; per-invocation traversal state is always discardable.
pub struct KerMlQueries<'m> {
    producer_evidence: bool,
    pub(crate) context: SemanticContext<'m>,
    pub(crate) namespace_cache: crate::namespaces::NamespaceCache,
    pub(crate) effective_names_cache: Mutex<BTreeMap<ElementId, Arc<QueryResult<EffectiveNames>>>>,
    pub(crate) source_role_cache: Mutex<crate::relationship_sources::SourceRoleCache>,
    pub(crate) library_cache: std::sync::Mutex<BTreeMap<ElementId, QueryResult<Vec<ElementId>>>>,
    pub(crate) result_cache: std::sync::Mutex<BTreeMap<ElementId, QueryResult<Vec<ElementId>>>>,
    // Scoped to this immutable model/context, like the other query caches.
    // Only origin values are shared; every answer still expands its own full
    // dependency closure and records every applicable computation search.
    origin_cache: Mutex<BTreeMap<FactKey, Option<Arc<Origin>>>>,
    declared_origin_cache: Mutex<BTreeMap<FactKey, Option<Arc<Origin>>>>,
}

impl<'m> KerMlQueries<'m> {
    pub fn new(context: SemanticContext<'m>) -> Self {
        Self {
            producer_evidence: false,
            context,
            namespace_cache: Default::default(),
            effective_names_cache: Default::default(),
            source_role_cache: Default::default(),
            library_cache: Default::default(),
            result_cache: Default::default(),
            origin_cache: Default::default(),
            declared_origin_cache: Default::default(),
        }
    }
    /// A producer needs immediate canonical proof edges and all search reads,
    /// not the expanded user-facing explanation forest. This mode never escapes
    /// through a public query evaluator or a returned aggregate production proof.
    pub(crate) fn for_production(context: SemanticContext<'m>) -> Self {
        Self {
            producer_evidence: true,
            ..Self::new(context)
        }
    }
    pub fn context(&self) -> &SemanticContextId {
        self.context.id()
    }
    /// Start a fresh bounded query batch over the exact same immutable input.
    /// Validated bindings and context identity are shared; traversal caches and
    /// retained query proofs are released when the previous evaluator is dropped.
    pub fn fork(&self) -> Self {
        Self::new(self.context.fork())
    }
    /// Exact immutable canonical view used by this evaluator. Composed language
    /// layers can borrow it without supplying a second, potentially different graph.
    pub fn model(&self) -> &'m ModelView {
        self.context.model
    }
    /// Prove that every registered producer capable of changing this answer is
    /// closed. Missing evidence remains an explicit dependency and incomplete
    /// result. This query never treats graph quiescence as an absence proof.
    pub fn producer_closure(
        &self,
        subject: ElementId,
        requirement: SemanticClosureRequirement,
    ) -> QueryResult<bool> {
        let mut answer = self.result(false);
        let certificate = self.context.producer_closure();
        let search = SearchDependency::ProducerClosure {
            subject,
            requirement,
            certificate_digest: certificate.map(|certificate| certificate.digest()),
        };
        answer.search_dependencies.insert(search.clone());
        answer.value =
            certificate.is_some_and(|certificate| certificate.is_closed(subject, requirement));
        if answer.value {
            answer.prove(
                QueryKind::ProducerClosure(requirement),
                subject,
                subject,
                Rule::ProducerClosure(requirement),
                [Evidence::Search(search)],
            );
        } else {
            answer.problem(
                Completeness::Incomplete,
                "KQ_PRODUCER_CLOSURE",
                subject,
                format!(
                    "No compatible scheduler certificate closes {requirement:?} for this subject"
                ),
            );
        }
        answer
    }
    /// Observe canonical facts for a composed language query, retaining the same
    /// provenance expansion and negative-read contracts as KerML queries. This
    /// does not derive a value: uncomputed derived properties and failed inputs
    /// remain incomplete/invalid. Optional absent stored facts are observations
    /// of absence, with search dependencies. Property aliases use their effective
    /// canonical storage and association projections.
    pub fn canonical_fact_evidence(&self, fact: FactKey) -> QueryResult<()> {
        use agq_kernel::derived::{PropertyState, StructuralSearch};
        let mut out = self.result(());
        out.search_dependencies
            .insert(SearchDependency::Kernel(StructuralSearch::DescriptorGraph));
        match fact {
            FactKey::Element(element) => {
                out.search_dependencies
                    .insert(SearchDependency::Element(element));
                self.fact(&mut out, fact);
            }
            FactKey::Property { element, property } => {
                if self
                    .checked::<views::Element, _>(&mut out, element)
                    .is_none()
                {
                    return out;
                }
                let record = self.model().element(element).expect("checked element");
                let resolved = self.accept(
                    &mut out,
                    element,
                    self.model()
                        .registry()
                        .resolve_property(record.metaclass(), property)
                        .map_err(ViewError::Registry),
                );
                let Some(descriptor) = resolved.flatten() else {
                    if out.completeness == Completeness::Complete {
                        out.problem(
                            Completeness::Invalid,
                            "KQ_CANONICAL_FACT",
                            element,
                            format!("Property {property} is not applicable to this element"),
                        );
                    }
                    return out;
                };
                let property = descriptor.id;
                self.query_projection_failure(&mut out, element, property);
                if matches!(
                    self.model().property_state(element, property),
                    Ok(PropertyState::NotComputed)
                ) {
                    self.accept::<_, ()>(
                        &mut out,
                        element,
                        Err(ViewError::NotComputed { element, property }),
                    );
                }
            }
            FactKey::AssociationOccurrence(id) => {
                if let Some(link) = self.model().association_occurrence(id) {
                    for &element in link.ends().values() {
                        out.search_dependencies.insert(SearchDependency::Kernel(
                            StructuralSearch::Association {
                                element,
                                association: link.association(),
                            },
                        ));
                    }
                } else {
                    // The kernel has no narrower negative occurrence-identity key.
                    out.search_dependencies
                        .insert(SearchDependency::Kernel(StructuralSearch::Model));
                }
                self.fact(&mut out, fact);
            }
        }
        out
    }
    pub(crate) fn result<T>(&self, value: T) -> QueryResult<T> {
        let mut result = QueryResult::new(self.context(), value);
        result.producer_evidence = self.producer_evidence;
        result
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
                let status = match &error {
                    ViewError::MissingRequired { element, property }
                        if self
                            .context()
                            .construction_obligations
                            .contains(&(*element, *property)) =>
                    {
                        Completeness::Incomplete
                    }
                    ViewError::NotComputed { .. } | ViewError::UnsupportedAssociationStorage(_) => {
                        Completeness::Incomplete
                    }
                    ViewError::ComputationFailure { failure, .. }
                        if matches!(
                            failure.as_ref(),
                            agq_kernel::derived::ComputationFailure::Incomplete { .. }
                        ) =>
                    {
                        Completeness::Incomplete
                    }
                    ViewError::Registry(
                        agq_kernel::metamodel::MetamodelError::PropertyConflict { .. }
                        | agq_kernel::metamodel::MetamodelError::UnsupportedAssociationRedefinition {
                            ..
                        },
                    ) => Completeness::Incomplete,
                    _ => Completeness::Invalid,
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
        if self
            .context()
            .construction_obligations
            .contains(&(element, property))
        {
            out.problem(
                Completeness::Incomplete,
                "KQ_CONSTRUCTION_OBLIGATION",
                element,
                format!("Required property {property} is pending structural construction"),
            );
        }
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
        } else if self.model().navigation_slot(element, property).is_some()
            || matches!(
                self.model().property_state(element, property),
                Ok(agq_kernel::derived::PropertyState::Incomplete(_)
                    | agq_kernel::derived::PropertyState::Invalid(_))
            )
        {
            let fact = FactKey::Property { element, property };
            self.fact(out, fact);
            evidence.push(Evidence::Fact(fact));
        }
        evidence
    }
    fn fact_origin(&self, key: FactKey) -> Option<Arc<Origin>> {
        if let Some(origin) = self.origin_cache.lock().expect("origin cache").get(&key) {
            return origin.clone();
        }
        let origin = match key {
            FactKey::AssociationOccurrence(id) => self
                .model()
                .association_occurrence(id)
                .map(|link| link.origin().clone()),
            FactKey::Element(id) => self.model().element(id).map(|e| e.origin().clone()),
            FactKey::Property { element, property } => self
                .model()
                .navigation_slot(element, property)
                .map(|s| s.origin().clone())
                .or_else(
                    || match self.model().property_state(element, property).ok()? {
                        agq_kernel::derived::PropertyState::Incomplete(failure)
                        | agq_kernel::derived::PropertyState::Invalid(failure) => {
                            Some(Origin::Derived(failure.explanation().clone().into()))
                        }
                        _ => None,
                    },
                ),
        }
        .map(Arc::new);
        self.origin_cache
            .lock()
            .expect("origin cache")
            .entry(key)
            .or_insert(origin)
            .clone()
    }

    fn declared_fact_origin(&self, key: FactKey) -> Option<Arc<Origin>> {
        if let Some(current) = self.fact_origin(key)
            && matches!(current.as_ref(), Origin::Declared(_))
        {
            return Some(current);
        }
        let mut cache = self
            .declared_origin_cache
            .lock()
            .expect("declared origin cache");
        cache
            .entry(key)
            .or_insert_with(|| {
                self.model()
                    .declared_fact_origin(key)
                    .map(|source| Arc::new(Origin::Declared(source.clone())))
            })
            .clone()
    }

    /// Expand provenance without conflating an original declared assertion with
    /// a later derived extension of the same property key.
    pub(crate) fn fact<T>(&self, out: &mut QueryResult<T>, key: FactKey) {
        let Some(root) = self.fact_origin(key) else {
            return;
        };
        match root.as_ref() {
            Origin::Declared(_) => {
                out.canonical_dependencies.insert(Dependency::Declared(key));
            }
            Origin::Derived(_) => {
                out.canonical_dependencies.insert(Dependency::Derived(key));
            }
            Origin::AssociationOccurrences(links) => {
                for &id in links {
                    let fact = FactKey::AssociationOccurrence(id);
                    let dependency =
                        if matches!(self.fact_origin(fact).as_deref(), Some(Origin::Derived(_))) {
                            Dependency::Derived(fact)
                        } else {
                            Dependency::Declared(fact)
                        };
                    out.canonical_dependencies.insert(dependency);
                }
            }
        }
        if self.producer_evidence {
            // Immediate DAG edges above retain explainability. Search metadata
            // must still traverse dependencies, including negative reads beneath
            // an ownership extension or association navigation projection.
            let mut queue = vec![key];
            while let Some(fact) = queue.pop() {
                if !out.positive_dependencies.insert(fact) {
                    continue;
                }
                if let Some(searches) = self.model().computation_searches_shared(fact) {
                    out.shared_search_dependencies.insert(searches);
                }
                match self.fact_origin(fact).as_deref() {
                    Some(Origin::Derived(proof)) => {
                        queue.extend(proof.dependencies.iter().filter_map(|d| {
                            if let Dependency::Derived(fact) = d {
                                Some(*fact)
                            } else {
                                None
                            }
                        }))
                    }
                    Some(Origin::AssociationOccurrences(links)) => {
                        queue.extend(links.iter().copied().map(FactKey::AssociationOccurrence))
                    }
                    _ => {}
                }
            }
            return;
        }
        let mut queue = vec![(key, false)];
        while let Some((key, declared)) = queue.pop() {
            if declared {
                if out.declared_fact_origins.contains_key(&key) {
                    continue;
                }
                if let Some(origin) = self.declared_fact_origin(key) {
                    let Origin::Declared(source) = origin.as_ref() else {
                        unreachable!()
                    };
                    out.declared_fact_origins
                        .insert(key, Arc::new(source.clone()));
                    out.positive_dependencies.insert(key);
                    out.fact_origins.entry(key).or_insert(origin);
                }
                continue;
            }
            if out
                .fact_origins
                .get(&key)
                .is_some_and(|o| !matches!(o.as_ref(), Origin::Declared(_)))
            {
                continue;
            }
            let Some(origin) = self.fact_origin(key) else {
                continue;
            };
            out.search_dependencies.extend(
                self.model()
                    .computation_searches_for(key)
                    .cloned()
                    .map(SearchDependency::Kernel),
            );
            out.positive_dependencies.insert(key);
            out.fact_origins.insert(key, origin.clone());
            match origin.as_ref() {
                Origin::Declared(source) => {
                    out.declared_fact_origins
                        .entry(key)
                        .or_insert_with(|| Arc::new(source.clone()));
                }
                Origin::AssociationOccurrences(links) => {
                    queue.extend(
                        links
                            .iter()
                            .copied()
                            .map(|id| (FactKey::AssociationOccurrence(id), false)),
                    );
                }
                Origin::Derived(explanation) => {
                    queue.extend(explanation.dependencies.iter().map(|d| match d {
                        Dependency::Declared(f) => (*f, true),
                        Dependency::Derived(f) => (*f, false),
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
        let mut out = self.targets(specific, QueryKind::DirectSpecializations);
        if !self.context().options.exclude_implied {
            let implied = self.library_specializations(specific);
            out.value.extend(implied.value.iter().copied());
            out.merge(implied);
            out.value.sort();
            out.value.dedup();
        }
        if self.is(specific, c::FEATURE) && !self.context().options.exclude_implied {
            let result = self.reference_expression_result(specific);
            out.value.extend(result.value.iter().copied());
            out.merge(result);
            let redefinitions = self.implied_redefinitions(specific);
            for &target in &redefinitions.value {
                out.value.push(target);
                out.prove(
                    QueryKind::DirectSpecializations,
                    specific,
                    target,
                    Rule::Redefinition,
                    [claim(QueryKind::RedefinedFeatures, specific, target)],
                );
            }
            out.merge(redefinitions);
            out.value.sort();
            out.value.dedup();
        }
        out
    }
    /// Direct FeatureTyping endpoints only; not the complete derived Feature::type.
    pub fn direct_feature_types(&self, feature: ElementId) -> QueryResult<Vec<ElementId>> {
        self.targets(feature, QueryKind::DirectFeatureTypes)
    }

    /// KerML Type::supertypes and Feature::supertypes: conjugation replaces the
    /// specialization scopes; a chained feature additionally supplies its target.
    /// This is an identity set, not the normative ordered collection projection.
    pub fn supertypes(&self, ty: ElementId) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        if self.context().pending_specialization_scopes.contains(&ty) {
            out.problem(
                Completeness::Incomplete,
                "KQ_PENDING_SUPERTYPES",
                ty,
                "Pending specialization assertions affect supertype search",
            );
        }
        self.property(&mut out, ty, p::TYPE_IS_CONJUGATED);
        if matches!(
            self.model().property_state(ty, p::TYPE_IS_CONJUGATED),
            Ok(agq_kernel::derived::PropertyState::Computed(_)
                | agq_kernel::derived::PropertyState::Incomplete(_)
                | agq_kernel::derived::PropertyState::Invalid(_))
        ) && let Ok(view) = views::Type::try_new(ty, self.model())
        {
            self.accept(&mut out, ty, view.is_conjugated());
        }
        let owned = self.owned_relationships(ty);
        let conjugations: Vec<_> = owned
            .value
            .iter()
            .copied()
            .filter(|r| self.is(*r, c::CONJUGATION))
            .collect();
        let chains: Vec<_> = owned
            .value
            .iter()
            .copied()
            .filter(|r| self.is(*r, c::FEATURE_CHAINING))
            .collect();
        out.merge(owned);
        if conjugations.len() > 1 {
            out.problem(
                Completeness::Invalid,
                "KQ_CONJUGATOR_ARITY",
                ty,
                "Type has multiple owned conjugations",
            );
        }
        if let Some(&conjugation) = conjugations.first() {
            if let Some(original) =
                self.read_reference(&mut out, conjugation, p::CONJUGATION_ORIGINAL_TYPE)
            {
                out.value.push(original);
                let premises: Vec<_> = out
                    .positive_dependencies
                    .iter()
                    .copied()
                    .map(Evidence::Fact)
                    .chain(
                        out.search_dependencies
                            .iter()
                            .cloned()
                            .map(Evidence::Search),
                    )
                    .collect();
                out.prove(
                    QueryKind::Supertypes,
                    ty,
                    original,
                    Rule::ConjugatedInheritance,
                    premises,
                );
            } else {
                out.problem(
                    Completeness::Incomplete,
                    "KQ_MISSING_CONJUGATOR",
                    ty,
                    "Conjugation original Type is unresolved",
                );
            }
        } else {
            self.property(&mut out, ty, p::TYPE_IS_CONJUGATED);
            if matches!(
                self.read_value(&mut out, ty, p::TYPE_IS_CONJUGATED),
                Some(Value::Boolean(true))
            ) {
                out.problem(
                    Completeness::Incomplete,
                    "KQ_MISSING_CONJUGATOR",
                    ty,
                    "Conjugated Type requires its original Type",
                );
            }
            let direct = self.direct_specializations(ty);
            out.value.extend(direct.value.iter().copied());
            for &general in &direct.value {
                out.prove(
                    QueryKind::Supertypes,
                    ty,
                    general,
                    Rule::Specialization,
                    [claim(QueryKind::DirectSpecializations, ty, general)],
                );
            }
            out.merge(direct);
        }
        for (index, chain) in chains.iter().enumerate() {
            let Some(target) =
                self.read_reference(&mut out, *chain, p::FEATURE_CHAINING_CHAINING_FEATURE)
            else {
                out.problem(
                    Completeness::Incomplete,
                    "KQ_CHAIN_TARGET",
                    *chain,
                    "Feature chain target is unresolved",
                );
                continue;
            };
            if index + 1 == chains.len() && target != ty {
                out.value.push(target);
                let premises: Vec<_> = out
                    .positive_dependencies
                    .iter()
                    .copied()
                    .map(Evidence::Fact)
                    .chain(
                        out.search_dependencies
                            .iter()
                            .cloned()
                            .map(Evidence::Search),
                    )
                    .collect();
                out.prove(
                    QueryKind::Supertypes,
                    ty,
                    target,
                    Rule::FeatureChainInheritance,
                    premises,
                );
            }
        }
        out.value.sort();
        out.value.dedup();
        out
    }
    pub fn subsetted_features(&self, feature: ElementId) -> QueryResult<Vec<ElementId>> {
        self.targets(feature, QueryKind::SubsettedFeatures)
    }
    pub fn redefined_features(&self, feature: ElementId) -> QueryResult<Vec<ElementId>> {
        let mut out = self.targets(feature, QueryKind::RedefinedFeatures);
        let implied = self.implied_redefinitions(feature);
        out.value.extend(implied.value.iter().copied());
        out.merge(implied);
        out.value.sort();
        out.value.dedup();
        out
    }

    pub(crate) fn targets(
        &self,
        source: ElementId,
        kind: QueryKind,
    ) -> QueryResult<Vec<ElementId>> {
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
        let mut candidates = self.incoming_source_relationships(source, class, source_property);
        let owned = self.owned_relationships(source);
        // Owned relationships remain relevant even when their source endpoint
        // has not been computed yet. Ref/CrossSubsetting additionally derive
        // that endpoint from ownership below. Unknown *incoming general* sources
        // are not evidence about this Type's outgoing specialization population.
        candidates.extend(owned.value.iter().copied().filter(|id| self.is(*id, class)));
        out.merge(owned);
        for id in candidates {
            let Some(view) = self.checked::<views::Specialization, _>(&mut out, id) else {
                continue;
            };
            let mut evidence = self.property(&mut out, id, source_property);
            let specific = if self.is(id, c::REFERENCE_SUBSETTING)
                || self.is(id, c::CROSS_SUBSETTING)
            {
                let state = self.model().property_state(id, source_property);
                if matches!(
                    state,
                    Ok(agq_kernel::derived::PropertyState::Incomplete(_)
                        | agq_kernel::derived::PropertyState::Invalid(_))
                ) {
                    self.accept(&mut out, id, view.specific());
                    continue;
                }
                let owner = self.owning_related_element(id);
                let value = owner.value;
                out.merge(owner);
                if matches!(state, Ok(agq_kernel::derived::PropertyState::Computed(_))) {
                    let computed = self.accept(&mut out, id, view.specific());
                    if computed != value {
                        out.problem(Completeness::Invalid,"KQ_OWNED_SPECIALIZATION_SOURCE",id,
                            "Computed specialization source contradicts its required owning Feature");
                    }
                }
                if value.is_none() {
                    out.problem(
                        Completeness::Invalid,
                        "KQ_OWNED_SPECIALIZATION_SOURCE",
                        id,
                        "Owned specialization requires an owning Feature",
                    );
                }
                value
            } else {
                self.accept(&mut out, id, view.specific())
            };
            if specific != Some(source) {
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
