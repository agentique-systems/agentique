use crate::{contract::claim, *};
use agq_kerml::{classes as c, properties as p, views};
use agq_kernel::{ElementId, provenance::FactKey};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

impl KerMlQueries<'_> {
    /// Owned and inherited FeatureMembership members for the documented acyclic,
    /// unconjugated slice. Returns an identity set in ID order, not normative
    /// Type::feature ordering. Imports and feature chains mark results incomplete.
    pub fn effective_features(&self, ty: ElementId) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        if self.checked::<views::Type, _>(&mut out, ty).is_none() {
            return out;
        }
        let mut edges = BTreeMap::<ElementId, BTreeSet<ElementId>>::new();
        let mut own = BTreeMap::<ElementId, BTreeMap<ElementId, ElementId>>::new();
        let mut queue = VecDeque::from([ty]);
        let mut seen = BTreeSet::from([ty]);
        while let Some(current) = queue.pop_front() {
            if self
                .context()
                .pending_specialization_scopes
                .contains(&current)
                || self.context().pending_namespace_scopes.contains(&current)
            {
                out.problem(
                    Completeness::Incomplete,
                    "KQ_PENDING_INHERITANCE",
                    current,
                    "Pending project declarations or specializations may affect effective features",
                );
            }
            self.property(&mut out, current, p::TYPE_IS_CONJUGATED);
            if matches!(
                self.model().property_state(current, p::TYPE_IS_CONJUGATED),
                Ok(agq_kernel::derived::PropertyState::Computed(_)
                    | agq_kernel::derived::PropertyState::Incomplete(_)
                    | agq_kernel::derived::PropertyState::Invalid(_))
            ) {
                let view = views::Type::try_new(current, self.model()).expect("type endpoint");
                if self.accept(&mut out, current, view.is_conjugated()) == Some(true) {
                    out.problem(
                        Completeness::Incomplete,
                        "KQ_UNSUPPORTED_INHERITANCE",
                        current,
                        "conjugated types are outside the effective-feature slice",
                    );
                }
            }
            let owned = self.owned_relationships(current);
            let mut parents = BTreeSet::new();
            let direct = self.direct_specializations(current);
            for &id in &owned.value {
                if self.is(id, c::SPECIALIZATION) {
                    let view =
                        views::Specialization::try_new(id, self.model()).expect("checked class");
                    self.property(&mut out, id, p::SPECIALIZATION_SPECIFIC);
                    self.property(&mut out, id, p::SPECIALIZATION_GENERAL);
                    self.property(&mut out, id, p::RELATIONSHIP_IS_IMPLIED);
                    if view.specific().ok() == Some(current)
                        && !(self.context().options.exclude_implied
                            && view.is_implied().ok() == Some(true))
                        && let Ok(general) = view.general()
                    {
                        parents.insert(general);
                        if seen.insert(general) {
                            queue.push_back(general);
                        }
                    }
                }
                if [c::CONJUGATION, c::IMPORT, c::FEATURE_CHAINING]
                    .iter()
                    .any(|class| self.is(id, *class))
                {
                    out.problem(Completeness::Incomplete, "KQ_UNSUPPORTED_INHERITANCE", id, "conjugation, imports and feature chaining are outside the effective-feature slice");
                }
            }
            let features = self.direct_features(current);
            let memberships = self.memberships(current);
            let mut members = BTreeMap::new();
            for &id in &memberships.value {
                let member = self.member(id);
                if self.is(id, c::FEATURE_MEMBERSHIP) {
                    if let Some(feature) = member.value {
                        members.insert(feature, id);
                    }
                } else if member
                    .value
                    .is_some_and(|target| self.is(target, c::FEATURE))
                {
                    out.problem(Completeness::Incomplete, "KQ_NONFEATURE_MEMBERSHIP", id,
                        "inheritance involving feature aliases or ordinary owning memberships is outside this slice");
                }
                out.merge(member);
            }
            own.insert(current, members);
            edges.insert(current, parents);
            out.merge(features);
            out.merge(memberships);
            out.merge(direct);
            out.merge(owned);
        }
        let Some(order) = dependency_order(&edges) else {
            // Cyclic specialization is not itself illegal. The recursive membership
            // exclusion algorithm needs a separate fixed-point rule not implemented here.
            out.problem(
                Completeness::Incomplete,
                "KQ_CYCLIC_INHERITANCE",
                ty,
                "effective features in cyclic specialization graphs are outside this rule slice",
            );
            return out;
        };

        // Compute each feature's redefinition closure once within this invocation.
        // Query results/evidence are merged once, not copied into every ancestor result.
        let mut redef_graph = BTreeMap::<ElementId, BTreeSet<ElementId>>::new();
        let mut pending: VecDeque<_> = own.values().flat_map(|m| m.keys().copied()).collect();
        while let Some(feature) = pending.pop_front() {
            if redef_graph.contains_key(&feature) {
                continue;
            }
            let redefined = self.redefined_features(feature);
            let owned = self.owned_relationships(feature);
            let mut targets = BTreeSet::new();
            for &id in &owned.value {
                if self.is(id, c::REDEFINITION) {
                    let view =
                        views::Redefinition::try_new(id, self.model()).expect("checked class");
                    self.property(&mut out, id, p::REDEFINITION_REDEFINING_FEATURE);
                    self.property(&mut out, id, p::REDEFINITION_REDEFINED_FEATURE);
                    if view.redefining_feature().ok() == Some(feature)
                        && let Ok(target) = view.redefined_feature()
                    {
                        targets.insert(target);
                    }
                }
            }
            pending.extend(targets.iter().copied());
            redef_graph.insert(feature, targets);
            out.merge(redefined);
            out.merge(owned);
        }
        let mut closures = BTreeMap::<ElementId, BTreeSet<ElementId>>::new();
        for &feature in redef_graph.keys() {
            let mut visited = BTreeSet::from([feature]);
            let mut queue = VecDeque::from([feature]);
            out.prove(
                QueryKind::AllRedefinedFeatures,
                feature,
                feature,
                Rule::Redefinition,
                [Evidence::Fact(FactKey::Element(feature))],
            );
            while let Some(current) = queue.pop_front() {
                for &target in &redef_graph[&current] {
                    if visited.insert(target) {
                        queue.push_back(target);
                        out.prove(
                            QueryKind::AllRedefinedFeatures,
                            feature,
                            target,
                            Rule::Redefinition,
                            [
                                claim(QueryKind::AllRedefinedFeatures, feature, current),
                                claim(QueryKind::RedefinedFeatures, current, target),
                            ],
                        );
                    }
                }
            }
            closures.insert(feature, visited);
        }
        let mut effective = BTreeMap::<ElementId, BTreeMap<ElementId, ElementId>>::new();
        for current in order {
            let mut inherited = BTreeMap::<ElementId, ElementId>::new();
            let mut paths = BTreeMap::<ElementId, BTreeSet<ElementId>>::new();
            for parent in &edges[&current] {
                for (&feature, &membership) in &effective[parent] {
                    let view = views::Membership::try_new(membership, self.model())
                        .expect("membership evidence");
                    self.property(&mut out, membership, p::MEMBERSHIP_VISIBILITY);
                    let visible = self
                        .accept(&mut out, membership, view.visibility())
                        .is_some_and(|literal| {
                            let agq_kernel::metamodel::ValueKind::Enumeration(domain) = self
                                .model()
                                .registry()
                                .property(p::MEMBERSHIP_VISIBILITY)
                                .expect("normative property")
                                .value_kind
                            else {
                                unreachable!()
                            };
                            self.model()
                                .registry()
                                .enumeration(domain)
                                .expect("normative enum")
                                .literals[&literal]
                                != "private"
                        });
                    if visible {
                        inherited.insert(feature, membership);
                        paths.entry(feature).or_default().insert(*parent);
                    }
                }
            }
            // Record the inheritance candidate before filtering so removal proofs
            // explain both why it reached this type and why it was excluded.
            for (&feature, &membership) in &inherited {
                for &parent in &paths[&feature] {
                    let mut premises = vec![
                        claim(QueryKind::EffectiveFeatures, parent, feature),
                        claim(QueryKind::DirectSpecializations, current, parent),
                    ];
                    premises.extend(self.property(
                        &mut out,
                        current,
                        p::ELEMENT_OWNED_RELATIONSHIP,
                    ));
                    premises.extend(self.property(&mut out, membership, p::MEMBERSHIP_VISIBILITY));
                    out.prove(
                        QueryKind::InheritedFeature,
                        current,
                        feature,
                        Rule::InheritedFeature,
                        premises,
                    );
                }
            }
            // Type::removeRedefinedFeatures, both normative conditions. Index the
            // first suppressor so candidates do not compare every pair of features.
            let mut superseded = BTreeMap::new();
            for &feature in inherited.keys() {
                for &target in &closures[&feature] {
                    if target != feature {
                        superseded.entry(target).or_insert(feature);
                    }
                }
            }
            let mut owned_targets = BTreeMap::new();
            for &feature in own[&current].keys() {
                // This condition uses direct redefinition, including non-owned assertions.
                let redefined = self.redefined_features(feature);
                for &target in &redefined.value {
                    owned_targets.entry(target).or_insert(feature);
                }
                out.merge(redefined);
            }
            let mut result = own[&current].clone();
            for &feature in result.keys() {
                out.prove(
                    QueryKind::EffectiveFeatures,
                    current,
                    feature,
                    Rule::EffectiveOwnedFeature,
                    [claim(QueryKind::DirectFeatures, current, feature)],
                );
            }
            for (&feature, &membership) in &inherited {
                let suppression = if let Some(&other) = superseded.get(&feature) {
                    Some(vec![
                        claim(QueryKind::AllRedefinedFeatures, other, feature),
                        claim(QueryKind::InheritedFeature, current, other),
                    ])
                } else {
                    closures[&feature].iter().find_map(|target| {
                        owned_targets.get(target).map(|owner| {
                            vec![
                                claim(QueryKind::AllRedefinedFeatures, feature, *target),
                                claim(QueryKind::RedefinedFeatures, *owner, *target),
                                claim(QueryKind::DirectFeatures, current, *owner),
                            ]
                        })
                    })
                };
                if let Some(mut premises) = suppression {
                    premises.push(claim(QueryKind::Member, membership, feature));
                    premises.push(claim(QueryKind::InheritedFeature, current, feature));
                    out.prove(
                        QueryKind::SuppressedFeature,
                        current,
                        feature,
                        Rule::RemoveRedefinedFeature,
                        premises,
                    );
                    continue;
                }
                result.insert(feature, membership);
                out.prove(
                    QueryKind::EffectiveFeatures,
                    current,
                    feature,
                    Rule::InheritedFeature,
                    [
                        claim(QueryKind::InheritedFeature, current, feature),
                        Evidence::Search(SearchDependency::Incoming { target: feature }),
                    ],
                );
            }
            effective.insert(current, result);
        }
        out.value = effective
            .remove(&ty)
            .unwrap_or_default()
            .into_keys()
            .collect();
        out
    }
}

/// Iterative dependency-first order; every vertex/edge is visited a bounded number of times.
fn dependency_order(edges: &BTreeMap<ElementId, BTreeSet<ElementId>>) -> Option<Vec<ElementId>> {
    let mut remaining: BTreeMap<_, _> = edges.iter().map(|(&k, v)| (k, v.len())).collect();
    let mut children = BTreeMap::<ElementId, Vec<ElementId>>::new();
    for (&child, parents) in edges {
        for &parent in parents {
            children.entry(parent).or_default().push(child);
        }
    }
    let mut ready: BTreeSet<_> = remaining
        .iter()
        .filter(|(_, n)| **n == 0)
        .map(|(&k, _)| k)
        .collect();
    let mut order = Vec::new();
    while let Some(node) = ready.pop_first() {
        order.push(node);
        for child in children.get(&node).into_iter().flatten() {
            let count = remaining.get_mut(child).expect("graph vertex");
            *count -= 1;
            if *count == 0 {
                ready.insert(*child);
            }
        }
    }
    (order.len() == edges.len()).then_some(order)
}
