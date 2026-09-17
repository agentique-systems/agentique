use crate::SemanticContextId;
use agq_kernel::{
    ElementId, MetaclassId, PropertyId,
    provenance::{FactKey, Origin},
};
use std::collections::{BTreeMap, BTreeSet};

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
    Instances {
        class: MetaclassId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diagnostic {
    pub code: &'static str,
    pub subject: ElementId,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum QueryKind {
    OwnedRelationships,
    OwningRelationship,
    OwningRelatedElement,
    Owner,
    Memberships,
    Member,
    LookupDeclaredMember,
    DirectFeatures,
    DirectSpecializations,
    AllSpecializations,
    DirectFeatureTypes,
    SubsettedFeatures,
    RedefinedFeatures,
    AllRedefinedFeatures,
    InheritedFeature,
    EffectiveFeatures,
    SuppressedFeature,
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
    StoredRelationship,
    InverseAssociation,
    ElementOwner,
    OwnedMembership,
    MembershipEndpoint,
    DeclaredMemberName,
    OwnedFeature,
    Specialization,
    TransitiveSpecialization,
    FeatureTyping,
    Subsetting,
    Redefinition,
    InheritedFeature,
    EffectiveOwnedFeature,
    RemoveRedefinedFeature,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Explanation {
    pub rule: Rule,
    pub premises: BTreeSet<Evidence>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryResult<T> {
    pub context: SemanticContextId,
    pub value: T,
    pub completeness: Completeness,
    pub diagnostics: BTreeSet<Diagnostic>,
    pub positive_dependencies: BTreeSet<FactKey>,
    pub search_dependencies: BTreeSet<SearchDependency>,
    /// Alternative proofs share conclusion keys; no feature or semantic ID is allocated.
    pub explanations: BTreeMap<Conclusion, BTreeSet<Explanation>>,
    /// Leaf evidence retains kernel declared origins and recursively expanded overlay proofs.
    pub fact_origins: BTreeMap<FactKey, Origin>,
}

impl<T> QueryResult<T> {
    pub(crate) fn new(context: &SemanticContextId, value: T) -> Self {
        Self {
            context: context.clone(),
            value,
            completeness: Completeness::Complete,
            diagnostics: BTreeSet::new(),
            positive_dependencies: BTreeSet::new(),
            search_dependencies: BTreeSet::new(),
            explanations: BTreeMap::new(),
            fact_origins: BTreeMap::new(),
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
        self.fact_origins.extend(other.fact_origins);
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
