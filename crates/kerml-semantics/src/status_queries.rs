//! Explicit outcome-only queries for bulk publication checks. Evidence-bearing
//! queries remain available through `KerMlQueries` over the same immutable graph.
use crate::producer_worklist::{InvalidationKey, query_read_keys};
use crate::*;
use agq_kernel::{ElementId, MetaclassId, PropertyId, provenance::Dependency};
use std::collections::BTreeSet;

/// A query outcome with no explanation payload. Complete still means complete
/// for this bounded query, not publication or conformance completion. Obtain a
/// full `QueryResult` from `KerMlQueries` when inspecting evidence or dependencies.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryOutcome<T> {
    pub context: SemanticContextId,
    pub value: T,
    pub completeness: Completeness,
    pub diagnostics: BTreeSet<Diagnostic>,
}
impl<T> From<QueryResult<T>> for QueryOutcome<T> {
    fn from(answer: QueryResult<T>) -> Self {
        Self {
            context: answer.context,
            value: answer.value,
            completeness: answer.completeness,
            diagnostics: answer.diagnostics,
        }
    }
}
/// Bounded positive and negative reads for selective outcome invalidation.
/// This is a read set, not an expanded explanation or a publication certificate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryReadSet {
    canonical_dependencies: BTreeSet<Dependency>,
    search_dependencies: BTreeSet<SearchDependency>,
    keys: BTreeSet<InvalidationKey>,
}
impl QueryReadSet {
    pub fn canonical_dependencies(&self) -> &BTreeSet<Dependency> {
        &self.canonical_dependencies
    }
    pub fn search_dependencies(&self) -> &BTreeSet<SearchDependency> {
        &self.search_dependencies
    }
    /// `affected_elements` includes changed records/owners and BOTH old and new
    /// reference and occurrence endpoints, including inverse navigation. Changes
    /// to pending scopes or construction obligations also contribute their subject
    /// IDs. A changed static context contract invalidates every outcome.
    pub fn affected_by(
        &self,
        affected_elements: &BTreeSet<ElementId>,
        context_contract_changed: bool,
    ) -> bool {
        context_contract_changed
            || (!affected_elements.is_empty()
                && (self.keys.contains(&InvalidationKey::Global)
                    || affected_elements.iter().any(|id| {
                        self.keys.contains(&InvalidationKey::Element(*id))
                            || self.keys.contains(&InvalidationKey::Incoming(*id))
                    })))
    }
    /// Compare immutable query-contract inputs across a reconstructed candidate.
    /// Revision/content digests and pending-state populations may change. Callers
    /// account for those graph and pending-state changes through `affected_by`.
    /// The exact formal target/profile/library contract must remain unchanged;
    /// its per-frontier digest and evidence are allowed to rebind.
    pub fn context_compatible(previous: &SemanticContextId, current: &SemanticContextId) -> bool {
        let mut current = current.clone();
        current.revision = previous.revision;
        current.model_digest = previous.model_digest;
        current.library_graph_digest = previous.library_graph_digest;
        current.derivation_phase = previous.derivation_phase;
        current.pending_specialization_scopes = previous.pending_specialization_scopes.clone();
        current.pending_namespace_scopes = previous.pending_namespace_scopes.clone();
        current.construction_obligations = previous.construction_obligations.clone();
        if let (Some(current_targets), Some(previous_targets)) = (
            &current.formal_constraint_targets,
            &previous.formal_constraint_targets,
        ) && current_targets.same_binding_contract(previous_targets)
        {
            current.formal_constraint_targets = previous.formal_constraint_targets.clone();
        }
        &current == previous
    }
}
/// Explicit outcome/read-set pair for revision-aware bulk query consumers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryOutcomeWithReads<T> {
    pub outcome: QueryOutcome<T>,
    pub reads: QueryReadSet,
}
/// An immutable evaluator for bulk decisions that need values and diagnostics.
/// Its private evaluator retains canonical dependencies while evaluating rules,
/// and does not expand the explanation forest that these operations never return.
pub struct KerMlStatusQueries<'m> {
    queries: KerMlQueries<'m>,
}
impl<'m> KerMlStatusQueries<'m> {
    pub fn new(context: SemanticContext<'m>) -> Self {
        Self {
            queries: KerMlQueries::for_production(context),
        }
    }
    pub fn context(&self) -> &SemanticContextId {
        self.queries.context()
    }
    /// Release traversal caches at a caller-chosen batch boundary.
    pub fn fork(&self) -> Self {
        Self::new(self.queries.context.fork())
    }
    pub fn lookup_relationship_target(
        &self,
        relationship: ElementId,
        property: PropertyId,
        name: &QualifiedName,
    ) -> QueryOutcome<Vec<MemberMatch>> {
        self.queries
            .lookup_relationship_target(relationship, property, name)
            .into()
    }
    pub fn lookup_relationship_target_with_reads(
        &self,
        relationship: ElementId,
        property: PropertyId,
        name: &QualifiedName,
    ) -> QueryOutcomeWithReads<Vec<MemberMatch>> {
        let answer = self
            .queries
            .lookup_relationship_target(relationship, property, name);
        let keys = query_read_keys(&answer, self.queries.model());
        QueryOutcomeWithReads {
            outcome: QueryOutcome {
                context: answer.context,
                value: answer.value,
                completeness: answer.completeness,
                diagnostics: answer.diagnostics,
            },
            reads: QueryReadSet {
                canonical_dependencies: answer.canonical_dependencies,
                search_dependencies: answer.search_dependencies,
                keys,
            },
        }
    }
    pub fn resolve_name(
        &self,
        scope: ElementId,
        name: &QualifiedName,
        expected: MetaclassId,
        membership_target: bool,
    ) -> QueryOutcome<Resolution> {
        self.queries
            .resolve_name(scope, name, expected, membership_target)
            .into()
    }
    pub fn resolve_reference(
        &self,
        specific: ElementId,
        name: &QualifiedName,
        expected: MetaclassId,
    ) -> QueryOutcome<Resolution> {
        self.queries
            .resolve_reference(specific, name, expected)
            .into()
    }
}
impl<'m> KerMlQueries<'m> {
    pub fn status_queries(&self) -> KerMlStatusQueries<'m> {
        KerMlStatusQueries::new(self.context.fork())
    }
}
