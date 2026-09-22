//! Normative ordered ownership projections and canonical inherited results.
use crate::*;
use agq_kerml::{classes as c, properties as p};
use agq_kernel::{ElementId, provenance::FactKey};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

impl KerMlQueries<'_> {
    /// Type::ownedSpecialization.general in Element::ownedRelationship order.
    /// Incoming, non-owned specializations are not members of this projection.
    pub fn owned_specialization_targets(&self, ty: ElementId) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        let owned = self.owned_relationships_of_type(ty, c::SPECIALIZATION);
        for &relationship in &owned.value {
            if !self.is(relationship, c::SPECIALIZATION) {
                continue;
            }
            if self.context().options.exclude_implied
                && matches!(
                    self.read_value(&mut out, relationship, p::RELATIONSHIP_IS_IMPLIED),
                    Some(agq_kernel::value::Value::Boolean(true))
                )
            {
                continue;
            }
            let specific = if self.is(relationship, c::REFERENCE_SUBSETTING)
                || self.is(relationship, c::CROSS_SUBSETTING)
            {
                Some(ty)
            } else {
                self.read_reference(&mut out, relationship, p::SPECIALIZATION_SPECIFIC)
            };
            if specific != Some(ty) {
                continue;
            }
            if let Some(general) =
                self.read_reference(&mut out, relationship, p::SPECIALIZATION_GENERAL)
            {
                out.value.push(general);
            } else {
                out.problem(
                    Completeness::Incomplete,
                    "KQ_ORDERED_GENERAL",
                    relationship,
                    "Ordered owned specialization projection requires its general Type",
                );
            }
        }
        out.merge(owned);
        out
    }

    /// Canonical ReturnParameterMembership identities, including inherited ones.
    /// The computation precedes positional result redefinition, avoiding recursive
    /// calls from that rule back through full effective-feature computation.
    pub fn result_parameters(&self, ty: ElementId) -> QueryResult<Vec<ElementId>> {
        if let Some(result) = self
            .result_cache
            .lock()
            .expect("query cache")
            .get(&ty)
            .cloned()
        {
            return result;
        }
        let result = self.compute_result_parameters(ty);
        let mut cache = self.result_cache.lock().expect("query cache");
        if cache.len() >= 256 {
            cache.clear();
        }
        cache.insert(ty, result.clone());
        result
    }
    fn compute_result_parameters(&self, ty: ElementId) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        let mut graph = BTreeMap::<ElementId, Vec<ElementId>>::new();
        let mut own = BTreeMap::<ElementId, Vec<(ElementId, ElementId)>>::new();
        let mut queue = VecDeque::from([ty]);
        while let Some(current) = queue.pop_front() {
            if graph.contains_key(&current) {
                continue;
            }
            let members = self.memberships_of_type(current, c::RETURN_PARAMETER_MEMBERSHIP);
            let mut results = vec![];
            for &membership in &members.value {
                let member = self.member(membership);
                if let Some(feature) = member.value {
                    results.push((membership, feature));
                }
                out.merge(member);
            }
            out.merge(members);
            own.insert(current, results);
            let mut parents = self.owned_specialization_targets(current);
            if !self.context().options.exclude_implied {
                let library = self.library_specializations(current);
                parents.value.extend(library.value.iter().copied());
                parents.merge(library);
            }
            // Conjugation replaces ordinary supertypes; a feature target supplies
            // its inherited memberships as prescribed by Feature::supertypes.
            let conjugations = self.owned_relationships_of_type(current, c::CONJUGATION);
            let mut chain = vec![];
            for &r in &conjugations.value {
                parents.value.clear();
                parents.value.extend(self.read_reference(
                    &mut out,
                    r,
                    p::CONJUGATION_ORIGINAL_TYPE,
                ));
            }
            out.merge(conjugations);
            let chains = self.owned_relationships_of_type(current, c::FEATURE_CHAINING);
            for &r in &chains.value {
                chain.extend(self.read_reference(
                    &mut out,
                    r,
                    p::FEATURE_CHAINING_CHAINING_FEATURE,
                ));
            }
            if let Some(target) = chain.last().filter(|&&target| target != current) {
                parents.value.push(*target);
            }
            out.merge(chains);
            parents.value.retain(|&p| p != current);
            queue.extend(parents.value.iter().copied());
            graph.insert(current, parents.value.clone());
            out.merge(parents);
        }
        let mut values = BTreeMap::<ElementId, Vec<(ElementId, ElementId)>>::new();
        let mut implied = BTreeMap::<ElementId, BTreeSet<ElementId>>::new();
        while values.len() < graph.len() {
            let ready: Vec<_> = graph
                .iter()
                .filter(|(id, ps)| {
                    !values.contains_key(id) && ps.iter().all(|p| values.contains_key(p))
                })
                .map(|(id, _)| *id)
                .collect();
            if ready.is_empty() {
                out.problem(
                    Completeness::Incomplete,
                    "KQ_RESULT_INHERITANCE_CYCLE",
                    ty,
                    "Cyclic result inheritance requires an established membership fixed point",
                );
                return out;
            }
            for current in ready {
                let mut inherited = vec![];
                for parent in &graph[&current] {
                    for &(membership, feature) in &values[parent] {
                        if self.visible(
                            &mut out,
                            membership,
                            p::MEMBERSHIP_VISIBILITY,
                            MemberAccess::NonPrivate,
                        ) && !inherited.contains(&(membership, feature))
                        {
                            inherited.push((membership, feature));
                        }
                    }
                }
                if !self.context().options.exclude_implied
                    && (self.is(current, c::FUNCTION) || self.is(current, c::EXPRESSION))
                {
                    for &(_, feature) in &own[&current] {
                        for parent in &graph[&current] {
                            if self.is(*parent, c::FUNCTION) || self.is(*parent, c::EXPRESSION) {
                                implied.entry(feature).or_default().extend(
                                    values[parent]
                                        .iter()
                                        .map(|(_, f)| *f)
                                        .filter(|&f| f != feature),
                                );
                            }
                        }
                    }
                }
                // Other owned features matter only when they may suppress an
                // inherited result. Their population cannot affect a type that
                // has no inherited result identities to remove.
                let mut owned_features: Vec<_> = own[&current].iter().map(|(_, f)| *f).collect();
                let owned_results_suppress_inherited = inherited.iter().all(|(_, inherited)| {
                    owned_features.iter().any(|owned| {
                        implied
                            .get(owned)
                            .is_some_and(|targets| targets.contains(inherited))
                    })
                });
                if !inherited.is_empty() && !owned_results_suppress_inherited {
                    owned_features.clear();
                    let members = self.memberships_of_type(current, c::FEATURE_MEMBERSHIP);
                    for &membership in &members.value {
                        let member = self.member(membership);
                        owned_features.extend(member.value);
                        out.merge(member);
                    }
                    out.merge(members);
                }
                let mut closures = BTreeMap::new();
                for feature in owned_features
                    .iter()
                    .copied()
                    .chain(inherited.iter().map(|(_, f)| *f))
                {
                    let mut visited = BTreeSet::new();
                    let mut pending = vec![feature];
                    while let Some(f) = pending.pop() {
                        if !visited.insert(f) {
                            continue;
                        }
                        pending.extend(implied.get(&f).into_iter().flatten().copied());
                        let explicit = self.targets(f, QueryKind::RedefinedFeatures);
                        pending.extend(explicit.value.iter().copied());
                        out.merge(explicit);
                    }
                    closures.insert(feature, visited);
                }
                let mut selected = own[&current].clone();
                for &(membership, feature) in &inherited {
                    // Type::removeRedefinedFeatures: inherited redefinition and
                    // any owned feature redefining a common ancestral feature.
                    let suppressed = inherited
                        .iter()
                        .any(|(_, other)| *other != feature && closures[other].contains(&feature))
                        || owned_features.iter().any(|f| {
                            let explicit = self.targets(*f, QueryKind::RedefinedFeatures);
                            let mut direct: BTreeSet<_> = explicit.value.iter().copied().collect();
                            out.merge(explicit);
                            direct.extend(implied.get(f).into_iter().flatten().copied());
                            !direct.is_disjoint(&closures[&feature])
                        });
                    if !suppressed && !selected.contains(&(membership, feature)) {
                        selected.push((membership, feature));
                    }
                }
                values.insert(current, selected);
            }
        }
        out.value = values
            .remove(&ty)
            .unwrap_or_default()
            .into_iter()
            .map(|(_, feature)| feature)
            .collect();
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
        for &feature in &out.value.clone() {
            self.fact(&mut out, FactKey::Element(feature));
            out.prove(
                QueryKind::ExpressionResults,
                ty,
                feature,
                Rule::InheritedResult,
                premises.clone(),
            );
        }
        out
    }
}
