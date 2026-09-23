//! Evidence is separate from semantic identity and from modeled properties.
use crate::{
    DocumentId, ElementId, GeneratorId, LibraryId, PropertyId, RuleId, SourceRevisionId,
    SyntaxNodeId, TransformationId,
};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;

/// Validated half-open byte range in a separately managed source document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ByteRange {
    start: u64,
    end: u64,
}
impl ByteRange {
    /// Construct a half-open range, rejecting reversed endpoints.
    pub fn new(start: u64, end: u64) -> Result<Self, InvalidByteRange> {
        if start > end {
            Err(InvalidByteRange { start, end })
        } else {
            Ok(Self { start, end })
        }
    }
    /// Inclusive byte offset.
    pub fn start(self) -> u64 {
        self.start
    }
    /// Exclusive byte offset.
    pub fn end(self) -> u64 {
        self.end
    }
}
/// Source range endpoints were supplied in reverse order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("source byte range {start}..{end} is reversed")]
pub struct InvalidByteRange {
    pub start: u64,
    pub end: u64,
}

/// Source evidence without an AST dependency. Document bounds are checked upstream.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SourceOrigin {
    /// Document identity allocated by the source store.
    pub document: DocumentId,
    /// Exact immutable source revision to which the byte range belongs.
    pub revision: SourceRevisionId,
    /// Location within that source document.
    pub range: ByteRange,
    /// Optional identity maintained by the syntax layer.
    pub syntax_node: Option<SyntaxNodeId>,
}

/// Provenance of an explicitly submitted element or slot (including imports).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DeclaredOrigin {
    /// Explicitly authored, optionally backed by source text.
    Authored { source: Option<SourceOrigin> },
    /// Submitted from a separately pinned standard-library artifact.
    StandardLibrary { library: LibraryId },
    /// A reviewed, versioned correction to a pinned external model. This is
    /// neither a claim about the original source nor ordinary semantic inference.
    /// Authority interpretation belongs to the language or importing layer.
    ReviewedCorrection {
        /// Exact correction profile, including its version.
        profile: String,
        /// Reviewed entry within that profile.
        entry: String,
        /// Stable evidence URIs or content-qualified evidence references.
        authority: BTreeSet<String>,
        /// Content-qualified identity of the originating library.
        library: LibraryId,
        /// Deterministic locator for the reviewed input assertion.
        source_key: String,
        /// Deterministic semantic operation path and output role.
        output_key: String,
    },
    /// Inputs are historical provenance, not current semantic references.
    Transformation {
        transformation: TransformationId,
        inputs: BTreeSet<ElementId>,
    },
    /// Submitted by tooling, without claiming semantic inference.
    Generated { generator: GeneratorId },
}

/// Identifies an element assertion or a property assertion for explanations.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum FactKey {
    AssociationOccurrence(crate::AssociationOccurrenceId),
    Element(ElementId),
    Property {
        element: ElementId,
        property: PropertyId,
    },
}

/// Positive evidence in one pinned declared revision / derivation overlay.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub enum Dependency {
    Declared(FactKey),
    Derived(FactKey),
}

/// One derivation's immediate evidence; follow `Derived` dependencies to explain it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Explanation {
    /// Producer's stable versioned rule identity; rule execution is outside the kernel.
    pub rule: RuleId,
    /// Immediate positive evidence. The producer is responsible for completeness;
    /// the kernel checks existence and acyclicity, not the truth of the inference.
    pub dependencies: BTreeSet<Dependency>,
}

/// Shares equal immutable evidence while producers plan and enqueue facts.
/// Content equality alone determines sharing; allocation and hash iteration
/// order never participate in semantic identity or validation.
#[derive(Clone, Debug, Default)]
pub struct ExplanationPool {
    base: Option<Arc<ExplanationPool>>,
    entries: Arc<HashSet<Arc<Explanation>>>,
    // Every address is kept alive by `entries`. This only avoids re-hashing an
    // already interned immutable allocation; equality still decides sharing for
    // distinct allocations. Addresses never become semantic identities.
    allocations: Arc<HashSet<usize>>,
    derived_dependencies: Arc<HashMap<usize, Vec<FactKey>>>,
    statistics: ExplanationPoolStatistics,
}

/// Deterministic work counters local to one explanation pool.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExplanationPoolStatistics {
    pub interned: usize,
    pub reused: usize,
}

