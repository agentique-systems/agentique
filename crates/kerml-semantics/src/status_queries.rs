//! Explicit outcome-only queries for bulk publication checks. Evidence-bearing
//! queries remain available through `KerMlQueries` over the same immutable graph.
use crate::read_dependencies::query_read_keys;
use crate::*;
use agq_kernel::{ElementId, MetaclassId, PropertyId};
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
        let mut answer = self
            .queries
            .lookup_relationship_target(relationship, property, name);
        let keys = query_read_keys(&answer, self.queries.model());
        answer.expand_search_dependencies();
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
