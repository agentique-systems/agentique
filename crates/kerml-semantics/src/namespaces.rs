//! KerML 1.0 Namespace/Type membership operations on canonical records.
//! Iterative graph construction and fixed-point propagation avoid recursive imports.
use crate::*;
use agq_kerml::{classes as c, properties as p};
use agq_kernel::{ElementId, PropertyId, provenance::FactKey, value::Value};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Visibility at the namespace boundary, independent of the caller's parser.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MemberAccess {
    All,
    Public,
    NonPrivate,
}

/// Membership identity is retained: an alias does not become its target's membership.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemberMatch {
    pub membership: ElementId,
    pub element: ElementId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct State {
    namespace: ElementId,
    access: MemberAccess,
    recursive: bool,
    excluded: Option<ElementId>,
    inherited_projection: bool,
}
pub(crate) type NamespaceCache =
    std::sync::Mutex<BTreeMap<State, std::sync::Arc<QueryResult<Vec<MemberMatch>>>>>;
#[derive(Default)]
struct Population {
    own: BTreeSet<MemberMatch>,
    imported: BTreeSet<MemberMatch>,
    owned_names: BTreeSet<String>,
    local_redefinitions: BTreeSet<ElementId>,
    imports: Vec<State>,
    inherited: Vec<State>,
    recursive: Vec<State>,
}

