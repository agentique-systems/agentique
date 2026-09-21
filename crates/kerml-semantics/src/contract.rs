use crate::SemanticContextId;
use agq_kernel::{
    ElementId, MetaclassId, PropertyId,
    provenance::{DeclaredOrigin, Dependency, FactKey, Origin},
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

/// Complete means complete for the named bounded query, not language validation success.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Completeness {
    Complete,
    Incomplete,
    Invalid,
}

/// Set reads remain dependencies even when empty. Match pre- AND post-change state.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SearchDependency {
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
    Element(ElementId),
    PropertySet {
        element: ElementId,
        property: PropertyId,
    },
    /// Every reference occurrence to this target (including source retargeting).
    Incoming {
        target: ElementId,
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

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diagnostic {
    pub code: &'static str,
    pub subject: ElementId,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum QueryKind {
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
    OrderedNamingFeature,
    ImpliedNamingAgreement,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryResult<T> {
    // Internal producer evaluation retains canonical dependencies and bounded
    // searches; public query evaluators always retain the full explanation view.
    pub(crate) producer_evidence: bool,
    pub context: SemanticContextId,
    pub value: T,
    pub completeness: Completeness,
    pub diagnostics: BTreeSet<Diagnostic>,
    pub positive_dependencies: BTreeSet<FactKey>,
    pub search_dependencies: BTreeSet<SearchDependency>,
    /// Alternative proofs share conclusion keys; no feature or semantic ID is allocated.
    pub explanations: BTreeMap<Conclusion, BTreeSet<Explanation>>,
    /// Leaf evidence retains kernel declared origins and recursively expanded overlay proofs.
    /// Immutable origins are shared when answers are cloned; their content, not
    /// allocation identity, participates in answer equality and debugging output.
    pub fact_origins: BTreeMap<FactKey, Arc<Origin>>,
    /// Submitted source evidence beneath any extended collection with the same key.
    pub declared_fact_origins: BTreeMap<FactKey, Arc<DeclaredOrigin>>,
    /// Immediate canonical facts actually read by the query. Derived dependencies
    /// retain their kernel DAG instead of flattening its transitive closure into
    /// every newly produced relationship. Projections resolve to occurrences.
    pub canonical_dependencies: BTreeSet<Dependency>,
}

impl<T> QueryResult<T> {
    pub(crate) fn new(context: &SemanticContextId, value: T) -> Self {
        Self {
            producer_evidence: false,
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
        self.explanations
            .entry(Conclusion {
                query,
                subject,
                value,
            })
            .or_default()
            .insert(Explanation {
                rule,
                premises: premises.into_iter().collect(),
            });
    }
}
pub(crate) fn claim(query: QueryKind, subject: ElementId, value: ElementId) -> Evidence {
    Evidence::Conclusion(Conclusion {
        query,
        subject,
        value,
    })
}
