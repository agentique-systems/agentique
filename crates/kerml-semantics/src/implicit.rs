//! Structural consequences of KerML 1.0 checks, without executable evaluation.
use crate::*;
use agq_kerml::{classes as c, properties as p};
use agq_kernel::{ElementId, value::Value};

#[derive(Clone, Copy)]
enum Position {
    Parameter,
    Result,
    End,
}
impl KerMlQueries<'_> {
    pub(crate) fn expression_result<T>(
        &self,
        out: &mut QueryResult<T>,
        expression: ElementId,
    ) -> Option<ElementId> {
        let results = self.result_parameters(expression);
        let result = match results.value.as_slice() {
            [result] => Some(*result),
            [] => None,
            _ => {
                out.problem(
                    Completeness::Invalid,
                    "KQ_RESULT_ARITY",
                    expression,
                    "Expression has multiple effective result memberships",
                );
                None
            }
        };
        out.merge(results);
        result
    }
    pub(crate) fn argument_expression<T>(
        &self,
        out: &mut QueryResult<T>,
        expression: ElementId,
    ) -> Option<ElementId> {
        let members = self.memberships(expression);
        let parameter = members.value.iter().copied().find(|m| {
            self.is(*m, c::PARAMETER_MEMBERSHIP) && !self.is(*m, c::RETURN_PARAMETER_MEMBERSHIP)
        });
        out.merge(members);
        let parameter = parameter?;
        let member = self.member(parameter);
        let feature = member.value;
        out.merge(member);
        let owned = self.owned_relationships(feature?);
        let value = owned
            .value
            .iter()
            .copied()
            .find(|r| self.is(*r, c::FEATURE_VALUE));
        out.merge(owned);
        self.read_reference(out, value?, p::RELATIONSHIP_OWNED_RELATED_ELEMENT)
    }
    pub(crate) fn reference_expression_result(
        &self,
        feature: ElementId,
    ) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        let membership = self.owning_relationship(feature);
        let is_result = membership
            .value
            .is_some_and(|m| self.is(m, c::RETURN_PARAMETER_MEMBERSHIP));
        out.merge(membership);
        if !is_result {
            return out;
        }
        let owner = self.owning_type(feature);
        let expression = owner.value;
        out.merge(owner);
        let Some(expression) = expression.filter(|e| self.is(*e, c::FEATURE_REFERENCE_EXPRESSION))
        else {
            return out;
        };
        let members = self.memberships(expression);
        let referent = members
            .value
            .iter()
            .copied()
            .find(|m| !self.is(*m, c::PARAMETER_MEMBERSHIP));
        out.merge(members);
        if let Some(membership) = referent {
            let member = self.member(membership);
            out.value.extend(member.value);
            out.merge(member);
            for target in out.value.clone() {
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
                    QueryKind::DirectSpecializations,
                    feature,
                    target,
                    Rule::ExpressionResult,
                    premises,
                );
            }
        }
        out
    }
    pub(crate) fn library_specializations(&self, source: ElementId) -> QueryResult<Vec<ElementId>> {
        if let Some(result) = self
            .library_cache
            .lock()
            .expect("query cache")
            .get(&source)
            .cloned()
        {
            return result;
        }
        let result = self.compute_library_specializations(source);
        let mut cache = self.library_cache.lock().expect("query cache");
        if cache.len() >= 256 {
            cache.clear();
        }
        cache.insert(source, result.clone());
        result
    }
    // Required library bases before 8.4.2 redundancy suppression. Keeping this
    // separate lets redundancy traverse implied parent bases without recursive
    // cache calls or assuming the source's proposed edge.
    pub(crate) fn required_library_bases(&self, source: ElementId) -> QueryResult<Vec<ElementId>> {
        use StandardRole as R;
        let mut out = self.result(vec![]);
        let formal = self.formal_constraint_specializations(source);
        out.value.extend(formal.value.iter().copied());
        out.merge(formal);
        let Some(bindings) = self.context().standard_bindings.as_ref() else {
            return out;
        };
        out.search_dependencies
            .insert(SearchDependency::StandardLibraries);
        self.fact(&mut out, agq_kernel::provenance::FactKey::Element(source));
        let unconditional = self.metaclass_library_bases(source);
        out.value.extend(unconditional.value.iter().copied());
        out.merge(unconditional);
        let mut roles = vec![];
        if self.is(source, c::INVARIANT) {
            roles.push(
                if matches!(
                    self.read_value(&mut out, source, p::INVARIANT_IS_NEGATED),
                    Some(Value::Boolean(true))
                ) {
                    R::FalseEvaluations
                } else {
                    R::TrueEvaluations
                },
            );
        }
        if self.is(source, c::ASSOCIATION) {
            let ends = self.structural_end_features(source);
            if ends.completeness == Completeness::Complete && ends.value.len() == 2 {
                roles.push(R::BinaryLink);
            }
            out.merge(ends);
        }
        if self.is(source, c::FEATURE) {
            let typing = self.direct_feature_types(source);
            for target in &typing.value {
                for (class, role) in [
                    (c::STRUCTURE, R::Objects),
                    (c::CLASS, R::Occurrences),
                    (c::DATA_TYPE, R::DataValues),
                ] {
                    if self.is(*target, class) {
                        roles.push(role);
                    }
                }
            }
            out.merge(typing);
        }
        out.value.extend(
            roles
                .into_iter()
                .map(|role| bindings.get(role))
                .filter(|&target| target != source),
        );
        out.value.sort();
        out.value.dedup();
        out
    }

    /// Unconditional library implications of metaclass conformance. Keeping
    /// these separate avoids circularly testing an owner-typing antecedent
    /// before applying the mandatory Step/Expression base that supplies typing.
    pub(crate) fn metaclass_library_bases(&self, source: ElementId) -> QueryResult<Vec<ElementId>> {
        use StandardRole as R;
        let mut out = self.result(vec![]);
        let Some(bindings) = &self.context().standard_bindings else {
            return out;
        };
        out.search_dependencies
            .insert(SearchDependency::StandardLibraries);
        self.fact(&mut out, agq_kernel::provenance::FactKey::Element(source));
        for (class, role) in [
            (c::TYPE, R::Anything),
            (c::DATA_TYPE, R::DataValue),
            (c::CLASS, R::Occurrence),
            (c::STRUCTURE, R::Object),
            (c::ASSOCIATION, R::Link),
            (c::METACLASS, R::Metaobject),
            (c::BEHAVIOR, R::Performance),
            (c::FUNCTION, R::Evaluation),
            (c::PREDICATE, R::BooleanEvaluation),
            (c::FEATURE, R::Things),
            (c::STEP, R::Performances),
            (c::EXPRESSION, R::Evaluations),
            (c::BOOLEAN_EXPRESSION, R::BooleanEvaluations),
            (c::MULTIPLICITY, R::Naturals),
            (c::METADATA_FEATURE, R::Metaobjects),
            (c::CONNECTOR, R::Links),
            (c::BINDING_CONNECTOR, R::SelfLinks),
        ] {
            if self.is(source, class) {
                let target = bindings.get(role);
                if target != source {
                    out.value.push(target);
                }
            }
        }
        out.value.sort();
        out.value.dedup();
        out
    }
    fn compute_library_specializations(&self, source: ElementId) -> QueryResult<Vec<ElementId>> {
        let required = self.required_library_bases(source);
        let candidates: std::collections::BTreeSet<_> = required.value.iter().copied().collect();
        let mut out = self.result(vec![]);
        out.merge(required);
        if candidates.is_empty() {
            return out;
        }
        let explicit = self.targets(source, QueryKind::DirectSpecializations);
        let mut roots = explicit.value.clone();
        roots.extend(candidates.iter().copied());
        let mut reachable = std::collections::BTreeMap::new();
        for root in roots {
            let mut visited = std::collections::BTreeSet::new();
            let mut pending = vec![root];
            while let Some(current) = pending.pop() {
                // 8.4.2 redundancy is tested without assuming source's own
                // proposed implication, including in specialization cycles.
                if current == source || !visited.insert(current) {
                    continue;
                }
                let mut generals = self.targets(current, QueryKind::DirectSpecializations);
                let implied = self.required_library_bases(current);
                generals.value.extend(implied.value.iter().copied());
                generals.merge(implied);
                pending.extend(generals.value.iter().copied());
                out.merge(generals);
            }
            reachable.insert(root, visited);
        }
        for &target in &candidates {
            if target == source
                || explicit
                    .value
                    .iter()
                    .any(|e| reachable[e].contains(&target))
                || candidates
                    .iter()
                    .any(|&other| other != target && reachable[&other].contains(&target))
            {
                continue;
            }
            self.fact(&mut out, agq_kernel::provenance::FactKey::Element(target));
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
                QueryKind::DirectSpecializations,
                source,
                target,
                Rule::LibrarySpecialization,
                premises,
            );
        }
        out.merge(explicit);
        out.value.sort();
        out.value.dedup();
        out
    }
    fn positioned_features<T>(
        &self,
        out: &mut QueryResult<T>,
        owner: ElementId,
        position: Position,
    ) -> Vec<ElementId> {
        let members = self.memberships(owner);
        let mut features = vec![];
        for &membership in &members.value {
            if !self.is(membership, c::FEATURE_MEMBERSHIP) {
                continue;
            }
            let member = self.member(membership);
            if let Some(feature) = member.value {
                let result = self.is(membership, c::RETURN_PARAMETER_MEMBERSHIP);
                let selected = match position {
                    Position::Result => result,
                    Position::End => matches!(
                        self.read_value(out, feature, p::FEATURE_IS_END),
                        Some(Value::Boolean(true))
                    ),
                    Position::Parameter => {
                        !result
                            && self
                                .read_value(out, feature, p::FEATURE_DIRECTION)
                                .is_some()
                    }
                };
                if selected {
                    features.push(feature);
                }
            }
            out.merge(member);
        }
        out.merge(members);
        features
    }
    pub(crate) fn implied_redefinitions(&self, feature: ElementId) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        let membership = self.owning_relationship(feature);
        let result = membership
            .value
            .is_some_and(|m| self.is(m, c::RETURN_PARAMETER_MEMBERSHIP));
        out.merge(membership);
        let directed = self
            .read_value(&mut out, feature, p::FEATURE_DIRECTION)
            .is_some();
        let end = matches!(
            self.read_value(&mut out, feature, p::FEATURE_IS_END),
            Some(Value::Boolean(true))
        );
        if !result && !directed && !end {
            return out;
        }
        let owner = self.owning_type(feature);
        let Some(ty) = owner.value else {
            out.merge(owner);
            return out;
        };
        out.merge(owner);
        let behavioral = self.is(ty, c::BEHAVIOR) || self.is(ty, c::STEP);
        let mut positions = vec![];
        if result && (self.is(ty, c::FUNCTION) || self.is(ty, c::EXPRESSION)) {
            positions.push((Position::Result, Rule::ResultRedefinition));
        }
        if directed && !result && behavioral {
            positions.push((Position::Parameter, Rule::ParameterRedefinition));
        }
        if end {
            positions.push((Position::End, Rule::EndRedefinition));
        }
        if positions.is_empty() {
            return out;
        }
        // checkFeatureParameterRedefinition excludes explicit argument redefinitions
        // on an InvocationExpression. Other positional checks remain independent.
        if self.is(ty, c::INVOCATION_EXPRESSION) {
            let explicit = self.targets(feature, QueryKind::RedefinedFeatures);
            if !explicit.value.is_empty() {
                positions.retain(|(p, _)| !matches!(p, Position::Parameter));
            }
            out.merge(explicit);
        }
        let mut supers = self.owned_specialization_targets(ty);
        let library = self.library_specializations(ty);
        supers.value.extend(library.value.iter().copied());
        supers.merge(library);
        for (position, rule) in positions {
            let own = self.positioned_features(&mut out, ty, position);
            let Some(index) = own.iter().position(|f| *f == feature) else {
                continue;
            };
            for &general in &supers.value {
                let inherited = if matches!(position, Position::End) {
                    let ends = self.structural_end_features(general);
                    let values = ends.value.clone();
                    out.merge(ends);
                    values
                } else if matches!(position, Position::Result) {
                    if !self.is(general, c::FUNCTION) && !self.is(general, c::EXPRESSION) {
                        continue;
                    }
                    let results = self.result_parameters(general);
                    if results.value.len() > 1 {
                        out.problem(Completeness::Invalid,"KQ_RESULT_ARITY",general,
                            "Positional result redefinition requires one effective general result membership");
                    }
                    let values = results.value.clone();
                    out.merge(results);
                    values
                } else {
                    let parameters = self.structural_parameter_features(general);
                    let values = parameters.value.clone();
                    out.merge(parameters);
                    values
                };
                if let Some(&target) = inherited.get(index).filter(|&&target| target != feature) {
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
                        QueryKind::RedefinedFeatures,
                        feature,
                        target,
                        rule,
                        premises,
                    );
                }
            }
        }
        out.merge(supers);
        out.value.sort();
        out.value.dedup();
        out
    }

    /// Structural end ordering used by checkFeatureEndRedefinition and binary
    /// association specialization. Compute before library implications to avoid
    /// circularly assuming the very binary base whose obligation is being tested.
    pub(crate) fn structural_end_features(&self, ty: ElementId) -> QueryResult<Vec<ElementId>> {
        self.structural_positioned_features(ty, Position::End)
    }
    /// Effective non-result parameters in semantic order, including feature-chain targets.
    /// This projection precedes positional redefinition and does not copy inherited members.
    pub fn structural_parameter_features(&self, ty: ElementId) -> QueryResult<Vec<ElementId>> {
        self.structural_positioned_features(ty, Position::Parameter)
    }
    fn structural_positioned_features(
        &self,
        ty: ElementId,
        position: Position,
    ) -> QueryResult<Vec<ElementId>> {
        use std::collections::{BTreeMap, BTreeSet, VecDeque};
        let mut out = self.result(vec![]);
        let mut graph = BTreeMap::new();
        let mut owned = BTreeMap::new();
        let mut queue = VecDeque::from([ty]);
        while let Some(current) = queue.pop_front() {
            if graph.contains_key(&current) {
                continue;
            }
            let ends = self.positioned_features(&mut out, current, position);
            owned.insert(current, ends);
            let relationships = self.owned_relationships(current);
            let targets = self.owned_specialization_targets(current);
            let mut generals: Vec<_> = targets
                .value
                .iter()
                .copied()
                .filter(|&g| g != current)
                .collect();
            out.merge(targets);
            let mut conjugated = None;
            let mut chained = None;
            for &relationship in &relationships.value {
                if self.is(relationship, c::CONJUGATION) {
                    conjugated =
                        self.read_reference(&mut out, relationship, p::CONJUGATION_ORIGINAL_TYPE);
                }
                if self.is(relationship, c::FEATURE_CHAINING) {
                    chained = self.read_reference(
                        &mut out,
                        relationship,
                        p::FEATURE_CHAINING_CHAINING_FEATURE,
                    );
                }
            }
            if let Some(original) = conjugated {
                generals = vec![original];
            }
            if let Some(target) = chained.filter(|&target| target != current)
                && !generals.contains(&target)
            {
                generals.push(target);
            }
            out.merge(relationships);
            queue.extend(generals.iter().copied());
            graph.insert(current, generals);
        }
        let mut values = BTreeMap::<ElementId, Vec<ElementId>>::new();
        while values.len() < graph.len() {
            let ready: Vec<_> = graph
                .iter()
                .filter(|(id, parents)| {
                    !values.contains_key(id) && parents.iter().all(|p| values.contains_key(p))
                })
                .map(|(id, _)| *id)
                .collect();
            if ready.is_empty() {
                // Language extensions authenticate their own rule-set identity.
                // Keep the sealed KerML-only interpretation reproducible.
                if !self.context().semantic_extensions.is_empty()
                    && complete_owned_position_cycles(&graph, &owned, &mut values)
                {
                    continue;
                }
                out.problem(
                    Completeness::Incomplete,
                    "KQ_END_CYCLE",
                    ty,
                    "Cyclic specialization prevents complete structural end ordering",
                );
                return out;
            }
            for current in ready {
                let mut ends = owned[&current].clone();
                let mut suppressed = BTreeSet::new();
                // If positional redefinition already replaces every inherited
                // end, unresolved explicit targets cannot affect this end set.
                let covered = graph[&current]
                    .iter()
                    .all(|g| values[g].len() <= ends.len());
                for feature in ends.iter().filter(|_| !covered) {
                    let mut pending = vec![*feature];
                    let mut seen = BTreeSet::new();
                    while let Some(f) = pending.pop() {
                        if !seen.insert(f) {
                            continue;
                        }
                        let explicit = self.targets(f, QueryKind::RedefinedFeatures);
                        suppressed.extend(explicit.value.iter().copied());
                        pending.extend(explicit.value.iter().copied());
                        out.merge(explicit);
                    }
                }
                // Every owned end position implies redefinition of that position
                // in each direct general, independently of its declared name.
                for general in &graph[&current] {
                    suppressed.extend(values[general].iter().take(ends.len()).copied());
                }
                for general in &graph[&current] {
                    for &feature in &values[general] {
                        let membership = self.owning_relationship(feature);
                        let visible = membership.value.is_some_and(|m| {
                            self.visible(
                                &mut out,
                                m,
                                p::MEMBERSHIP_VISIBILITY,
                                MemberAccess::NonPrivate,
                            )
                        });
                        out.merge(membership);
                        if visible && !suppressed.contains(&feature) && !ends.contains(&feature) {
                            ends.push(feature);
                        }
                    }
                }
                values.insert(current, ends);
            }
        }
        out.value = values.remove(&ty).unwrap_or_default();
        out
    }
}

