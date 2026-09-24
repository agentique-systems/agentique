use crate::SemanticContextId;
use agq_kernel::{
    ElementId, MetaclassId, PropertyId,
    provenance::{DeclaredOrigin, Dependency, FactKey, Origin},
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Complete means complete for the named bounded query, not language validation success.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum Completeness {
    Complete,
    Incomplete,
    Invalid,
}

/// A bounded structural Feature population with language-defined membership
/// semantics. The stable contract identifies a query projection, not a model fact.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum FeaturePopulationKind {
    Parameter,
    End,
    Result,
}

impl FeaturePopulationKind {
    /// Opaque versioned identity retained by the language-neutral kernel.
    pub const fn contract_id(self) -> &'static str {
        match self {
            Self::Parameter => "agq-feature-population/Parameter/1",
            Self::End => "agq-feature-population/End/1",
            Self::Result => "agq-feature-population/Result/1",
        }
    }

    /// Recognize only the exact supported projection contract version.
    pub fn from_contract_id(id: &str) -> Option<Self> {
        match id {
            "agq-feature-population/Parameter/1" => Some(Self::Parameter),
            "agq-feature-population/End/1" => Some(Self::End),
            "agq-feature-population/Result/1" => Some(Self::Result),
            _ => None,
        }
    }
}

/// Set reads remain dependencies even when empty. Match pre- AND post-change state.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SearchDependency {
    /// A negative/exhaustive semantic conclusion reads scheduler closure, not a
    /// canonical fact. `None` preserves the missing-witness dependency.
    ProducerClosure {
        subject: ElementId,
        requirement: crate::SemanticClosureRequirement,
        certificate_digest: Option<[u8; 32]>,
        source: Option<crate::ClosureSource>,
    },
    /// The semantic producer role, including the absence of such provenance.
    ImpliedBindingRole(Option<crate::ImpliedBindingRole>),
    /// Explicit profile-scoped redefinition search, including unsuccessful scopes.
    RedefinitionScope {
        relationship: ElementId,
        namespace: ElementId,
        path: RedefinitionRulePath,
    },
    /// Generic overlay computation searches, including empty descriptor/navigation searches.
    Kernel(agq_kernel::derived::StructuralSearch),
    /// Existence and metaclass identity, excluding stored property populations.
    Element(ElementId),
    PropertySet {
        element: ElementId,
        property: PropertyId,
    },
    /// Every reference occurrence to this target (including source retargeting).
    Incoming {
        target: ElementId,
    },
    /// Relationship population restricted to a descriptor-resolved source role,
    /// including subtypes and property redefinitions, even when empty.
    SourceRelationships {
        source: ElementId,
        class: MetaclassId,
        property: PropertyId,
    },
    /// Direct owned relationship population restricted by registered metaclass,
    /// including subtypes and relationships with unresolved source endpoints.
    OwnedRelationships {
        owner: ElementId,
        class: MetaclassId,
    },
    /// Direct owned relationships in `class`, excluding every registered subtype
    /// of any class in `excluded`. Empty exclusions preserve the full population.
    /// Unknown class identities cannot establish a negative selection result.
    OwnedRelationshipsExcluding {
        owner: ElementId,
        class: MetaclassId,
        excluded: BTreeSet<MetaclassId>,
    },
    /// Direct owned member population selected by an exact structural projection.
    /// Canonical carrier facts remain proof support; this records the population
    /// whose future changes can alter the projected result, including absence.
    StructuralFeaturePopulation {
        owner: ElementId,
        kind: FeaturePopulationKind,
    },
    /// Owned membership population, including endpoint, visibility and name changes.
    NamespaceMembers {
        namespace: ElementId,
    },
    /// Import population and import visibility, recursion and target changes.
    ImportSet {
        namespace: ElementId,
    },
    /// A traversed namespace import, including misses in its target namespace.
    ImportedNamespace {
        import: ElementId,
        namespace: ElementId,
    },
    /// Available root namespaces at the project/dependency boundary.
    ProjectRoots {
        root: ElementId,
    },
    StandardLibraries,
    /// Reviewed exact target and the context's profile/manifest/library identities.
    FormalConstraintTarget(crate::FormalConstraintId),
    /// The selected validation algorithm and its reviewed profile/manifest context.
    ValidationRule(&'static str),
    Instances {
        class: MetaclassId,
    },
}

