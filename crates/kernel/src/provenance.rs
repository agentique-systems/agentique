//! Evidence is separate from semantic identity and from modeled properties.
use crate::{
    DocumentId, ElementId, GeneratorId, LibraryId, PropertyId, RuleId, SourceRevisionId,
    SyntaxNodeId, TransformationId,
};
use std::collections::BTreeSet;

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
#[derive(Clone, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeclaredOrigin {
    /// Explicitly authored, optionally backed by source text.
    Authored { source: Option<SourceOrigin> },
    /// Submitted from a separately pinned standard-library artifact.
    StandardLibrary { library: LibraryId },
    /// Inputs are historical provenance, not current semantic references.
    Transformation {
        transformation: TransformationId,
        inputs: BTreeSet<ElementId>,
    },
    /// Submitted by tooling, without claiming semantic inference.
    Generated { generator: GeneratorId },
}

/// Identifies an element assertion or a property assertion for explanations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FactKey {
    Element(ElementId),
    Property {
        element: ElementId,
        property: PropertyId,
    },
}

/// Positive evidence in one pinned declared revision / derivation overlay.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Dependency {
    Declared(FactKey),
    Derived(FactKey),
}

/// One derivation's immediate evidence; follow `Derived` dependencies to explain it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Explanation {
    /// Producer's stable versioned rule identity; rule execution is outside the kernel.
    pub rule: RuleId,
    /// Immediate positive evidence. The producer is responsible for completeness;
    /// the kernel checks existence and acyclicity, not the truth of the inference.
    pub dependencies: BTreeSet<Dependency>,
}

/// Explicit separation between submitted facts and semantic inference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Origin {
    Declared(DeclaredOrigin),
    Derived(Explanation),
}
