//! Reference denotation belongs here, independent of any textual parser.
use crate::*;
use agq_kerml::{classes as c, properties as p};
use agq_kernel::{ElementId, MetaclassId};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QualifiedName {
    pub absolute: bool,
    pub segments: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolution {
    Resolved(ElementId),
    Unresolved,
    Ambiguous(Vec<ElementId>),
    WrongKind(ElementId),
    /// Imports, inherited lookup, or other unavailable evidence blocks denotation.
    Incomplete,
}
impl KerMlQueries<'_> {
    /// KerML 1.0 8.2.3.5 context-relationship rules. This query determines scope
    /// from canonical ownership, including redefinition and ordered feature chains.
    pub fn lookup_relationship_target(
        &self,
        relationship: ElementId,
        property: agq_kernel::PropertyId,
        name: &QualifiedName,
    ) -> QueryResult<Vec<MemberMatch>> {
        let mut out = self.result(vec![]);
        let mut context = relationship;
        let mut visited = BTreeSet::new();
        loop {
            if !visited.insert(context) {
                out.problem(
                    Completeness::Invalid,
                    "KQ_CONTEXT_CYCLE",
                    context,
                    "Cyclic name-resolution context",
                );
                return out;
            }
            let mut owner = self.owning_related_element(context);
            if owner.value.is_none() {
                let nested = self.owner(context);
                owner.value = nested.value;
                owner.merge(nested);
            }
            let Some(mut scope) = owner.value else {
                out.merge(owner);
                return out;
            };
            out.merge(owner);
            let owned_specific = if self.is(context, c::REFERENCE_SUBSETTING) {
                true
            } else if self.is(context, c::SPECIALIZATION) {
                self.read_reference(&mut out, context, p::SPECIALIZATION_SPECIFIC) == Some(scope)
            } else if self.is(context, c::CONJUGATION) {
                self.read_reference(&mut out, context, p::CONJUGATION_CONJUGATED_TYPE)
                    == Some(scope)
            } else {
                false
            };
            if self.is(context, c::FEATURE_CHAINING) {
                let chain = self.owned_relationships(scope);
                let steps: Vec<_> = chain
                    .value
                    .iter()
                    .copied()
                    .filter(|r| self.is(*r, c::FEATURE_CHAINING))
                    .collect();
                let position = steps.iter().position(|r| *r == context);
                out.merge(chain);
                if let Some(position) = position.filter(|n| *n > 0) {
                    if let Some(previous) = self.read_reference(
                        &mut out,
                        steps[position - 1],
                        p::FEATURE_CHAINING_CHAINING_FEATURE,
                    ) {
                        let lookup = self.lookup_path(previous, name);
                        out.value = lookup.value.clone();
                        out.merge(lookup);
                    }
                    return out;
                }
                let owning = self.owning_relationship(scope);
                let next = owning.value;
                out.merge(owning);
                if let Some(next) = next {
                    context = next;
                    continue;
                }
                return out;
            }
            if self.is(context, c::REDEFINITION)
                && property == p::REDEFINITION_REDEFINED_FEATURE
                && self.is(scope, c::FEATURE)
                && owned_specific
            {
                let owner = self.owner(scope);
                let ty = owner.value;
                out.merge(owner);
                if let Some(ty) = ty.filter(|t| self.is(*t, c::TYPE)) {
                    if self.context().options.baseline_profile
                        == agq_kerml::BaselineProfile::OPERATIONAL_V2
                    {
                        let lookup = self.operational_redefinition_target(context, scope, ty, name);
                        out.value = lookup.value.clone();
                        out.merge(lookup);
                        return out;
                    }
                    let lookup = self.published_redefinition_target(ty, name);
                    out.value = lookup.value.clone();
                    out.merge(lookup);
                    return out;
                }
            }
            if self.is(context, c::SPECIALIZATION) || self.is(context, c::CONJUGATION) {
                if self.is(scope, c::TYPE) && owned_specific {
                    let parent = self.owner(scope);
                    let next = parent.value;
                    out.merge(parent);
                    if let Some(next) = next {
                        scope = next;
                    }
                }
                if self.is(context, c::REFERENCE_SUBSETTING) && self.is(scope, c::CONNECTOR) {
                    let parent = self.owner(scope);
                    let next = parent.value;
                    out.merge(parent);
                    if let Some(next) = next {
                        scope = next;
                    }
                }
            } else if self.is(context, c::MEMBERSHIP) && self.is(scope, c::FEATURE_CHAIN_EXPRESSION)
            {
                if let Some(argument) = self.argument_expression(&mut out, scope)
                    && let Some(result) = self.expression_result(&mut out, argument)
                {
                    scope = result;
                } else {
                    out.problem(
                        Completeness::Incomplete,
                        "KQ_CHAIN_ARGUMENT_RESULT",
                        context,
                        "Feature-chain lookup requires its argument expression result",
                    );
                    return out;
                }
            } else if self.is(context, c::MEMBERSHIP)
                && (self.is(scope, c::FEATURE_REFERENCE_EXPRESSION)
                    || (self.is(scope, c::INSTANTIATION_EXPRESSION)
                        && !self.is(context, c::FEATURE_MEMBERSHIP)))
            {
                loop {
                    let parent = self.owner(scope);
                    let next = parent.value;
                    out.merge(parent);
                    let skip = self.is(scope, c::FEATURE_REFERENCE_EXPRESSION)
                        || self.is(scope, c::INSTANTIATION_EXPRESSION)
                        || next.is_some_and(|n| self.is(n, c::INSTANTIATION_EXPRESSION));
                    if !skip {
                        break;
                    }
                    let Some(next) = next else {
                        break;
                    };
                    scope = next;
                }
            }
            let lookup = self.lookup_path(scope, name);
            out.value = lookup.value.clone();
            out.merge(lookup);
            return out;
        }
    }
    fn published_redefinition_target(
        &self,
        ty: ElementId,
        name: &QualifiedName,
    ) -> QueryResult<Vec<MemberMatch>> {
        let mut out = self.result(vec![]);
        let owned = self.owned_relationships(ty);
        let specializations: Vec<_> = owned
            .value
            .iter()
            .copied()
            .filter(|r| self.is(*r, c::SPECIALIZATION))
            .collect();
        out.merge(owned);
        for specialization in specializations {
            if let Some(general) =
                self.read_reference(&mut out, specialization, p::SPECIALIZATION_GENERAL)
            {
                let lookup = self.lookup_path(general, name);
                out.value = lookup.value.clone();
                out.merge(lookup);
                if !out.value.is_empty() {
                    return out;
                }
            }
        }
        // Implied generalizations also participate. Remove redundant
        // ancestors before selecting an implied starting namespace;
        // their more specific descendants already provide that scope.
        let generals = self.direct_specializations(ty);
        let mut redundant = BTreeSet::new();
        for &general in &generals.value {
            let ancestors = self.all_specializations(general);
            redundant.extend(ancestors.value.iter().copied());
            out.merge(ancestors);
        }
        let scopes: Vec<_> = generals
            .value
            .iter()
            .copied()
            .filter(|g| !redundant.contains(g))
            .collect();
        out.merge(generals);
        for general in scopes {
            let lookup = self.lookup_path(general, name);
            out.value = lookup.value.clone();
            out.merge(lookup);
            if !out.value.is_empty() {
                return out;
            }
        }
        out
    }

    /// Candidate denotations, with completeness and evidence preserved. During
    /// construction these are provisional until every answer-affecting read is complete.
    pub fn lookup_path(
        &self,
        scope: ElementId,
        name: &QualifiedName,
    ) -> QueryResult<Vec<MemberMatch>> {
        self.lookup_path_excluding(scope, name, None)
    }

    fn lookup_path_excluding(
        &self,
        scope: ElementId,
        name: &QualifiedName,
        excluded: Option<(ElementId, ElementId, RedefinitionRulePath)>,
    ) -> QueryResult<Vec<MemberMatch>> {
        let mut out = self.result(vec![]);
        if name.segments.is_empty() || name.segments.iter().any(String::is_empty) {
            out.problem(
                Completeness::Invalid,
                "KQ_REFERENCE_NAME",
                scope,
                "Reference path must have nonempty segments",
            );
            return out;
        }
        let mut scopes = vec![];
        let mut current = Some(scope);
        let mut seen = BTreeSet::new();
        while let Some(id) = current {
            if !seen.insert(id) {
                break;
            }
            scopes.push(id);
            let parent = self.owner(id);
            current = parent.value;
            out.merge(parent);
        }
        let root = *scopes.last().unwrap_or(&scope);
        if name.absolute {
            scopes.clear();
            scopes.push(root);
        }
        for namespace in scopes {
            if let Some((_, relationship, path)) = excluded {
                out.search_dependencies
                    .insert(SearchDependency::RedefinitionScope {
                        relationship,
                        namespace,
                        path,
                    });
            }
            let candidates = if namespace == root {
                out.search_dependencies
                    .insert(SearchDependency::ProjectRoots { root });
                let roots = self
                    .context()
                    .available_roots
                    .get(&root)
                    .cloned()
                    .unwrap_or_else(|| BTreeSet::from([root]));
                let mut candidates = BTreeSet::new();
                for available in roots {
                    let lookup = self.lookup_member_excluding(
                        available,
                        &name.segments[0],
                        if available == root {
                            MemberAccess::All
                        } else {
                            MemberAccess::Public
                        },
                        excluded.map(|(feature, _, _)| feature),
                    );
                    candidates.extend(lookup.value.iter().copied());
                    out.merge(lookup);
                }
                candidates.into_iter().collect()
            } else {
                let lookup = self.lookup_member_excluding(
                    namespace,
                    &name.segments[0],
                    MemberAccess::All,
                    excluded.map(|(feature, _, _)| feature),
                );
                let candidates = lookup.value.clone();
                out.merge(lookup);
                candidates
            };
            if !candidates.is_empty() {
                out.value = candidates;
                break;
            }
        }
        for segment in &name.segments[1..] {
            if out.value.len() != 1 {
                break;
            }
            let namespace = out.value[0].element;
            if !self.is(namespace, c::NAMESPACE) {
                out.value.clear();
                break;
            }
            let lookup = self.lookup_member_excluding(
                namespace,
                segment,
                MemberAccess::Public,
                excluded.map(|(feature, _, _)| feature),
            );
            out.value = lookup.value.clone();
            out.merge(lookup);
        }
        out
    }

    fn lookup_member_excluding(
        &self,
        namespace: ElementId,
        name: &str,
        access: MemberAccess,
        excluded: Option<ElementId>,
    ) -> QueryResult<Vec<MemberMatch>> {
        if excluded.is_none() {
            return self.lookup_member(namespace, name, access);
        }
        let population = self.namespace_members_excluding(namespace, access, excluded);
        self.named_population(population, name)
    }

    fn named_population(
        &self,
        population: QueryResult<Vec<MemberMatch>>,
        name: &str,
    ) -> QueryResult<Vec<MemberMatch>> {
        let mut out = self.result(vec![]);
        for member in &population.value {
            if self
                .names(&mut out, member.membership, Some(member.element))
                .contains(name)
            {
                out.value.push(*member);
            }
        }
        out.merge(population);
        out
    }

    /// AGQ-KERML10-002 / agentique-kerml10-redefinition-target/1.
    /// The dispatch above limits this interpretation to operational KerML 1.0/v2.
    fn operational_redefinition_target(
        &self,
        relationship: ElementId,
        feature: ElementId,
        owning_type: ElementId,
        name: &QualifiedName,
    ) -> QueryResult<Vec<MemberMatch>> {
        let mut out = self.result(vec![]);
        if name.segments.is_empty() || name.segments.iter().any(String::is_empty) {
            out.problem(
                Completeness::Invalid,
                "KQ_REFERENCE_NAME",
                relationship,
                "Reference path must have nonempty segments",
            );
            return out;
        }
        if name.absolute {
            out = self.lookup_path_excluding(
                owning_type,
                name,
                Some((feature, relationship, RedefinitionRulePath::ExplicitRoot)),
            );
        } else {
            let generals = self.supertypes(owning_type);
            let mut candidates = BTreeSet::new();
            for &namespace in &generals.value {
                out.search_dependencies
                    .insert(SearchDependency::RedefinitionScope {
                        relationship,
                        namespace,
                        path: RedefinitionRulePath::Inherited,
                    });
                let lookup = self.lookup_member_excluding(
                    namespace,
                    &name.segments[0],
                    MemberAccess::NonPrivate,
                    Some(feature),
                );
                candidates.extend(lookup.value.iter().copied());
                out.merge(lookup);
            }
            out.search_dependencies
                .insert(SearchDependency::RedefinitionScope {
                    relationship,
                    namespace: owning_type,
                    path: RedefinitionRulePath::Inherited,
                });
            out.merge(generals);
            // The special search compares matching candidates from every general
            // scope. A differently named redefinition on another branch does not
            // hide a name that is independently available through this branch.
            let mut suppressed = BTreeSet::new();
            for candidate in &candidates {
                let mut pending = vec![candidate.element];
                let mut seen = BTreeSet::new();
                while let Some(current) = pending.pop() {
                    if !seen.insert(current) || !self.is(current, c::FEATURE) {
                        continue;
                    }
                    let redefined = self.redefined_features(current);
                    for target in &redefined.value {
                        if *target != candidate.element {
                            suppressed.insert(*target);
                        }
                        pending.push(*target);
                    }
                    out.merge(redefined);
                }
            }
            if !candidates.is_empty() && candidates.iter().all(|m| suppressed.contains(&m.element))
            {
                out.problem(
                    Completeness::Invalid,
                    "KQ_REDEFINITION_CYCLE",
                    relationship,
                    "Mutually redefining candidates cannot establish a target",
                );
                out.value = candidates.into_iter().collect();
            } else {
                out.value = candidates
                    .into_iter()
                    .filter(|m| !suppressed.contains(&m.element))
                    .collect();
            }
            if out.value.is_empty() && out.completeness == Completeness::Complete {
                let parent = self.owner(owning_type);
                let lexical = parent.value;
                out.merge(parent);
                if let Some(lexical) = lexical {
                    let lookup = self.lookup_path_excluding(
                        lexical,
                        name,
                        Some((
                            feature,
                            relationship,
                            RedefinitionRulePath::LexicalContaining,
                        )),
                    );
                    out.value = lookup.value.clone();
                    out.merge(lookup);
                }
            } else {
                // A found prefix, ambiguity or incomplete inherited search cannot
                // trigger a lexical retry, even if a qualified suffix is absent.
                for segment in &name.segments[1..] {
                    if out.value.len() != 1 {
                        break;
                    }
                    let namespace = out.value[0].element;
                    if !self.is(namespace, c::NAMESPACE) {
                        out.value.clear();
                        break;
                    }
                    let lookup = self.lookup_member_excluding(
                        namespace,
                        segment,
                        MemberAccess::Public,
                        Some(feature),
                    );
                    out.value = lookup.value.clone();
                    out.merge(lookup);
                }
            }
        }
        self.fact(
            &mut out,
            agq_kernel::provenance::FactKey::Element(relationship),
        );
        for member in out.value.clone() {
            if member.element == feature || !self.is(member.element, c::FEATURE) {
                out.problem(
                    Completeness::Invalid,
                    "KQ_REDEFINITION_TARGET",
                    relationship,
                    "Redefinition requires a distinct Feature target",
                );
            }
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
                QueryKind::ResolveReference,
                relationship,
                member.element,
                Rule::OperationalRedefinitionTargetV1,
                premises,
            );
        }
        out
    }
    /// Resolve a name in an explicit semantic namespace. Membership targets are
    /// needed for MembershipImport; other references denote member elements.
    pub fn resolve_name(
        &self,
        scope: ElementId,
        name: &QualifiedName,
        expected: MetaclassId,
        membership_target: bool,
    ) -> QueryResult<Resolution> {
        let lookup = self.lookup_path(scope, name);
        self.denote_lookup(scope, name, expected, membership_target, lookup)
    }

    fn denote_lookup(
        &self,
        scope: ElementId,
        name: &QualifiedName,
        expected: MetaclassId,
        membership_target: bool,
        lookup: QueryResult<Vec<MemberMatch>>,
    ) -> QueryResult<Resolution> {
        let found: Vec<_> = lookup
            .value
            .iter()
            .map(|m| {
                if membership_target {
                    m.membership
                } else {
                    m.element
                }
            })
            .collect();
        let mut out = self.result(Resolution::Unresolved);
        out.merge(lookup);
        if out.completeness != Completeness::Complete {
            out.value = Resolution::Incomplete;
            return out;
        }
        match found.as_slice() {
            [] => out.problem(
                Completeness::Invalid,
                "KQ_UNRESOLVED",
                scope,
                format!("Unresolved reference {}", name.segments.join("::")),
            ),
            [target] if self.is(*target, expected) => {
                out.value = Resolution::Resolved(*target);
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
                    QueryKind::ResolveReference,
                    scope,
                    *target,
                    Rule::NamespaceResolution,
                    premises,
                );
            }
            [target] => {
                out.value = Resolution::WrongKind(*target);
                out.problem(
                    Completeness::Invalid,
                    "KQ_REFERENCE_KIND",
                    scope,
                    "Reference target has the wrong metaclass",
                );
            }
            _ => {
                out.value = Resolution::Ambiguous(found);
                out.problem(
                    Completeness::Invalid,
                    "KQ_AMBIGUOUS",
                    scope,
                    format!("Ambiguous reference {}", name.segments.join("::")),
                );
            }
        }
        out
    }
    /// Resolve an authored owned redefinition assertion before its relationship
    /// can be published. Evidence is anchored in the existing defining Feature;
    /// callers retain the source assertion's identity and origin separately.
    pub fn resolve_redefinition_reference(
        &self,
        feature: ElementId,
        name: &QualifiedName,
    ) -> QueryResult<Resolution> {
        let parent = self.owner(feature);
        let ty = parent.value;
        let mut lookup = if let Some(ty) = ty.filter(|t| self.is(*t, c::TYPE)) {
            if self.context().options.baseline_profile == agq_kerml::BaselineProfile::OPERATIONAL_V2
            {
                self.operational_redefinition_target(feature, feature, ty, name)
            } else {
                self.published_redefinition_target(ty, name)
            }
        } else {
            self.lookup_path(ty.unwrap_or(feature), name)
        };
        lookup.merge(parent);
        self.denote_lookup(feature, name, c::FEATURE, false, lookup)
    }

    /// KerML 8.2.3.5: an owned specialization starts in its specific Type's
    /// owning namespace. Names and imports are resolved exclusively by queries.
    pub fn resolve_reference(
        &self,
        specific: ElementId,
        name: &QualifiedName,
        expected: MetaclassId,
    ) -> QueryResult<Resolution> {
        let parent = self.owner(specific);
        let mut out = self.resolve_name(parent.value.unwrap_or(specific), name, expected, false);
        out.merge(parent);
        out
    }
}