/// Paths in AGQ-KERML10-002, agentique-kerml10-redefinition-target/1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RedefinitionRulePath {
    ExplicitRoot,
    Inherited,
    LexicalContaining,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct Diagnostic {
    pub code: &'static str,
    pub subject: ElementId,
    pub message: String,
}

impl<'de> serde::Deserialize<'de> for Diagnostic {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        struct Stored {
            code: String,
            subject: ElementId,
            message: String,
        }
        let stored = Stored::deserialize(deserializer)?;
        Ok(Self {
            code: checkpoint_diagnostic_code::<D::Error>(stored.code)?,
            subject: stored.subject,
            message: stored.message,
        })
    }
}

// Codes are static in the query API. Authenticated checkpoint restoration interns
// their bounded vocabulary once; messages and graph populations are never leaked.
fn checkpoint_diagnostic_code<E: serde::de::Error>(code: String) -> Result<&'static str, E> {
    static CODES: std::sync::OnceLock<std::sync::Mutex<BTreeMap<String, &'static str>>> =
        std::sync::OnceLock::new();
    if code.len() > 256 {
        return Err(serde::de::Error::custom("diagnostic code length"));
    }
    let mut codes = CODES
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| serde::de::Error::custom("diagnostic code interner poisoned"))?;
    if let Some(code) = codes.get(&code) {
        return Ok(code);
    }
    if codes.len() >= 4096 {
        return Err(serde::de::Error::custom("diagnostic code vocabulary"));
    }
    let interned = Box::leak(code.clone().into_boxed_str());
    codes.insert(code, interned);
    Ok(interned)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum QueryKind {
    /// Subject and value both identify the certified semantic subject.
    ProducerClosure(crate::SemanticClosureRequirement),
    MultiplicityFeaturingContext,
    ImportedMemberships,
    ImportedMembershipPopulation,
    MultiplicityBound,
    OwnedRelationships,
    OwningRelationship,
    OwningRelatedElement,
    Owner,
    Memberships,
    Member,
    LookupDeclaredMember,
    ResolveReference,
    LookupMember,
    NamespaceMemberships,
    /// Whole membership population; subject and value are the namespace itself.
    NamespacePopulation,
    DirectFeatures,
    DirectSpecializations,
    Supertypes,
    AllSpecializations,
    DirectFeatureTypes,
    SubsettedFeatures,
    RedefinedFeatures,
    AllRedefinedFeatures,
    InheritedFeature,
    EffectiveFeatures,
    SuppressedFeature,
    OwningType,
    ExpressionResults,
    /// Whole return-parameter population; subject and value identify the Type,
    /// including when its proven collection is empty.
    ResultPopulation,
    NamingSource,
    RedefinitionEndConformance,
    FormalConstraintTarget,
    ConnectorFeaturing,
    OwnedCrossFeature,
    OwnedCrossDomain,
    OwnedCrossSubsetting,
    CrossFeature,
    TypingFeatures,
    FeatureTypes,
    FeatureTarget,
    FeatureWithValue,
}

/// Subject is part of every conclusion; a target ID alone cannot identify a proof.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Conclusion {
    pub query: QueryKind,
    pub subject: ElementId,
    pub value: ElementId,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Evidence {
    Fact(FactKey),
    Conclusion(Conclusion),
    Search(SearchDependency),
}