/// Resolve only cycles whose position vectors have a finite structural proof.
/// Every member must own the same number of positions, covering every resolved
/// external general. Then all inherited positions are replaced positionally and
/// each exact owned vector is the result. No seed iteration or identity ordering
/// selects among otherwise possible inherited orders. The zero-position case
/// similarly proves an empty result only when all external results are empty.
fn complete_owned_position_cycles(
    graph: &std::collections::BTreeMap<ElementId, Vec<ElementId>>,
    owned: &std::collections::BTreeMap<ElementId, Vec<ElementId>>,
    values: &mut std::collections::BTreeMap<ElementId, Vec<ElementId>>,
) -> bool {
    use std::collections::{BTreeMap, BTreeSet};

    // Iterative Kosaraju over unresolved vertices. Already resolved generals
    // remain boundary inputs, so unrelated cyclic ancestors cannot be assumed
    // to have the same cardinality as this component.
    let unresolved: BTreeSet<_> = graph
        .keys()
        .copied()
        .filter(|id| !values.contains_key(id))
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
    let mut completed = false;
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
        let count = owned[&seed].len();
        let covered = component.iter().all(|id| {
            owned[id].len() == count
                && graph[id].iter().all(|general| {
                    component.contains(general)
                        || values
                            .get(general)
                            .is_some_and(|positions| positions.len() <= count)
                })
        });
        if covered {
            for id in component {
                values.insert(id, owned[&id].clone());
            }
            completed = true;
        }
    }
    completed
}

#[cfg(test)]
#[path = "../tests/unit/owned_position_cycles.rs"]
mod owned_position_cycles;
