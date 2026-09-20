//! Exact, reviewed formal-constraint targets. Names bind once; rules use identities.
use crate::*;
use agq_kerml::{BaselineProfile, classes as c, properties as p};
use agq_kernel::{
    ElementId, LibraryId, MetaclassId,
    provenance::{DeclaredOrigin, FactKey, Origin},
    value::Value,
};
use std::collections::{BTreeMap, BTreeSet};

/// Closed identities from the pinned KerML 1.0 XMI, not validator display names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FormalConstraintId {
    FeatureSubobjectSpecialization,
    StepSubperformanceSpecialization,
    FeaturePortionSpecialization,
    FeatureSuboccurrenceSpecialization,
    StepOwnedPerformanceSpecialization,
    StepEnclosedPerformanceSpecialization,
}

/// Published and reviewed path contract, before canonical binding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormalConstraintTarget {
    pub rule_id: &'static str,
    pub published_target: [&'static str; 3],
    pub operational_target: [&'static str; 3],
    pub authority: &'static str,
    pub owner_metaclass: MetaclassId,
    pub target_metaclass: MetaclassId,
}
impl FormalConstraintId {
    pub const ALL: [Self; 6] = [
        Self::FeatureSubobjectSpecialization,
        Self::StepSubperformanceSpecialization,
        Self::FeaturePortionSpecialization,
        Self::FeatureSuboccurrenceSpecialization,
        Self::StepOwnedPerformanceSpecialization,
        Self::StepEnclosedPerformanceSpecialization,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::FeatureSubobjectSpecialization => "checkFeatureSubobjectSpecialization",
            Self::StepSubperformanceSpecialization => "checkStepSubperformanceSpecialization",
            Self::FeaturePortionSpecialization => "checkFeaturePortionSpecialization",
            Self::FeatureSuboccurrenceSpecialization => "checkFeatureSuboccurrenceSpecialization",
            Self::StepOwnedPerformanceSpecialization => "checkStepOwnedPerformanceSpecialization",
            Self::StepEnclosedPerformanceSpecialization => {
                "checkStepEnclosedPerformanceSpecialization"
            }
        }
    }
    pub const fn target(self) -> FormalConstraintTarget {
        use FormalConstraintId::*;
        let (
            rule_id,
            published_target,
            operational_target,
            authority,
            owner_metaclass,
            target_metaclass,
        ) = match self {
            FeatureSubobjectSpecialization => (
                "Core-Features-Feature-checkFeatureSubobjectSpecialization",
                ["Occurrence", "Occurrence", "suboccurrences"],
                ["Objects", "Object", "subobjects"],
                "KERML11-205",
                c::STRUCTURE,
                c::FEATURE,
            ),
            StepSubperformanceSpecialization => (
                "Kernel-Behaviors-Step-checkStepSubperformanceSpecialization",
                ["Performances", "Performance", "subperformance"],
                ["Performances", "Performance", "subperformances"],
                "KERML11-205",
                c::BEHAVIOR,
                c::STEP,
            ),
            FeaturePortionSpecialization => (
                "Core-Features-Feature-checkFeaturePortionSpecialization",
                ["Occurrence", "Occurrence", "portions"],
                ["Occurrences", "Occurrence", "portions"],
                "KERML11-206",
                c::CLASS,
                c::FEATURE,
            ),
            FeatureSuboccurrenceSpecialization => (
                "Core-Features-Feature-checkFeatureSuboccurrenceSpecialization",
                ["Occurrence", "Occurrence", "suboccurrences"],
                ["Occurrences", "Occurrence", "suboccurrences"],
                "KERML11-206",
                c::CLASS,
                c::FEATURE,
            ),
            StepOwnedPerformanceSpecialization => (
                "Kernel-Behaviors-Step-checkStepOwnedPerformanceSpecialization",
                ["Objects", "Object", "ownedPerformance"],
                ["Objects", "Object", "ownedPerformances"],
                "KERML11-207",
                c::STRUCTURE,
                c::STEP,
            ),
            StepEnclosedPerformanceSpecialization => (
                "Kernel-Behaviors-Step-checkStepEnclosedPerformanceSpecialization",
                ["Performances", "Performance", "enclosedPerformance"],
                ["Performances", "Performance", "enclosedPerformances"],
                "KERML11-207",
                c::BEHAVIOR,
                c::STEP,
            ),
        };
        FormalConstraintTarget {
            rule_id,
            published_target,
            operational_target,
            authority,
            owner_metaclass,
            target_metaclass,
        }
    }
    pub fn effective_path(self, profile: BaselineProfile) -> [&'static str; 3] {
        let target = self.target();
        if profile.corrects_formal_constraint_targets() {
            target.operational_target
        } else {
            target.published_target
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Binding {
    target: Option<ElementId>,
    completeness: Completeness,
    diagnostics: BTreeSet<Diagnostic>,
    facts: BTreeSet<FactKey>,
    searches: BTreeSet<SearchDependency>,
}

/// Bound to one immutable graph/profile. Missing targets stay explicit, including
/// the historical published miss; there is no alternate-path search.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FormalConstraintTargets {
    profile: BaselineProfile,
    model_digest: [u8; 32],
    library: LibraryId,
    bindings: BTreeMap<FormalConstraintId, Binding>,
}

impl<'m> SemanticContext<'m> {
    /// Resolve all six exact path contracts against canonical owned memberships.
    /// Failure records are retained so an applicable rule can never silently pass.
    pub fn with_formal_constraint_targets(self, roots: &[ElementId], library: LibraryId) -> Self {
        let queries = KerMlQueries::new(self);
        let bindings = FormalConstraintId::ALL
            .into_iter()
            .map(|rule| {
                let result = queries.bind_formal_target(rule, roots, library);
                (
                    rule,
                    Binding {
                        target: result.value,
                        completeness: result.completeness,
                        diagnostics: result.diagnostics,
                        facts: result.positive_dependencies,
                        searches: result.search_dependencies,
                    },
                )
            })
            .collect();
        let mut context = queries.context;
        context.id.formal_constraint_targets = Some(std::sync::Arc::new(FormalConstraintTargets {
            profile: context.id.options.baseline_profile,
            model_digest: context.id.model_digest,
            library,
            bindings,
        }));
        context
    }
}

impl KerMlQueries<'_> {
    fn bind_formal_target(
        &self,
        rule: FormalConstraintId,
        roots: &[ElementId],
        library: LibraryId,
    ) -> QueryResult<Option<ElementId>> {
        let mut out = self.result(None);
        let profile = self.context().options.baseline_profile;
        let contract = rule.target();
        let path = rule.effective_path(profile);
        let classes = [
            c::LIBRARY_PACKAGE,
            // KERML11-205's published target is in Occurrence, not Object.
            if rule == FormalConstraintId::FeatureSubobjectSpecialization
                && !profile.corrects_formal_constraint_targets()
            {
                c::CLASS
            } else {
                contract.owner_metaclass
            },
            contract.target_metaclass,
        ];
        let mut scopes = roots.to_vec();
        out.search_dependencies
            .insert(SearchDependency::FormalConstraintTarget(rule));
        out.search_dependencies
            .insert(SearchDependency::StandardLibraries);
        for (index, segment) in path.iter().enumerate() {
            let mut candidates = vec![];
            for &scope in &scopes {
                let members = self.memberships(scope);
                for &membership in &members.value {
                    if !self.is(membership, c::OWNING_MEMBERSHIP) {
                        continue;
                    }
                    let member = self.member(membership);
                    if let Some(element) = member.value
                        && matches!(self.read_value(&mut out, element, p::ELEMENT_DECLARED_NAME), Some(Value::String(name)) if name == segment)
                    {
                        candidates.push(element);
                        if !self.visible(
                            &mut out,
                            membership,
                            p::MEMBERSHIP_VISIBILITY,
                            MemberAccess::Public,
                        ) {
                            out.problem(
                                Completeness::Invalid,
                                "KQ_FORMAL_TARGET_VISIBILITY",
                                element,
                                contract.rule_id,
                            );
                        }
                        for id in [membership, element] {
                            self.fact(&mut out, FactKey::Element(id));
                            if self.model().element(id).unwrap().origin()
                                != &Origin::Declared(DeclaredOrigin::StandardLibrary { library })
                            {
                                out.problem(
                                    Completeness::Invalid,
                                    "KQ_FORMAL_TARGET_LIBRARY",
                                    id,
                                    contract.rule_id,
                                );
                            }
                        }
                    }
                    out.merge(member);
                }
                out.merge(members);
            }
            let [element] = candidates.as_slice() else {
                let subject = scopes.first().copied().unwrap_or(ElementId::from_u128(0));
                out.problem(
                    if out.completeness == Completeness::Complete {
                        Completeness::Invalid
                    } else {
                        out.completeness
                    },
                    if candidates.is_empty() {
                        "KQ_FORMAL_TARGET_MISSING"
                    } else {
                        "KQ_FORMAL_TARGET_AMBIGUOUS"
                    },
                    subject,
                    format!(
                        "{} requires exact {} under {}",
                        contract.rule_id,
                        path.join("::"),
                        profile.id()
                    ),
                );
                return out;
            };
            if self.model().element(*element).unwrap().metaclass() != classes[index] {
                out.problem(
                    Completeness::Invalid,
                    "KQ_FORMAL_TARGET_METACLASS",
                    *element,
                    contract.rule_id,
                );
            }
            if out.completeness != Completeness::Complete {
                return out;
            }
            scopes = vec![*element];
        }
        out.value = scopes.first().copied();
        out
    }

    /// Read the bound canonical target with the exact rule/profile/provenance proof.
    pub fn formal_constraint_target(
        &self,
        rule: FormalConstraintId,
    ) -> QueryResult<Option<ElementId>> {
        let mut out = self.result(None);
        out.search_dependencies
            .insert(SearchDependency::FormalConstraintTarget(rule));
        out.search_dependencies
            .insert(SearchDependency::StandardLibraries);
        let Some(targets) = self.context().formal_constraint_targets.as_ref() else {
            out.problem(
                Completeness::Incomplete,
                "KQ_FORMAL_TARGET_UNBOUND",
                ElementId::from_u128(0),
                rule.target().rule_id,
            );
            return out;
        };
        let binding = &targets.bindings[&rule];
        out.value = binding.target;
        out.completeness = binding.completeness;
        out.diagnostics.extend(binding.diagnostics.iter().cloned());
        out.search_dependencies
            .extend(binding.searches.iter().cloned());
        for &fact in &binding.facts {
            self.fact(&mut out, fact);
        }
        if let Some(target) = out.value {
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
            out.prove(
                QueryKind::FormalConstraintTarget,
                target,
                target,
                if targets.profile.corrects_formal_constraint_targets() {
                    Rule::OperationalFormalConstraintTargetV1
                } else {
                    Rule::PublishedFormalConstraintTarget
                },
                premises,
            );
        }
        out
    }

    /// Structural antecedents from the pinned normative prose. This is not an
    /// OCL interpreter; the frozen matrix retains the literal formal bodies.
    pub fn formal_constraint_applies(
        &self,
        rule: FormalConstraintId,
        feature: ElementId,
    ) -> QueryResult<bool> {
        use FormalConstraintId::*;
        let mut out = self.result(false);
        let step_rule = matches!(
            rule,
            StepSubperformanceSpecialization
                | StepOwnedPerformanceSpecialization
                | StepEnclosedPerformanceSpecialization
        );
        if !self.is(feature, if step_rule { c::STEP } else { c::FEATURE }) {
            return out;
        }
        self.fact(&mut out, FactKey::Element(feature));
        let flag = match rule {
            FeaturePortionSpecialization => Some(p::FEATURE_IS_PORTION),
            StepEnclosedPerformanceSpecialization => None,
            _ => Some(p::FEATURE_IS_COMPOSITE),
        };
        if let Some(flag) = flag {
            match self.read_value(&mut out, feature, flag) {
                Some(Value::Boolean(true)) => {}
                Some(Value::Boolean(false)) => return out,
                _ => {
                    out.problem(
                        Completeness::Incomplete,
                        "KQ_FORMAL_ANTECEDENT",
                        feature,
                        "Missing required feature flag",
                    );
                    return out;
                }
            }
        }
        let class = if matches!(
            rule,
            FeatureSubobjectSpecialization | StepOwnedPerformanceSpecialization
        ) {
            c::STRUCTURE
        } else {
            c::CLASS
        };
        // A complete negative owned-typing prerequisite is conclusive before
        // the owning Feature's potentially incomplete effective type closure.
        if !step_rule {
            let owned = self.owned_relationships(feature);
            let mut typed = false;
            for &relationship in &owned.value {
                if self.is(relationship, c::FEATURE_TYPING)
                    && let Some(ty) =
                        self.read_reference(&mut out, relationship, p::FEATURE_TYPING_TYPE)
                {
                    typed |= self.is(ty, class);
                }
            }
            out.merge(owned);
            if !typed {
                return out;
            }
        }
        let owner = self.owning_type(feature);
        let owning_type = owner.value;
        out.merge(owner);
        let Some(owner) = owning_type else {
            return out;
        };
        if matches!(
            rule,
            StepSubperformanceSpecialization | StepEnclosedPerformanceSpecialization
        ) {
            out.value = self.is(owner, c::BEHAVIOR) || self.is(owner, c::STEP);
            return out;
        }
        let mut owner_matches = self.is(owner, class);
        if !owner_matches && self.is(owner, c::FEATURE) {
            let types = self.direct_feature_types(owner);
            owner_matches = types.value.iter().any(|&t| self.is(t, class));
            out.merge(types);
            if !owner_matches {
                // A negative direct-typing answer does not establish the full
                // derived Feature::type closure through subsetting and chains.
                out.problem(
                    Completeness::Incomplete,
                    "KQ_FORMAL_ANTECEDENT_TYPE_CLOSURE",
                    owner,
                    "Effective owner typing is required to establish this antecedent",
                );
            }
        }
        if !owner_matches {
            return out;
        }
        out.value = true;
        out
    }

    pub(crate) fn formal_constraint_specializations(
        &self,
        feature: ElementId,
    ) -> QueryResult<Vec<ElementId>> {
        let mut out = self.result(vec![]);
        if self.context().formal_constraint_targets.is_none() {
            return out;
        }
        for rule in FormalConstraintId::ALL {
            let applies = self.formal_constraint_applies(rule, feature);
            let applicable = applies.value;
            out.merge(applies);
            if applicable {
                let target = self.formal_constraint_target(rule);
                let target_id = target.value.filter(|&id| id != feature);
                out.merge(target);
                if let Some(target) = target_id {
                    out.value.push(target);
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
                    out.prove(
                        QueryKind::DirectSpecializations,
                        feature,
                        target,
                        if self
                            .context()
                            .options
                            .baseline_profile
                            .corrects_formal_constraint_targets()
                        {
                            Rule::OperationalFormalConstraintTargetV1
                        } else {
                            Rule::PublishedFormalConstraintTarget
                        },
                        premises,
                    );
                }
            }
        }
        out
    }

    /// Validate each applicable exact rule, including reflexive specialization.
    pub fn validate_formal_target_constraints(
        &self,
        feature: ElementId,
    ) -> QueryResult<Vec<&'static str>> {
        let mut out = self.result(vec![]);
        for rule in FormalConstraintId::ALL {
            let applies = self.formal_constraint_applies(rule, feature);
            out.value.push(rule.name());
            if applies.value {
                let target = self.formal_constraint_target(rule);
                if let Some(target_id) = target.value {
                    let supers = self.all_specializations(feature);
                    if target_id != feature && !supers.value.contains(&target_id) {
                        out.problem(
                            Completeness::Invalid,
                            rule.name(),
                            feature,
                            format!("Required canonical target {target_id}"),
                        );
                    }
                    out.merge(supers);
                }
                out.merge(target);
            }
            out.merge(applies);
        }
        out
    }
}