/// Stable within RULE_SET_VERSION. Exact normative anchors are in ADR 0005.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    /// Every relevant producer is closed under the witnessed exact certificate.
    ProducerClosure(crate::SemanticClosureRequirement),
    MultiplicityFeaturingContext,
    PublishedImportedMemberships,
    OperationalImportedMembershipsV1,
    MultiplicityBound,
    TypingFeatures,
    FeatureTypes,
    FeatureTarget,
    FeatureWithValue,
    PublishedOwnedCrossFeature,
    /// KERML11-1; exact pilot resolution predicates and semantic ownership order.
    OperationalOwnedCrossFeatureV1,
    OwnedCrossDomain,
    OwnedCrossSubsetting,
    CrossFeature,
    ImpliedBinding(crate::ImpliedBindingRole),
    ConnectorFeaturing,
    StoredRelationship,
    InverseAssociation,
    ElementOwner,
    OwnedMembership,
    MembershipEndpoint,
    DeclaredMemberName,
    DeclaredReferenceResolution,
    NamespaceResolution,
    /// Agentique semantic erratum, operational KerML 1.0/v2 and its successors.
    OperationalRedefinitionTargetV1,
    ParameterRedefinition,
    ResultRedefinition,
    EndRedefinition,
    LibrarySpecialization,
    ExpressionResult,
    OwnedFeature,
    Specialization,
    TransitiveSpecialization,
    ConjugatedInheritance,
    FeatureChainInheritance,
    FeatureTyping,
    Subsetting,
    Redefinition,
    InheritedFeature,
    EffectiveOwnedFeature,
    RemoveRedefinedFeature,
    FeatureOwningType,
    InheritedResult,
    /// A cyclic return population established from complete owned populations,
    /// its specialization component, and complete external boundary inputs.
    InheritedResultFixedPoint,
    OrderedNamingFeature,
    ImpliedNamingAgreement,
    /// Normative naming override supplied by the context's composed language.
    ExtensionNamingFeature(&'static str),
    PublishedRedefinitionEndConformance,
    /// AGQ-KERML10-004 / KERML11-68; operational v4 and successors.
    OperationalRedefinitionEndConformanceV1,
    PublishedFormalConstraintTarget,
    OperationalFormalConstraintTargetV1,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Explanation {
    pub rule: Rule,
    pub premises: BTreeSet<Evidence>,
}

#[derive(Clone, Debug)]
pub struct QueryResult<T> {
    // Internal producer evaluation retains canonical dependencies and bounded
    // searches; public query evaluators always retain the full explanation view.
    pub(crate) producer_evidence: bool,
    // A positive fact may cite only its original declared contribution. Keep
    // current derived-origin expansion separate so a later broad read is never
    // suppressed by that narrower proof. Includes direct current roots even
    // before a derived append; original selected support is not a current root.
    pub(crate) producer_expanded_facts: BTreeSet<FactKey>,
    // Only private producer/status evaluators defer kernel search expansion.
    // Public evidence fields are populated eagerly by ordinary query evaluators.
    pub(crate) shared_search_dependencies: SharedSearchDependencies,
    pub context: SemanticContextId,
    pub value: T,
    pub completeness: Completeness,
    pub diagnostics: BTreeSet<Diagnostic>,
    pub positive_dependencies: BTreeSet<FactKey>,
    /// Searches of the current semantic context. Historical searches supporting
    /// sealed dependency facts remain on their authenticated dependency overlay.
    pub search_dependencies: BTreeSet<SearchDependency>,
    /// Alternative proofs share conclusion keys; no feature or semantic ID is allocated.
    pub explanations: BTreeMap<Conclusion, BTreeSet<Explanation>>,
    /// Leaf evidence retains kernel declared origins and recursively expanded overlay proofs.
    /// Immutable origins are shared when answers are cloned; their content, not
    /// allocation identity, participates in answer equality and debugging output.
    /// Sealed dependency roots retain their origin and link to the dependency's
    /// proof DAG; historical facts are not flattened into current-context keys.
    pub fact_origins: BTreeMap<FactKey, Arc<Origin>>,
    /// Submitted source evidence beneath any extended collection with the same key.
    pub declared_fact_origins: BTreeMap<FactKey, Arc<DeclaredOrigin>>,
    /// Immediate canonical facts actually read by the query. Derived dependencies
    /// retain their kernel DAG instead of flattening its transitive closure into
    /// every newly produced relationship. Projections resolve to occurrences.
    pub canonical_dependencies: BTreeSet<Dependency>,
}

impl<T> QueryResult<T> {
    /// Combine evidence from queries over the exact same immutable context.
    /// This retains compact producer search sets as well as public proof data.
    /// A mismatched context is rejected without modifying either result.
    pub fn merge_evidence<U>(
        &mut self,
        other: QueryResult<U>,
    ) -> Result<(), QueryEvidenceContextMismatch> {
        if self.context != other.context {
            return Err(QueryEvidenceContextMismatch);
        }
        self.merge(other);
        Ok(())
    }
    /// Project a value without discarding context, completeness or any evidence.
    /// A projection that performs additional semantic reads must account for
    /// those reads separately; this operation only transforms the existing value.
    pub fn map<U>(self, transform: impl FnOnce(T) -> U) -> QueryResult<U> {
        QueryResult {
            value: transform(self.value),
            producer_evidence: self.producer_evidence,
            producer_expanded_facts: self.producer_expanded_facts,
            shared_search_dependencies: self.shared_search_dependencies,
            context: self.context,
            completeness: self.completeness,
            diagnostics: self.diagnostics,
            positive_dependencies: self.positive_dependencies,
            search_dependencies: self.search_dependencies,
            explanations: self.explanations,
            fact_origins: self.fact_origins,
            declared_fact_origins: self.declared_fact_origins,
            canonical_dependencies: self.canonical_dependencies,
        }
    }
    pub(crate) fn new(context: &SemanticContextId, value: T) -> Self {
        Self {
            producer_evidence: false,
            producer_expanded_facts: BTreeSet::new(),
            shared_search_dependencies: SharedSearchDependencies::default(),
            context: context.clone(),
            value,
            completeness: Completeness::Complete,
            diagnostics: BTreeSet::new(),
            positive_dependencies: BTreeSet::new(),
            search_dependencies: BTreeSet::new(),
            explanations: BTreeMap::new(),
            fact_origins: BTreeMap::new(),
            declared_fact_origins: BTreeMap::new(),
            canonical_dependencies: BTreeSet::new(),
        }
    }
    pub(crate) fn merge<U>(&mut self, other: QueryResult<U>) {
        debug_assert_eq!(self.context, other.context);
        self.completeness = self.completeness.max(other.completeness);
        self.diagnostics.extend(other.diagnostics);
        self.positive_dependencies
            .extend(other.positive_dependencies);
        self.search_dependencies.extend(other.search_dependencies);
        self.producer_expanded_facts
            .extend(other.producer_expanded_facts);
        if self.producer_evidence {
            self.shared_search_dependencies
                .merge(other.shared_search_dependencies);
        } else {
            self.search_dependencies.extend(
                other
                    .shared_search_dependencies
                    .iter()
                    .cloned()
                    .map(SearchDependency::Kernel),
            );
        }
        for (claim, proofs) in other.explanations {
            self.explanations.entry(claim).or_default().extend(proofs);
        }
        for (key, origin) in other.fact_origins {
            // A declared leaf from one answer must not hide an actual read of
            // the extended derived fact in another answer.
            if !matches!(origin.as_ref(), Origin::Declared(_)) {
                self.fact_origins.insert(key, origin);
            } else {
                self.fact_origins.entry(key).or_insert(origin);
            }
        }
        self.declared_fact_origins
            .extend(other.declared_fact_origins);
        self.canonical_dependencies
            .extend(other.canonical_dependencies);
    }
    /// Materialize deferred reads only when exposing an evidence-bearing result.
    pub(crate) fn expand_search_dependencies(&mut self) {
        self.search_dependencies.extend(
            self.shared_search_dependencies
                .iter()
                .cloned()
                .map(SearchDependency::Kernel),
        );
        self.shared_search_dependencies.clear();
    }
    pub(crate) fn clear_search_dependencies(&mut self) {
        self.search_dependencies.clear();
        self.shared_search_dependencies.clear();
        self.producer_expanded_facts.clear();
    }
    pub(crate) fn problem(
        &mut self,
        status: Completeness,
        code: &'static str,
        subject: ElementId,
        message: impl Into<String>,
    ) {
        self.completeness = self.completeness.max(status);
        self.diagnostics.insert(Diagnostic {
            code,
            subject,
            message: message.into(),
        });
    }
    pub(crate) fn prove(
        &mut self,
        query: QueryKind,
        subject: ElementId,
        value: ElementId,
        rule: Rule,
        premises: impl IntoIterator<Item = Evidence>,
    ) {
        if self.producer_evidence
            && !matches!(
                rule,
                Rule::ParameterRedefinition | Rule::ResultRedefinition | Rule::EndRedefinition
            )
        {
            return;
        }
        let mut premises: BTreeSet<_> = premises.into_iter().collect();
        if self.producer_evidence {
            // Positional redefinition proofs remain observable to the producer.
            // Their premise assembly reads the whole answer's search population;
            // preserve that exact boundary while other private proofs are omitted.
            premises.extend(
                self.shared_search_dependencies
                    .iter()
                    .cloned()
                    .map(|search| Evidence::Search(SearchDependency::Kernel(search))),
            );
        }
        self.explanations
            .entry(Conclusion {
                query,
                subject,
                value,
            })
            .or_default()
            .insert(Explanation { rule, premises });
    }
}

/// Evidence from distinct graph/profile contexts cannot justify one conclusion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueryEvidenceContextMismatch;
impl std::fmt::Display for QueryEvidenceContextMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("query evidence contexts differ")
    }
}
impl std::error::Error for QueryEvidenceContextMismatch {}