impl KerMlQueries<'_> {
    pub(crate) fn read_value<T>(
        &self,
        out: &mut QueryResult<T>,
        id: ElementId,
        property: PropertyId,
    ) -> Option<&Value> {
        self.property(out, id, property);
        let class = self.model().element(id)?.metaclass();
        let actual = self
            .model()
            .registry()
            .resolve_property(class, property)
            .ok()??
            .id;
        self.model()
            .navigation_slot(id, actual)?
            .value()
            .values()
            .next()
    }
    pub(crate) fn read_reference<T>(
        &self,
        out: &mut QueryResult<T>,
        id: ElementId,
        property: PropertyId,
    ) -> Option<ElementId> {
        match self.read_value(out, id, property) {
            Some(Value::Reference(id)) => Some(*id),
            _ => None,
        }
    }
    fn flag<T>(&self, out: &mut QueryResult<T>, id: ElementId, property: PropertyId) -> bool {
        matches!(
            self.read_value(out, id, property),
            Some(Value::Boolean(true))
        )
    }
    pub(crate) fn visible<T>(
        &self,
        out: &mut QueryResult<T>,
        id: ElementId,
        property: PropertyId,
        access: MemberAccess,
    ) -> bool {
        let Some(Value::Enumeration(literal)) = self.read_value(out, id, property) else {
            return false;
        };
        let agq_kernel::metamodel::ValueKind::Enumeration(domain) = self
            .model()
            .registry()
            .property(property)
            .expect("KerML property")
            .value_kind
        else {
            return false;
        };
        let Some(name) = self
            .model()
            .registry()
            .enumeration(domain)
            .expect("visibility domain")
            .literals
            .get(literal)
        else {
            return false;
        };
        match access {
            MemberAccess::All => true,
            MemberAccess::Public => name == "public",
            MemberAccess::NonPrivate => name != "private",
        }
    }
    pub(crate) fn names<T>(
        &self,
        out: &mut QueryResult<T>,
        membership: ElementId,
        element: Option<ElementId>,
    ) -> BTreeSet<String> {
        let mut names = BTreeSet::new();
        if !self.is(membership, c::OWNING_MEMBERSHIP) {
            for property in [p::MEMBERSHIP_MEMBER_NAME, p::MEMBERSHIP_MEMBER_SHORT_NAME] {
                if let Some(Value::String(name)) = self.read_value(out, membership, property) {
                    names.insert(name.clone());
                }
            }
            // Plain Membership names are optional stored values. Only an
            // OwningMembership derives its names from the member Element.
            return names;
        }
        let mut pending: Vec<_> = element.into_iter().collect();
        let mut seen = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if !seen.insert(id) {
                out.problem(
                    Completeness::Incomplete,
                    "KQ_NAMING_CYCLE",
                    id,
                    "Cyclic namingFeature dependencies do not establish an effective name",
                );
                continue;
            }
            let before = names.len();
            for property in [p::ELEMENT_DECLARED_NAME, p::ELEMENT_DECLARED_SHORT_NAME] {
                if let Some(Value::String(name)) = self.read_value(out, id, property) {
                    names.insert(name.clone());
                }
            }
            if names.len() == before && self.is(id, c::FEATURE) {
                // Feature::namingFeature is the FIRST owned Redefinition. It
                // does not combine names from all redefinition targets.
                let relationships = self.owned_relationships(id);
                if let Some(relationship) = relationships
                    .value
                    .iter()
                    .find(|r| self.is(**r, c::REDEFINITION))
                {
                    pending.extend(self.read_reference(
                        out,
                        *relationship,
                        p::REDEFINITION_REDEFINED_FEATURE,
                    ));
                } else {
                    // A required implied positional redefinition participates
                    // in naming too. A singleton establishes its first target
                    // without inventing an order among implied relationships.
                    let implied = self.implied_redefinitions(id);
                    if implied.value.len() == 1 {
                        pending.extend(implied.value.iter().copied());
                    } else if implied.value.len() > 1 {
                        out.problem(
                            Completeness::Incomplete,
                            "KQ_IMPLIED_NAMING_ORDER",
                            id,
                            "Multiple implied redefinitions require an established owned relationship order for naming",
                        );
                    }
                    out.merge(implied);
                }
                out.merge(relationships);
            }
        }
        names
    }
    fn redefinition_closure<T>(
        &self,
        out: &mut QueryResult<T>,
        features: impl IntoIterator<Item = ElementId>,
    ) -> BTreeSet<ElementId> {
        let mut seen = BTreeSet::new();
        let mut removed = BTreeSet::new();
        let mut pending: Vec<_> = features.into_iter().collect();
        while let Some(feature) = pending.pop() {
            if !seen.insert(feature) {
                continue;
            }
            let redefined = self.redefined_features(feature);
            pending.extend(redefined.value.iter().copied());
            removed.extend(redefined.value.iter().copied());
            out.merge(redefined);
        }
        removed
    }
    /// Exact named lookup over the normative membership population. Population
    /// results are memoized only inside this immutable evaluator/context.
    pub fn lookup_member(
        &self,
        namespace: ElementId,
        name: &str,
        access: MemberAccess,
    ) -> QueryResult<Vec<MemberMatch>> {
        let population = self.namespace_members(namespace, access);
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
        for member in out.value.clone() {
            out.prove(
                QueryKind::LookupMember,
                namespace,
                member.membership,
                Rule::NamespaceResolution,
                premises.clone(),
            );
        }
        out
    }
    /// Canonical owned, imported and inherited membership identities. Type
    /// redefinition filtering runs before selecting names, including renamed peers.
    pub fn namespace_members(
        &self,
        namespace: ElementId,
        access: MemberAccess,
    ) -> QueryResult<Vec<MemberMatch>> {
        self.namespace_members_excluding(namespace, access, None)
    }

    pub(crate) fn namespace_members_excluding(
        &self,
        namespace: ElementId,
        access: MemberAccess,
        excluded: Option<ElementId>,
    ) -> QueryResult<Vec<MemberMatch>> {
        self.membership_population(namespace, access, excluded, false)
    }

    /// Type::inheritedMemberships retains Membership identity during suppression.
    pub(crate) fn inherited_memberships(
        &self,
        namespace: ElementId,
    ) -> QueryResult<Vec<MemberMatch>> {
        self.membership_population(namespace, MemberAccess::NonPrivate, None, true)
    }

    fn membership_population(
        &self,
        namespace: ElementId,
        access: MemberAccess,
        excluded: Option<ElementId>,
        inherited_projection: bool,
    ) -> QueryResult<Vec<MemberMatch>> {
        // Exclusion cannot change a population that did not read the excluded
        // declaration. Reuse its full evidence within this immutable context.
        if let Some(feature) = excluded {
            let ordinary = self.namespace_members(namespace, access);
            if !ordinary
                .positive_dependencies
                .contains(&FactKey::Element(feature))
            {
                return ordinary;
            }
        }
        let start = State {
            namespace,
            access,
            recursive: false,
            excluded,
            inherited_projection,
        };
        if let Some(cached) = self
            .namespace_cache
            .lock()
            .expect("query cache")
            .get(&start)
            .cloned()
        {
            return (*cached).clone();
        }
        let mut out = self.result(vec![]);
        if !self.is(namespace, c::NAMESPACE) {
            out.problem(
                Completeness::Invalid,
                "KQ_NAMESPACE",
                namespace,
                "Lookup requires a Namespace",
            );
            return out;
        }
        let mut graph = BTreeMap::<State, Population>::new();
        let mut names = BTreeMap::new();
        let mut pending = VecDeque::from([start]);
        let mut closures = BTreeMap::new();
        while let Some(state) = pending.pop_front() {
            if graph.contains_key(&state) {
                continue;
            }
            let mut pop = Population::default();
            let namespace = state.namespace;
            out.search_dependencies
                .insert(SearchDependency::NamespaceMembers { namespace });
            out.search_dependencies
                .insert(SearchDependency::ImportSet { namespace });
            if self.context().pending_namespace_scopes.contains(&namespace)
                || self
                    .context()
                    .pending_specialization_scopes
                    .contains(&namespace)
            {
                out.problem(
                    Completeness::Incomplete,
                    "KQ_PENDING_NAMESPACE",
                    namespace,
                    "Pending declarations or specialization assertions affect this namespace",
                );
            }
            let memberships = self.memberships(namespace);
            for &membership in &memberships.value {
                let member = self.member(membership);
                if excluded.is_some() && member.value == excluded {
                    out.merge(member);
                    continue;
                }
                let member_names = self.names(&mut out, membership, member.value);
                pop.owned_names.extend(member_names.iter().cloned());
                names.insert(membership, member_names);
                let visible =
                    self.visible(&mut out, membership, p::MEMBERSHIP_VISIBILITY, state.access);
                if let Some(element) = member.value {
                    if self.is(element, c::FEATURE) {
                        let redefined = self.redefined_features(element);
                        // removeRedefinedFeatures uses ownedFeature, not aliases.
                        if self.is(membership, c::FEATURE_MEMBERSHIP) {
                            pop.local_redefinitions
                                .extend(redefined.value.iter().copied());
                        }
                        out.merge(redefined);
                        if let std::collections::btree_map::Entry::Vacant(entry) =
                            closures.entry(element)
                        {
                            let mut closure = self.redefinition_closure(&mut out, [element]);
                            closure.insert(element);
                            entry.insert(closure);
                        }
                    }
                    if visible && !state.inherited_projection {
                        pop.own.insert(MemberMatch {
                            membership,
                            element,
                        });
                    }
                    if state.recursive
                        && !state.inherited_projection
                        && visible
                        && self.is(element, c::NAMESPACE)
                        && self.is(membership, c::OWNING_MEMBERSHIP)
                    {
                        pop.recursive.push(State {
                            namespace: element,
                            ..state
                        });
                    }
                }
                out.merge(member);
            }
            out.merge(memberships);
            if self.is(namespace, c::TYPE) {
                let supers = self.supertypes(namespace);
                for &general in &supers.value {
                    pop.inherited.push(State {
                        namespace: general,
                        access: if state.access == MemberAccess::Public {
                            MemberAccess::Public
                        } else {
                            MemberAccess::NonPrivate
                        },
                        recursive: state.recursive,
                        excluded,
                        inherited_projection: false,
                    });
                }
                out.merge(supers);
            }
            let owned = self.owned_relationships(namespace);
            for &import in &owned.value {
                if state.inherited_projection
                    || !self.is(import, c::IMPORT)
                    || !self.visible(&mut out, import, p::IMPORT_VISIBILITY, state.access)
                {
                    continue;
                }
                let recursive = self.flag(&mut out, import, p::IMPORT_IS_RECURSIVE);
                let access = if self.flag(&mut out, import, p::IMPORT_IS_IMPORT_ALL) {
                    MemberAccess::All
                } else {
                    MemberAccess::Public
                };
                let target = if self.is(import, c::NAMESPACE_IMPORT) {
                    self.read_reference(&mut out, import, p::NAMESPACE_IMPORT_IMPORTED_NAMESPACE)
                } else if self.is(import, c::MEMBERSHIP_IMPORT) {
                    if let Some(membership) = self.read_reference(
                        &mut out,
                        import,
                        p::MEMBERSHIP_IMPORT_IMPORTED_MEMBERSHIP,
                    ) {
                        let member = self.member(membership);
                        if let Some(element) = member.value.filter(|e| Some(*e) != excluded) {
                            names.insert(
                                membership,
                                self.names(&mut out, membership, Some(element)),
                            );
                            pop.imported.insert(MemberMatch {
                                membership,
                                element,
                            });
                        }
                        let target = member
                            .value
                            .filter(|t| recursive && self.is(*t, c::NAMESPACE));
                        out.merge(member);
                        target
                    } else {
                        None
                    }
                } else {
                    out.problem(
                        Completeness::Incomplete,
                        "KQ_IMPORT_KIND",
                        import,
                        "Unsupported Import subtype",
                    );
                    None
                };
                if let Some(target) = target {
                    out.search_dependencies
                        .insert(SearchDependency::ImportedNamespace {
                            import,
                            namespace: target,
                        });
                    pop.imports.push(State {
                        namespace: target,
                        access,
                        recursive,
                        excluded,
                        inherited_projection: false,
                    });
                }
            }
            out.merge(owned);
            pending.extend(
                pop.imports
                    .iter()
                    .chain(&pop.inherited)
                    .chain(&pop.recursive)
                    .copied(),
            );
            graph.insert(state, pop);
        }
        // Alias-import targets can be features outside the otherwise visited scopes.
        for member in graph.values().flat_map(|p| p.imported.iter()) {
            if self.is(member.element, c::FEATURE) && !closures.contains_key(&member.element) {
                let mut closure = self.redefinition_closure(&mut out, [member.element]);
                closure.insert(member.element);
                closures.insert(member.element, closure);
            }
        }
        let mut values: BTreeMap<_, _> = graph.iter().map(|(s, p)| (*s, p.own.clone())).collect();
        let mut states = BTreeSet::new();
        loop {
            if !states.insert(values.clone()) {
                out.problem(
                    Completeness::Incomplete,
                    "KQ_MEMBERSHIP_FIXED_POINT",
                    namespace,
                    "Membership filtering in this cyclic inheritance graph has not converged",
                );
                break;
            }
            let mut next = BTreeMap::new();
            for (state, pop) in &graph {
                let mut members = pop.own.clone();
                let imported: BTreeSet<_> = pop
                    .imported
                    .iter()
                    .copied()
                    .chain(pop.imports.iter().flat_map(|s| values[s].iter().copied()))
                    .collect();
                members.extend(imported.into_iter().filter(|m| {
                    names
                        .get(&m.membership)
                        .is_none_or(|n| n.is_disjoint(&pop.owned_names))
                }));
                let inherited: BTreeSet<_> = pop
                    .inherited
                    .iter()
                    .flat_map(|s| values[s].iter().copied())
                    .collect();
                for member in &inherited {
                    let superseded = inherited.iter().any(|other| {
                        other.membership != member.membership
                            && closures
                                .get(&other.element)
                                .is_some_and(|c| c.contains(&member.element))
                    });
                    let locally_redefined = closures
                        .get(&member.element)
                        .is_some_and(|c| !c.is_disjoint(&pop.local_redefinitions));
                    if !superseded && !locally_redefined {
                        members.insert(*member);
                    }
                }
                members.extend(pop.recursive.iter().flat_map(|s| values[s].iter().copied()));
                next.insert(*state, members);
            }
            if next == values {
                break;
            }
            values = next;
        }
        out.value = values
            .remove(&start)
            .expect("start exists")
            .into_iter()
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
        out.prove(
            QueryKind::NamespacePopulation,
            namespace,
            namespace,
            Rule::NamespaceResolution,
            premises,
        );
        for member in out.value.clone() {
            self.fact(&mut out, FactKey::Element(member.membership));
            out.prove(
                QueryKind::NamespaceMemberships,
                namespace,
                member.membership,
                Rule::NamespaceResolution,
                [
                    crate::contract::claim(QueryKind::NamespacePopulation, namespace, namespace),
                    Evidence::Fact(FactKey::Element(member.membership)),
                ],
            );
        }
        self.namespace_cache
            .lock()
            .expect("query cache")
            .insert(start, std::sync::Arc::new(out.clone()));
        out
    }
}
