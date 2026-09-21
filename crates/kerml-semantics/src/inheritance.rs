use crate::{contract::claim, *};
use agq_kerml::{classes as c, properties as p, views};
use agq_kernel::{ElementId, provenance::FactKey};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

impl KerMlQueries<'_> {
    /// Owned and inherited FeatureMembership identities, including imports,
    /// chains and conjugation. Cycles use the namespace membership fixed point.
    /// Returns an identity set in ID order, not normative Type::feature ordering.
    pub fn effective_features(&self, ty: ElementId) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        if self.checked::<views::Type, _>(&mut out, ty).is_none() {
            return out;
        }
        let mut edges = BTreeMap::<ElementId, BTreeSet<ElementId>>::new();
        let mut own = BTreeMap::<ElementId, BTreeMap<ElementId, ElementId>>::new();
        let mut queue = VecDeque::from([ty]);
        let mut seen = BTreeSet::from([ty]);
        let mut has_external_memberships = false;
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
            let owned = self.owned_relationships(current);
            has_external_memberships |= owned.value.iter().any(|r| self.is(*r, c::IMPORT));
            let direct = self.supertypes(current);
            let parents: BTreeSet<_> = direct.value.iter().copied().collect();
            for &general in &parents {
                if seen.insert(general) {
                    queue.push_back(general);
                }
            }
            let features = self.direct_features(current);
            let memberships = self.memberships(current);
            has_external_memberships |= memberships
                .value
                .iter()
                .any(|m| !self.is(*m, c::OWNING_MEMBERSHIP));
            let mut members = BTreeMap::new();
            for &id in &memberships.value {
                let member = self.member(id);
                if self.is(id, c::FEATURE_MEMBERSHIP)
                    && let Some(feature) = member.value
                {
                    members.insert(feature, id);
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
        let order = dependency_order(&edges);

        // With imports or aliases, suppress the complete Membership population
        // before selecting FeatureMemberships. Distinct aliases retain their
        // normative effect even when they denote the same canonical feature.
        // Cyclic specialization is legal. Reuse the same convergence and
        // suppression contract as namespace navigation instead of rejecting it.
        if has_external_memberships || order.is_none() {
            let inherited = self.inherited_memberships(ty);
            let mut features: BTreeSet<_> = own[&ty].keys().copied().collect();
            for &feature in &features {
                out.prove(
                    QueryKind::EffectiveFeatures,
                    ty,
                    feature,
                    Rule::EffectiveOwnedFeature,
                    [claim(QueryKind::DirectFeatures, ty, feature)],
                );
            }
            for member in &inherited.value {
                if self.is(member.membership, c::FEATURE_MEMBERSHIP) {
                    features.insert(member.element);
                    out.prove(
                        QueryKind::EffectiveFeatures,
                        ty,
                        member.element,
                        Rule::InheritedFeature,
                        [
                            claim(QueryKind::NamespaceMemberships, ty, member.membership),
                            claim(QueryKind::Member, member.membership, member.element),
                        ],
                    );
                }
            }
            out.merge(inherited);
            out.value = features.into_iter().collect();
            return out;
        }

        // Compute each feature's redefinition closure once within this invocation.
        // Query results/evidence are merged once, not copied into every ancestor result.
        let mut redef_graph = BTreeMap::<ElementId, BTreeSet<ElementId>>::new();
        let mut pending: VecDeque<_> = own.values().flat_map(|m| m.keys().copied()).collect();
        while let Some(feature) = pending.pop_front() {
            if redef_graph.contains_key(&feature) {
                continue;
            }
            let redefined = self.redefined_features(feature);
            let targets: BTreeSet<_> = redefined.value.iter().copied().collect();
            pending.extend(targets.iter().copied());
            redef_graph.insert(feature, targets);
            out.merge(redefined);
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
        for current in order.expect("acyclic fast path") {
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
                        claim(QueryKind::Supertypes, current, parent),
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
