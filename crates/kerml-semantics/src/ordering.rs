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
            if self.context().pending_namespace_scopes.contains(&current)
                || self
                    .context()
                    .pending_specialization_scopes
                    .contains(&current)
            {
                out.problem(
                    Completeness::Incomplete,
                    "KQ_RESULT_POPULATION",
                    current,
                    "Pending memberships or generals prevent a closed return-parameter population",
                );
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
                if !self.context().semantic_extensions.is_empty()
                    && self.complete_result_cycles(
                        &graph,
                        &own,
                        &mut values,
                        &mut implied,
                        &mut out,
                    )
                {
                    continue;
                }
                out.problem(
                    Completeness::Incomplete,
                    "KQ_RESULT_INHERITANCE_CYCLE",
                    ty,
                    "Cyclic result inheritance requires an established membership fixed point",
                );
                return out;
            }
            for current in ready {
                let selected = self.select_inherited_results(
                    current,
                    &graph,
                    &own,
                    &values,
                    &mut implied,
                    &mut out,
                );
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
    /// Establish candidate reachability monotonically before suppression. This
    /// separates the finite membership lattice from semantic vector ordering:
    /// candidate identities never disappear while redefinition is discovered.
    /// The final ordered transfer must reach and independently satisfy the same
    /// equations as acyclic inheritance; otherwise the cycle stays incomplete.
    fn complete_result_cycles(
        &self,
        graph: &BTreeMap<ElementId, Vec<ElementId>>,
        own: &BTreeMap<ElementId, Vec<(ElementId, ElementId)>>,
        values: &mut BTreeMap<ElementId, Vec<(ElementId, ElementId)>>,
        implied: &mut BTreeMap<ElementId, BTreeSet<ElementId>>,
        out: &mut QueryResult<Vec<ElementId>>,
    ) -> bool {
        let mut completed = false;
        for component in unresolved_inheritance_components(graph, &values.keys().copied().collect())
        {
            if component.iter().any(|id| {
                graph[id]
                    .iter()
                    .any(|general| !component.contains(general) && !values.contains_key(general))
            }) {
                continue;
            }
            let mut candidates = values.clone();
            for &id in &component {
                candidates.insert(id, own[&id].clone());
            }
            loop {
                let before = candidates.clone();
                for &id in &component {
                    for parent in &graph[&id] {
                        for &(membership, feature) in &before[parent] {
                            if self.visible(
                                out,
                                membership,
                                p::MEMBERSHIP_VISIBILITY,
                                MemberAccess::NonPrivate,
                            ) && !candidates[&id].contains(&(membership, feature))
                            {
                                candidates.get_mut(&id).unwrap().push((membership, feature));
                            }
                        }
                    }
                }
                if candidates == before {
                    break;
                }
            }
            // Freeze transitive positional/explicit suppression using the full
            // candidate population. This avoids order-dependent discovery of a
            // later inherited redefiner after its ancestor was already emitted.
            let mut cycle_implied = implied.clone();
            for &id in &component {
                self.select_inherited_results(id, graph, own, &candidates, &mut cycle_implied, out);
            }
            let allowed: BTreeMap<_, BTreeSet<_>> = component
                .iter()
                .map(|&id| {
                    let selected = self.select_inherited_results(
                        id,
                        graph,
                        own,
                        &candidates,
                        &mut cycle_implied,
                        out,
                    );
                    (id, selected.into_iter().collect())
                })
                .collect();
            let mut fixed = values.clone();
            for &id in &component {
                fixed.insert(id, Vec::new());
            }
            let mut seen = BTreeSet::new();
            let established = loop {
                let state: Vec<_> = component.iter().map(|id| fixed[id].clone()).collect();
                if !seen.insert(state) {
                    break false;
                }
                let before = fixed.clone();
                for &id in &component {
                    let mut selected = own[&id].clone();
                    for parent in &graph[&id] {
                        for &candidate in &before[parent] {
                            if allowed[&id].contains(&candidate) && !selected.contains(&candidate) {
                                selected.push(candidate);
                            }
                        }
                    }
                    fixed.insert(id, selected);
                }
                if fixed == before {
                    // A stable filtered lattice result must also satisfy the
                    // original ordered equations with effective parent vectors.
                    let mut checked_implied = implied.clone();
                    for &id in &component {
                        self.select_inherited_results(
                            id,
                            graph,
                            own,
                            &fixed,
                            &mut checked_implied,
                            out,
                        );
                    }
                    let valid = component.iter().all(|&id| {
                        self.select_inherited_results(
                            id,
                            graph,
                            own,
                            &fixed,
                            &mut checked_implied,
                            out,
                        ) == fixed[&id]
                    });
                    if valid {
                        *implied = checked_implied;
                    }
                    break valid;
                }
            };
            if established {
                if out.completeness == Completeness::Complete {
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
                    for &id in &component {
                        // This certifies a population, not a fictitious result
                        // Feature. The conclusion keys identify every SCC member;
                        // exhaustive owned-return/general searches and external
                        // boundary facts justify even the empty fixed point.
                        out.prove(
                            QueryKind::ResultPopulation,
                            id,
                            id,
                            Rule::InheritedResultFixedPoint,
                            premises.clone(),
                        );
                    }
                }
                for id in component {
                    values.insert(id, fixed.remove(&id).unwrap());
                }
                completed = true;
            }
        }
        completed
    }

    fn select_inherited_results(
        &self,
        current: ElementId,
        graph: &BTreeMap<ElementId, Vec<ElementId>>,
        own: &BTreeMap<ElementId, Vec<(ElementId, ElementId)>>,
        values: &BTreeMap<ElementId, Vec<(ElementId, ElementId)>>,
        implied: &mut BTreeMap<ElementId, BTreeSet<ElementId>>,
        out: &mut QueryResult<Vec<ElementId>>,
    ) -> Vec<(ElementId, ElementId)> {
        let mut inherited = vec![];
        for parent in &graph[&current] {
            for &(membership, feature) in &values[parent] {
                if self.visible(
                    out,
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
        selected
    }
}

/// Strongly connected unresolved inheritance components. Resolved vertices
/// remain external boundary inputs; callers must prove those inputs complete.
pub(crate) fn unresolved_inheritance_components(
    graph: &BTreeMap<ElementId, Vec<ElementId>>,
    resolved: &BTreeSet<ElementId>,
) -> Vec<BTreeSet<ElementId>> {
    // Iterative Kosaraju over unresolved vertices. Already resolved generals
    // remain boundary inputs, so unrelated cyclic ancestors cannot be assumed
    // to have the same cardinality as this component.
    let unresolved: BTreeSet<_> = graph
        .keys()
        .copied()
        .filter(|id| !resolved.contains(id))
        .collect();
    let mut reverse = BTreeMap::<ElementId, Vec<ElementId>>::new();
    let mut seen = BTreeSet::new();
    let mut finish = Vec::with_capacity(unresolved.len());
    for &id in &unresolved {
        for &general in &graph[&id] {
            if unresolved.contains(&general) {
                reverse.entry(general).or_default().push(id);
            }
        }
        let mut pending = vec![(id, false)];
        while let Some((current, expanded)) = pending.pop() {
            if expanded {
                finish.push(current);
            } else if seen.insert(current) {
                pending.push((current, true));
                pending.extend(
                    graph[&current]
                        .iter()
                        .rev()
                        .filter(|general| unresolved.contains(general))
                        .map(|&general| (general, false)),
                );
            }
        }
    }
    let mut assigned = BTreeSet::new();
    let mut components = Vec::new();
    for seed in finish.into_iter().rev() {
        let mut component = BTreeSet::new();
        let mut pending = vec![seed];
        while let Some(current) = pending.pop() {
            if assigned.insert(current) {
                component.insert(current);
                pending.extend(reverse.get(&current).into_iter().flatten().copied());
            }
        }
        if component.is_empty() {
            continue;
        }
        components.push(component);
    }
    components
}
