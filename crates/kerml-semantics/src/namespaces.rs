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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct State {
    namespace: ElementId,
    access: MemberAccess,
    recursive: bool,
    excluded: Option<ElementId>,
    inherited_projection: bool,
    imported_projection: bool,
    excluded_import_scopes: Vec<ElementId>,
}
pub(crate) type NamespaceCache =
    std::sync::Mutex<BTreeMap<State, std::sync::Arc<QueryResult<Vec<MemberMatch>>>>>;
#[derive(Default)]
struct Population {
    own: Vec<MemberMatch>,
    collision_owned: Vec<MemberMatch>,
    imported: Vec<MemberMatch>,
    import_order: Vec<ImportSource>,
    owned_names: BTreeSet<String>,
    local_redefinitions: BTreeSet<ElementId>,
    imports: Vec<State>,
    inherited: Vec<State>,
    recursive: Vec<State>,
}

#[derive(Clone)]
enum ImportSource {
    Member(MemberMatch),
    Namespace(State),
}

fn deduplicate(members: impl IntoIterator<Item = MemberMatch>) -> Vec<MemberMatch> {
    let mut seen = BTreeSet::new();
    members
        .into_iter()
        .filter(|m| seen.insert(m.membership))
        .collect()
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
        if let Some(element) = element {
            let effective = self.effective_names(element);
            if let EffectiveNames::Determinate(values) = &effective.value {
                names.extend(values.iter().cloned());
            }
            out.merge(effective);
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
        self.membership_population(namespace, access, excluded, false, false)
    }

    /// Type::inheritedMemberships retains Membership identity during suppression.
    pub(crate) fn inherited_memberships(
        &self,
        namespace: ElementId,
    ) -> QueryResult<Vec<MemberMatch>> {
        self.membership_population(namespace, MemberAccess::NonPrivate, None, true, false)
    }

    /// Imported Membership identities, before combination with owned/inherited members.
    /// Historical profiles retain the formal OCL population. V8 applies prose collisions.
    pub fn imported_memberships(
        &self,
        namespace: ElementId,
        access: MemberAccess,
    ) -> QueryResult<Vec<MemberMatch>> {
        self.membership_population(namespace, access, None, false, true)
    }

    /// Membership::isDistinguishableFrom, shared by imports and validation.
    pub fn memberships_distinguishable(
        &self,
        left: ElementId,
        right: ElementId,
    ) -> QueryResult<bool> {
        let mut out = self.result(true);
        let a = self.member(left);
        let b = self.member(right);
        if let (Some(a), Some(b)) = (a.value, b.value) {
            let left_names = self.names(&mut out, left, Some(a));
            let right_names = self.names(&mut out, right, Some(b));
            out.value = left_names.is_disjoint(&right_names)
                || !self.comparable_member_kinds(&mut out, a, b);
        }
        out.merge(a);
        out.merge(b);
        out
    }

    pub(crate) fn comparable_member_kinds<T>(
        &self,
        out: &mut QueryResult<T>,
        left: ElementId,
        right: ElementId,
    ) -> bool {
        self.fact(out, FactKey::Element(left));
        self.fact(out, FactKey::Element(right));
        let registry = self.model().registry();
        let left = self
            .model()
            .element(left)
            .expect("membership endpoint")
            .metaclass();
        let right = self
            .model()
            .element(right)
            .expect("membership endpoint")
            .metaclass();
        registry.is_subtype(left, right).unwrap_or(false)
            || registry.is_subtype(right, left).unwrap_or(false)
    }

    fn membership_population(
        &self,
        namespace: ElementId,
        access: MemberAccess,
        excluded: Option<ElementId>,
        inherited_projection: bool,
        imported_projection: bool,
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
            imported_projection,
            excluded_import_scopes: vec![],
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
        let corrected = self
            .context()
            .options
            .baseline_profile
            .corrects_import_collisions();
        if corrected {
            out.search_dependencies
                .insert(SearchDependency::ValidationRule(
                    "agentique-kerml10-import-collisions/1",
                ));
        }
        let mut graph = BTreeMap::<State, Population>::new();
        let mut names = BTreeMap::new();
        let mut pending = VecDeque::from([start.clone()]);
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
                    pop.collision_owned.push(MemberMatch {
                        membership,
                        element,
                    });
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
                    if visible && !state.inherited_projection && !state.imported_projection {
                        pop.own.push(MemberMatch {
                            membership,
                            element,
                        });
                    }
                    if state.recursive
                        && !state.inherited_projection
                        && !state.imported_projection
                        && visible
                        && self.is(element, c::NAMESPACE)
                        && self.is(membership, c::OWNING_MEMBERSHIP)
                    {
                        pop.recursive.push(State {
                            namespace: element,
                            ..state.clone()
                        });
                    }
                }
                out.merge(member);
            }
            out.merge(memberships);
            if self.is(namespace, c::TYPE) && !state.imported_projection {
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
                        imported_projection: false,
                        excluded_import_scopes: state.excluded_import_scopes.clone(),
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
                            let candidate = MemberMatch {
                                membership,
                                element,
                            };
                            pop.imported.push(candidate);
                            pop.import_order.push(ImportSource::Member(candidate));
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
                    if corrected
                        && (target == namespace || state.excluded_import_scopes.contains(&target))
                    {
                        continue;
                    }
                    let mut excluded_import_scopes = state.excluded_import_scopes.clone();
                    if corrected {
                        excluded_import_scopes.push(namespace);
                        excluded_import_scopes.sort();
                        excluded_import_scopes.dedup();
                    }
                    out.search_dependencies
                        .insert(SearchDependency::ImportedNamespace {
                            import,
                            namespace: target,
                        });
                    let target = State {
                        namespace: target,
                        access,
                        recursive,
                        excluded,
                        inherited_projection: false,
                        imported_projection: false,
                        excluded_import_scopes,
                    };
                    pop.imports.push(target.clone());
                    pop.import_order.push(ImportSource::Namespace(target));
                }
            }
            out.merge(owned);
            pending.extend(
                pop.imports
                    .iter()
                    .chain(&pop.inherited)
                    .chain(&pop.recursive)
                    .cloned(),
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
        let mut values: BTreeMap<_, _> = graph
            .iter()
            .map(|(s, p)| (s.clone(), p.own.clone()))
            .collect();
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
                let imported =
                    deduplicate(pop.import_order.iter().flat_map(|source| match source {
                        ImportSource::Member(member) => vec![*member],
                        ImportSource::Namespace(state) => values[state].clone(),
                    }));
                if corrected {
                    // First exclude owned collisions; only then compare the surviving imports.
                    // All pairs see the same population, so traversal order never picks a winner.
                    let mut collides = |a: &MemberMatch, b: &MemberMatch| {
                        a.membership != b.membership
                            && names
                                .get(&a.membership)
                                .zip(names.get(&b.membership))
                                .is_some_and(|(a, b)| !a.is_disjoint(b))
                            && self.comparable_member_kinds(&mut out, a.element, b.element)
                    };
                    let candidates: Vec<_> = imported
                        .into_iter()
                        .filter(|m| !pop.collision_owned.iter().any(|own| collides(m, own)))
                        .collect();
                    members.extend(
                        candidates
                            .iter()
                            .copied()
                            .filter(|m| !candidates.iter().any(|other| collides(m, other))),
                    );
                } else {
                    members.extend(imported.into_iter().filter(|m| {
                        state.imported_projection
                            || names
                                .get(&m.membership)
                                .is_none_or(|n| n.is_disjoint(&pop.owned_names))
                    }));
                }
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
                        members.push(*member);
                    }
                }
                members.extend(pop.recursive.iter().flat_map(|s| values[s].iter().copied()));
                let mut members = deduplicate(members);
                if !corrected {
                    members.sort();
                }
                next.insert(state.clone(), members);
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
        let population_query = if imported_projection {
            QueryKind::ImportedMembershipPopulation
        } else {
            QueryKind::NamespacePopulation
        };
        let population_rule = if imported_projection {
            if corrected {
                Rule::OperationalImportedMembershipsV1
            } else {
                Rule::PublishedImportedMemberships
            }
        } else {
            Rule::NamespaceResolution
        };
        out.prove(
            population_query,
            namespace,
            namespace,
            population_rule,
            premises,
        );
        for member in out.value.clone() {
            self.fact(&mut out, FactKey::Element(member.membership));
            out.prove(
                if imported_projection {
                    QueryKind::ImportedMemberships
                } else {
                    QueryKind::NamespaceMemberships
                },
                namespace,
                member.membership,
                population_rule,
                [
                    crate::contract::claim(population_query, namespace, namespace),
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