impl ExplanationPool {
    pub(crate) fn fork(&self) -> Self {
        Self {
            base: Some(Arc::new(self.clone())),
            statistics: self.statistics,
            ..Default::default()
        }
    }
    fn contains_allocation(&self, allocation: usize) -> bool {
        self.allocations.contains(&allocation)
            || self
                .base
                .as_ref()
                .is_some_and(|base| base.contains_allocation(allocation))
    }
    fn find(&self, explanation: &Arc<Explanation>) -> Option<&Arc<Explanation>> {
        self.entries
            .get(explanation)
            .or_else(|| self.base.as_ref()?.find(explanation))
    }

    /// Counts requests, including equal proofs supplied in separate allocations.
    pub fn statistics(&self) -> ExplanationPoolStatistics {
        self.statistics
    }

    /// Retain one allocation for this exact rule and dependency set.
    pub fn intern(&mut self, explanation: Explanation) -> Arc<Explanation> {
        self.intern_shared(Arc::new(explanation))
    }
    /// Reuse an already shared proof without copying its dependency set.
    pub fn intern_shared(&mut self, explanation: Arc<Explanation>) -> Arc<Explanation> {
        let allocation = Arc::as_ptr(&explanation) as usize;
        if self.contains_allocation(allocation) {
            self.statistics.reused += 1;
            return explanation;
        }
        if let Some(existing) = self.find(&explanation).cloned() {
            self.statistics.reused += 1;
            existing
        } else {
            Arc::make_mut(&mut self.entries).insert(explanation.clone());
            Arc::make_mut(&mut self.allocations).insert(allocation);
            Arc::make_mut(&mut self.derived_dependencies).insert(
                allocation,
                explanation
                    .dependencies
                    .iter()
                    .filter_map(|dependency| match dependency {
                        Dependency::Derived(fact) => Some(*fact),
                        Dependency::Declared(_) => None,
                    })
                    .collect(),
            );
            self.statistics.interned += 1;
            explanation
        }
    }

    /// Borrow the unique proof's positive derived adjacency without rescanning
    /// declared evidence for every output that shares this proof.
    pub(crate) fn derived_dependencies(&self, explanation: &Arc<Explanation>) -> &[FactKey] {
        self.derived_dependencies
            .get(&(Arc::as_ptr(explanation) as usize))
            .map(Vec::as_slice)
            .or_else(|| {
                self.base
                    .as_ref()
                    .map(|base| base.derived_dependencies(explanation))
            })
            .expect("interned proof dependencies")
    }
}

/// Explicit separation between submitted facts and semantic inference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Origin {
    /// A read-only navigation projection; inspect every contributing canonical link.
    AssociationOccurrences(BTreeSet<crate::AssociationOccurrenceId>),
    Declared(DeclaredOrigin),
    /// Immutable evidence is shared by records, indexes and explanation lookup.
    Derived(Arc<Explanation>),
}

impl serde::Serialize for ByteRange {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(&(self.start, self.end), serializer)
    }
}
impl<'de> serde::Deserialize<'de> for ByteRange {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (start, end) = <(u64, u64) as serde::Deserialize>::deserialize(deserializer)?;
        Self::new(start, end).map_err(serde::de::Error::custom)
    }
}

#[cfg(any(test, feature = "verification"))]
impl ExplanationPool {
    pub(crate) fn storage_tables(
        &self,
        out: &mut std::collections::BTreeSet<crate::storage_observer::StorageTableIdentity>,
    ) {
        use crate::storage_observer::table;
        table(out, "proof_intern", Arc::as_ptr(&self.entries) as usize);
        table(
            out,
            "proof_allocations",
            Arc::as_ptr(&self.allocations) as usize,
        );
        table(
            out,
            "proof_adjacency",
            Arc::as_ptr(&self.derived_dependencies) as usize,
        );
        if let Some(base) = &self.base {
            base.storage_tables(out);
        }
    }
    pub(crate) fn storage_observation(&self, out: &mut crate::storage_observer::DependencyStorage) {
        if let Some(base) = &self.base {
            base.storage_tables(&mut out.base_tables);
            out.copied_dependency_entries.add(
                "proof_intern",
                self.entries
                    .iter()
                    .filter(|proof| base.find(proof).is_some())
                    .count(),
            );
            out.copied_dependency_entries.add(
                "proof_allocations",
                self.allocations
                    .iter()
                    .filter(|id| base.contains_allocation(**id))
                    .count(),
            );
            out.copied_dependency_entries.add(
                "proof_adjacency",
                self.derived_dependencies
                    .keys()
                    .filter(|id| base.contains_allocation(**id))
                    .count(),
            );
        }
    }
}
