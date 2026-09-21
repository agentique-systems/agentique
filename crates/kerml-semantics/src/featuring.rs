//! Structural featuring domains, including the role-scoped KERML11-8 check.
use crate::*;
use agq_kerml::{classes as c, properties as p};
use agq_kernel::{
    ElementId,
    provenance::{FactKey, Origin},
    value::Value,
};
use std::collections::BTreeSet;

/// A connector check retains ordinary and operational endpoint decisions separately.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConnectorFeaturing {
    pub connector: ElementId,
    pub domains: Vec<ElementId>,
    pub endpoints: Vec<ElementId>,
    pub ordinary_valid: Vec<bool>,
    pub exception_applied: Vec<bool>,
    pub role: Option<ImpliedBindingRole>,
    pub valid: bool,
}

impl KerMlQueries<'_> {
    /// KerML 1.0 Feature::isFeaturingType. Variable Features require the
    /// canonical snapshots role, or a redefinition featured by their owner.
    /// This predicate checks a candidate; it never invents a snapshot Feature.
    pub fn is_featuring_type(&self, feature: ElementId, candidate: ElementId) -> QueryResult<bool> {
        let mut out = self.result(false);
        if self
            .checked::<agq_kerml::views::Feature, _>(&mut out, feature)
            .is_none()
            || self
                .checked::<agq_kerml::views::Type, _>(&mut out, candidate)
                .is_none()
        {
            return out;
        }
        let owner = self.owning_type(feature);
        let owning_type = owner.value;
        out.merge(owner);
        let Some(owner) = owning_type else {
            return out;
        };
        match self.read_value(&mut out, feature, p::FEATURE_IS_VARIABLE) {
            Some(Value::Boolean(false)) => out.value = candidate == owner,
            Some(Value::Boolean(true)) => {
                out.search_dependencies
                    .insert(SearchDependency::StandardLibraries);
                let Some(bindings) = self.context().standard_bindings.as_ref() else {
                    out.problem(
                        Completeness::Incomplete,
                        "KQ_SNAPSHOT_BINDING",
                        feature,
                        "Variable featuring requires validated standard roles",
                    );
                    return out;
                };
                let snapshots = bindings.get(StandardRole::OccurrenceSnapshots);
                self.fact(&mut out, FactKey::Element(snapshots));
                self.fact(
                    &mut out,
                    FactKey::Element(bindings.get(StandardRole::Occurrence)),
                );
                if owner == bindings.get(StandardRole::Occurrence) {
                    out.value = candidate == snapshots;
                } else if self.is(candidate, c::FEATURE) {
                    let redefined = self.all_redefined_features(candidate);
                    let is_snapshot = redefined.value.contains(&snapshots);
                    out.merge(redefined);
                    if is_snapshot {
                        let domains = self.featuring_types(candidate);
                        out.value = domains.value.contains(&owner);
                        out.merge(domains);
                    }
                }
            }
            _ => out.problem(
                Completeness::Incomplete,
                "KQ_VARIABLE_FEATURING",
                feature,
                "Variable flag is not established",
            ),
        }
        out
    }

    /// Reflexive semantic supertype closure, including conjugation and chain terminals.
    pub fn all_supertypes(&self, ty: ElementId) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        let mut pending = vec![ty];
        let mut seen = BTreeSet::new();
        while let Some(current) = pending.pop() {
            if !seen.insert(current) {
                continue;
            }
            self.fact(&mut out, FactKey::Element(current));
            let parents = self.supertypes(current);
            pending.extend(parents.value.iter().copied());
            out.merge(parents);
        }
        out.value = seen.into_iter().collect();
        out
    }
    /// Transitive redefinition identity set, without copying inherited Features.
    pub fn all_redefined_features(&self, feature: ElementId) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        let mut pending = vec![feature];
        let mut seen = BTreeSet::new();
        while let Some(current) = pending.pop() {
            if !seen.insert(current) {
                continue;
            }
            let direct = self.redefined_features(current);
            out.value.extend(direct.value.iter().copied());
            pending.extend(direct.value.iter().copied());
            out.merge(direct);
        }
        out.value.sort();
        out.value.dedup();
        out
    }
    /// The canonical FeatureChaining order, never a sort of endpoint identities.
    pub fn chaining_features(&self, feature: ElementId) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        let owned = self.owned_relationships(feature);
        for &relationship in &owned.value {
            if self.is(relationship, c::FEATURE_CHAINING) {
                if let Some(target) = self.read_reference(
                    &mut out,
                    relationship,
                    p::FEATURE_CHAINING_CHAINING_FEATURE,
                ) {
                    out.value.push(target);
                } else {
                    out.problem(
                        Completeness::Incomplete,
                        "KQ_CHAIN_ENDPOINT",
                        relationship,
                        "Missing chaining feature",
                    );
                }
            }
        }
        out.merge(owned);
        out
    }

    /// Direct featuring domains, including required membership and first-chain consequences.
    pub fn featuring_types(&self, feature: ElementId) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        let mut pending = vec![feature];
        let mut visited = BTreeSet::new();
        while let Some(current) = pending.pop() {
            if !visited.insert(current) {
                continue;
            }
            if !self.is(current, c::FEATURE) {
                continue;
            }
            self.fact(&mut out, FactKey::Element(current));
            let owned = self.owned_relationships(current);
            let mut explicit = false;
            for &r in &owned.value {
                if self.is(r, c::TYPE_FEATURING) {
                    if let Some(ty) =
                        self.read_reference(&mut out, r, p::TYPE_FEATURING_FEATURING_TYPE)
                    {
                        out.value.push(ty);
                        explicit = true;
                    } else {
                        out.problem(
                            Completeness::Incomplete,
                            "KQ_FEATURING_ENDPOINT",
                            r,
                            "Missing featuring type",
                        );
                    }
                }
            }
            out.merge(owned);
            if self
                .context()
                .options
                .baseline_profile
                .corrects_owned_cross_domain()
            {
                let domain = self.owned_cross_feature_domain(current);
                if let Some(domain) = &domain.value {
                    if let [factor] = domain.factors.as_slice() {
                        out.value.extend(factor.types.iter().copied());
                    } else if domain.factors.len() > 1 && !explicit {
                        out.problem(
                            Completeness::Incomplete,
                            "KQ_CROSS_CARTESIAN_DOMAIN",
                            current,
                            "The required n-ary Cartesian domain has not been produced",
                        );
                    }
                }
                out.merge(domain);
            }
            let chain = self.chaining_features(current);
            if let Some(&first) = chain.value.first() {
                if first == current {
                    out.problem(
                        Completeness::Invalid,
                        "KQ_FEATURING_CYCLE",
                        current,
                        "Self feature chain",
                    );
                } else {
                    pending.push(first);
                }
            }
            out.merge(chain);
            let owner = self.owning_type(current);
            if let Some(ty) = owner.value {
                match self.read_value(&mut out, current, p::FEATURE_IS_VARIABLE) {
                    Some(Value::Boolean(false)) => out.value.push(ty),
                    Some(Value::Boolean(true)) if explicit => {}
                    Some(Value::Boolean(true))
                        if self
                            .context()
                            .standard_bindings
                            .as_ref()
                            .is_some_and(|b| b.get(StandardRole::Occurrence) == ty) =>
                    {
                        // Feature::isFeaturingType has a specific canonical
                        // Occurrence case; no per-feature snapshot type is needed.
                        out.search_dependencies
                            .insert(SearchDependency::StandardLibraries);
                        let snapshots = self
                            .context()
                            .standard_bindings
                            .as_ref()
                            .unwrap()
                            .get(StandardRole::OccurrenceSnapshots);
                        self.fact(&mut out, FactKey::Element(snapshots));
                        out.value.push(snapshots);
                    }
                    _ => out.problem(
                        Completeness::Incomplete,
                        "KQ_VARIABLE_FEATURING",
                        current,
                        "Variable featuring requires its snapshot domain",
                    ),
                }
            }
            out.merge(owner);
            if self.is(current, c::EXPRESSION) {
                let membership = self.owning_relationship(current);
                if let Some(m) = membership.value.filter(|m| self.is(*m, c::FEATURE_VALUE)) {
                    let owner = self.owning_related_element(m);
                    pending.extend(owner.value);
                    out.merge(owner);
                }
                out.merge(membership);
            }
        }
        out.value.sort();
        out.value.dedup();
        out
    }

    fn compatible<T>(
        &self,
        out: &mut QueryResult<T>,
        ty: ElementId,
        other: ElementId,
        active: &mut BTreeSet<(ElementId, ElementId)>,
    ) -> bool {
        if ty == other {
            self.fact(out, FactKey::Element(ty));
            return true;
        }
        let generals = self.all_supertypes(ty);
        let specializes = generals.value.contains(&other);
        out.merge(generals);
        if specializes {
            return true;
        }
        if !self.is(ty, c::FEATURE) || !self.is(other, c::FEATURE) {
            return false;
        }
        let own = self.direct_features(ty);
        let other_own = self.direct_features(other);
        let empty = own.value.is_empty() && other_own.value.is_empty();
        out.merge(own);
        out.merge(other_own);
        if !empty {
            return false;
        }
        let left = self.all_redefined_features(ty);
        let right = self.all_redefined_features(other);
        let common = left.value.iter().any(|f| right.value.contains(f));
        out.merge(left);
        out.merge(right);
        common && self.can_access_inner(out, ty, other, active)
    }

    fn featured_inner<T>(
        &self,
        out: &mut QueryResult<T>,
        feature: ElementId,
        domain: Option<ElementId>,
        active: &mut BTreeSet<(ElementId, ElementId)>,
    ) -> bool {
        let featuring = self.featuring_types(feature);
        let mut valid = true;
        for &ty in &featuring.value {
            valid &= if let Some(domain) = domain {
                self.compatible(out, domain, ty, active)
            } else if let Some(bindings) = &self.context().standard_bindings {
                out.search_dependencies
                    .insert(SearchDependency::StandardLibraries);
                ty == bindings.get(StandardRole::Anything)
            } else {
                out.problem(
                    Completeness::Incomplete,
                    "KQ_ANYTHING_BINDING",
                    feature,
                    "Null domain needs the canonical Base::Anything binding",
                );
                false
            };
        }
        out.merge(featuring);
        if valid {
            return true;
        }
        if let Some(domain) = domain {
            let chain = self.chaining_features(feature);
            let first = chain.value.first().copied().unwrap_or(feature);
            out.merge(chain);
            if matches!(
                self.read_value(out, first, p::FEATURE_IS_VARIABLE),
                Some(Value::Boolean(true))
            ) {
                let owner = self.owning_type(first);
                let matches = owner
                    .value
                    .is_some_and(|ty| self.compatible(out, domain, ty, active));
                out.merge(owner);
                return matches;
            }
        }
        false
    }

    /// Published Feature::isFeaturedWithin. No operational binding exception occurs here.
    pub fn is_featured_within(
        &self,
        feature: ElementId,
        domain: Option<ElementId>,
    ) -> QueryResult<bool> {
        let mut out = self.result(false);
        out.value = self.featured_inner(&mut out, feature, domain, &mut BTreeSet::new());
        out
    }

    fn can_access_inner<T>(
        &self,
        out: &mut QueryResult<T>,
        feature: ElementId,
        other: ElementId,
        active: &mut BTreeSet<(ElementId, ElementId)>,
    ) -> bool {
        if !active.insert((feature, other)) {
            out.problem(
                Completeness::Incomplete,
                "KQ_DOMAIN_RECURSION",
                feature,
                "Cyclic compatibility evidence",
            );
            return false;
        }
        let mut pending = vec![feature];
        let mut seen = BTreeSet::new();
        let mut valid = false;
        while let Some(next) = pending.pop() {
            if !seen.insert(next) {
                continue;
            }
            let domains = self.featuring_types(next);
            if domains.value.is_empty() {
                valid |= self.featured_inner(out, other, None, active);
            }
            for &ty in &domains.value {
                valid |= self.featured_inner(out, other, Some(ty), active);
                if self.is(ty, c::FEATURE) {
                    pending.push(ty);
                }
            }
            out.merge(domains);
        }
        active.remove(&(feature, other));
        valid
    }

    /// Published access through direct or indirect featuring domains.
    pub fn can_access(&self, feature: ElementId, other: ElementId) -> QueryResult<bool> {
        let mut out = self.result(false);
        out.value = self.can_access_inner(&mut out, feature, other, &mut BTreeSet::new());
        out
    }

    /// The exact first non-parameter owned membership, with its unresolved evidence retained.
    pub fn reference_referent(&self, expression: ElementId) -> QueryResult<Option<ElementId>> {
        let mut out = self.result(None);
        let members = self.memberships(expression);
        if let Some(&membership) = members
            .value
            .iter()
            .find(|m| !self.is(**m, c::PARAMETER_MEMBERSHIP))
        {
            let member = self.member(membership);
            out.value = member.value;
            out.merge(member);
        }
        out.merge(members);
        if out.value.is_none() {
            out.problem(
                Completeness::Incomplete,
                "KQ_REFERENCE_REFERENT",
                expression,
                "Reference expression has no established referent",
            );
        }
        out
    }

    /// Canonical endpoint sequence through end Features and ReferenceSubsetting.
    pub fn connector_endpoints(&self, connector: ElementId) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        let ends = self.structural_end_features(connector);
        for &end in &ends.value {
            let owned = self.owned_relationships(end);
            let refs: Vec<_> = owned
                .value
                .iter()
                .filter(|r| self.is(**r, c::REFERENCE_SUBSETTING))
                .copied()
                .collect();
            if let [r] = refs.as_slice() {
                if let Some(target) =
                    self.read_reference(&mut out, *r, p::REFERENCE_SUBSETTING_REFERENCED_FEATURE)
                {
                    out.value.push(target);
                }
            } else {
                out.problem(
                    Completeness::Incomplete,
                    "KQ_CONNECTOR_ENDPOINT",
                    end,
                    "End requires a unique reference subsetting",
                );
            }
            out.merge(owned);
        }
        out.merge(ends);
        out
    }

    /// Recover a typed origin from rule provenance; containment alone grants no role.
    pub fn implied_binding_role(&self, connector: ElementId) -> Option<ImpliedBindingRole> {
        let Origin::Derived(origin) = self.model().element(connector)?.origin() else {
            return None;
        };
        ImpliedBindingRole::ALL
            .into_iter()
            .find(|role| role.rule_id(self.context().options.baseline_profile) == origin.rule)
    }

    /// Ordinary connector conformance, with only the reviewed raw-result endpoint exception.
    pub fn validate_connector_featuring(
        &self,
        connector: ElementId,
    ) -> QueryResult<ConnectorFeaturing> {
        let role = self.implied_binding_role(connector);
        let mut out = self.result(ConnectorFeaturing {
            connector,
            domains: vec![],
            endpoints: vec![],
            ordinary_valid: vec![],
            exception_applied: vec![],
            role,
            valid: false,
        });
        let endpoints = self.connector_endpoints(connector);
        out.value.endpoints = endpoints.value.clone();
        out.merge(endpoints);
        let domains = self.featuring_types(connector);
        out.value.domains = domains.value.clone();
        out.merge(domains);
        let owner = self.owner(connector);
        let expression = owner.value;
        out.merge(owner);
        let implied = matches!(
            self.read_value(&mut out, connector, p::RELATIONSHIP_IS_IMPLIED),
            Some(Value::Boolean(true))
        );
        let eligible = role == Some(ImpliedBindingRole::FeatureReferenceResult)
            && implied
            && self.is(connector, c::BINDING_CONNECTOR)
            && out.value.endpoints.len() == 2
            && self
                .context()
                .options
                .baseline_profile
                .corrects_reference_binding()
            && expression.is_some_and(|e| self.is(e, c::FEATURE_REFERENCE_EXPRESSION));
        let mut qualified = None;
        if eligible {
            let expression = expression.expect("checked");
            let referent = self.reference_referent(expression);
            let result = self.result_parameters(expression);
            if let (Some(referent), [raw]) = (referent.value, result.value.as_slice()) {
                let owning = self.owning_type(*raw);
                let membership = self.owning_relationship(*raw);
                let domain = self.reference_binding_context(expression, referent, *raw);
                if owning.value == Some(expression)
                    && membership
                        .value
                        .is_some_and(|m| self.is(m, c::RETURN_PARAMETER_MEMBERSHIP))
                    && out.value.endpoints == [referent, *raw]
                    && domain.value.is_some_and(|d| out.value.domains == [d])
                    && domain.completeness == Completeness::Complete
                {
                    qualified = Some(*raw);
                }
                out.merge(owning);
                out.merge(membership);
                out.merge(domain);
            }
            out.merge(referent);
            out.merge(result);
        }
        for endpoint in out.value.endpoints.clone() {
            let domains: Vec<_> = if out.value.domains.is_empty() {
                vec![None]
            } else {
                out.value.domains.iter().copied().map(Some).collect()
            };
            let mut valid = true;
            for domain in domains {
                let check = self.is_featured_within(endpoint, domain);
                valid &= check.value;
                out.merge(check);
            }
            out.value.ordinary_valid.push(valid);
            out.value
                .exception_applied
                .push(!valid && qualified == Some(endpoint));
        }
        out.value.valid = out
            .value
            .ordinary_valid
            .iter()
            .zip(&out.value.exception_applied)
            .all(|(a, b)| *a || *b)
            && (!self.is(connector, c::BINDING_CONNECTOR) || out.value.endpoints.len() == 2);
        out.search_dependencies
            .insert(SearchDependency::ValidationRule(
                "checkConnectorTypeFeaturing",
            ));
        out.search_dependencies
            .insert(SearchDependency::ImpliedBindingRole(role));
        let premises = out
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
            .collect::<Vec<_>>();
        for (index, endpoint) in out.value.endpoints.clone().into_iter().enumerate() {
            if out.value.ordinary_valid[index] || out.value.exception_applied[index] {
                out.prove(
                    QueryKind::ConnectorFeaturing,
                    connector,
                    endpoint,
                    if out.value.exception_applied[index] {
                        Rule::ImpliedBinding(ImpliedBindingRole::FeatureReferenceResult)
                    } else {
                        Rule::ConnectorFeaturing
                    },
                    premises.clone(),
                );
            }
        }
        if !out.value.valid {
            out.problem(
                Completeness::Invalid,
                "checkConnectorTypeFeaturing",
                connector,
                "Related feature is outside the connector domain or the binding is not binary",
            );
        }
        out
    }

    /// Exact context selection for the reference-result role. Multiple nearest
    /// candidates remain incomplete; their identity ordering never picks a winner.
    pub fn reference_binding_context(
        &self,
        expression: ElementId,
        referent: ElementId,
        raw: ElementId,
    ) -> QueryResult<Option<ElementId>> {
        let mut out = self.result(None);
        let ordinary = self.common_connector_context(&[referent, raw]);
        out.value = ordinary.value;
        let complete = ordinary.completeness == Completeness::Complete;
        out.merge(ordinary);
        if out.value.is_some()
            || !complete
            || !self
                .context()
                .options
                .baseline_profile
                .corrects_reference_binding()
        {
            return out;
        }
        let candidates = self.featuring_closure(&[expression]);
        let mut valid = vec![];
        for &candidate in &candidates.value {
            let check = self.is_featured_within(referent, Some(candidate));
            if check.value && check.completeness == Completeness::Complete {
                valid.push(candidate);
            }
            out.merge(check);
        }
        out.merge(candidates);
        let nearest = self.nearest_context(&valid);
        out.value = nearest.value;
        out.merge(nearest);
        if out.value.is_none() {
            out.problem(
                Completeness::Incomplete,
                "KQ_REFERENCE_CONTEXT",
                expression,
                "No unique expression context features the referent",
            );
        }
        out
    }

    fn featuring_closure(&self, features: &[ElementId]) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        let mut pending = features.to_vec();
        let mut seen = BTreeSet::new();
        while let Some(next) = pending.pop() {
            if !seen.insert(next) || !self.is(next, c::FEATURE) {
                continue;
            }
            let featuring = self.featuring_types(next);
            out.value.extend(featuring.value.iter().copied());
            pending.extend(featuring.value.iter().copied());
            out.merge(featuring);
        }
        out.value.sort();
        out.value.dedup();
        out
    }
    fn nearest_context(&self, candidates: &[ElementId]) -> QueryResult<Option<ElementId>> {
        let mut out = self.result(None);
        let mut nearest = candidates.to_vec();
        for &candidate in candidates {
            let outer = self.featuring_closure(&[candidate]);
            nearest.retain(|other| *other == candidate || !outer.value.contains(other));
            out.merge(outer);
        }
        match nearest.as_slice() {
            [unique] => out.value = Some(*unique),
            [] => {}
            _ => out.problem(
                Completeness::Incomplete,
                "KQ_AMBIGUOUS_CONNECTOR_CONTEXT",
                candidates[0],
                "Multiple incomparable nearest featuring contexts",
            ),
        }
        out
    }
    /// Ordinary defaultFeaturingType candidate computation, without profile exceptions.
    pub fn common_connector_context(
        &self,
        endpoints: &[ElementId],
    ) -> QueryResult<Option<ElementId>> {
        let mut out = self.result(None);
        let candidates = self.featuring_closure(endpoints);
        let mut valid = vec![];
        for &candidate in &candidates.value {
            let mut ok = true;
            for &endpoint in endpoints {
                let check = self.is_featured_within(endpoint, Some(candidate));
                ok &= check.value;
                out.merge(check);
            }
            if ok {
                valid.push(candidate);
            }
        }
        out.merge(candidates);
        let nearest = self.nearest_context(&valid);
        out.value = nearest.value;
        out.merge(nearest);
        out
    }
}