impl<T: PartialEq> PartialEq for QueryResult<T> {
    fn eq(&self, other: &Self) -> bool {
        self.producer_evidence == other.producer_evidence
            && self.context == other.context
            && self.value == other.value
            && self.completeness == other.completeness
            && self.diagnostics == other.diagnostics
            && self.positive_dependencies == other.positive_dependencies
            && self.explanations == other.explanations
            && self.fact_origins == other.fact_origins
            && self.declared_fact_origins == other.declared_fact_origins
            && self.canonical_dependencies == other.canonical_dependencies
            && if self.shared_search_dependencies.is_empty()
                && other.shared_search_dependencies.is_empty()
            {
                self.search_dependencies == other.search_dependencies
            } else {
                let expanded = |answer: &Self| {
                    answer
                        .search_dependencies
                        .iter()
                        .cloned()
                        .chain(
                            answer
                                .shared_search_dependencies
                                .iter()
                                .cloned()
                                .map(SearchDependency::Kernel),
                        )
                        .collect::<BTreeSet<_>>()
                };
                expanded(self) == expanded(other)
            }
    }
}
impl<T: Eq> Eq for QueryResult<T> {}

/// Allocation keys are only a local deduplication index. Owned Arcs keep each
/// key alive; comparison, formatting, read boundaries and digests use contents.
#[derive(Clone, Default)]
pub(crate) struct SharedSearchDependencies {
    sets: BTreeMap<usize, Arc<BTreeSet<agq_kernel::derived::StructuralSearch>>>,
}
impl SharedSearchDependencies {
    pub(crate) fn insert(
        &mut self,
        searches: &Arc<BTreeSet<agq_kernel::derived::StructuralSearch>>,
    ) {
        if !searches.is_empty() {
            self.sets
                .entry(Arc::as_ptr(searches) as usize)
                .or_insert_with(|| searches.clone());
        }
    }
    fn merge(&mut self, other: Self) {
        self.sets.extend(other.sets);
    }
    pub(crate) fn iter(&self) -> impl Iterator<Item = &agq_kernel::derived::StructuralSearch> {
        self.sets.values().flat_map(|set| set.iter())
    }
    fn clear(&mut self) {
        self.sets.clear();
    }
    fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }
}
impl std::fmt::Debug for SharedSearchDependencies {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Canonical content order, without local allocation keys or multiplicity.
        formatter
            .debug_set()
            .entries(self.iter().collect::<BTreeSet<_>>())
            .finish()
    }
}
pub(crate) fn claim(query: QueryKind, subject: ElementId, value: ElementId) -> Evidence {
    Evidence::Conclusion(Conclusion {
        query,
        subject,
        value,
    })
}
