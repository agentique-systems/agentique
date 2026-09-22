//! Lossless range CST. See ADR 0006 for grammar, recovery and identity boundaries.
#![forbid(unsafe_code)]
mod lexer;
mod parser;
pub mod production;
mod reconcile;

pub use agq_kernel::{DocumentId, SourceRevisionId, SyntaxNodeId, provenance::ByteRange};
pub use reconcile::TextEdit;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKind {
    Whitespace,
    Note,
    Word,
    QuotedName,
    StringValue,
    /// KerML 1.0 8.2.2.4: unsigned decimal digits, separate from a real literal's dot.
    DecimalValue,
    /// Decimal mantissa followed by an exponent; signs belong only to the exponent.
    ExponentialValue,
    Symbol,
    Comment,
    Invalid,
}
impl TokenKind {
    pub fn is_trivia(self) -> bool {
        matches!(self, Self::Whitespace | Self::Note)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub range: ByteRange,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxDiagnostic {
    pub code: &'static str,
    pub range: ByteRange,
    pub message: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyntaxStatus {
    Success,
    Recovered,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclarationKind {
    Namespace,
    Type,
    Feature,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReferenceKind {
    Specialization,
    Typing,
    Subsetting,
    Redefinition,
    NamespaceImport,
    /// An import naming a specific Membership rather than a Namespace.
    MembershipImport,
    Alias,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Visibility {
    #[default]
    Public,
    Protected,
    Private,
}
impl Visibility {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Protected => "protected",
            Self::Private => "private",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Name {
    pub value: String,
    pub range: ByteRange,
}
#[derive(Clone, Debug)]
pub struct ReferenceSyntax {
    pub id: SyntaxNodeId,
    pub kind: ReferenceKind,
    pub range: ByteRange,
    pub absolute: bool,
    pub segments: Vec<Name>,
}
#[derive(Clone, Debug)]
pub struct DeclarationSyntax {
    pub id: SyntaxNodeId,
    pub kind: DeclarationKind,
    pub range: ByteRange,
    pub keyword: ByteRange,
    pub header: ByteRange,
    pub name: Option<Name>,
    pub is_abstract: bool,
    pub visibility: Visibility,
    /// A malformed/unsupported header must never assert declaration semantics.
    pub header_valid: bool,
    /// False if a body terminator was recovered. A valid header can still lower.
    pub complete: bool,
    pub references: Vec<ReferenceSyntax>,
    pub children: Vec<SyntaxNode>,
}
/// Bounded namespace syntax: named aliases and nonrecursive namespace imports.
/// Unsupported recursive/all/filter imports remain recovery regions.
#[derive(Clone, Debug)]
pub struct NamespaceReferenceSyntax {
    pub id: SyntaxNodeId,
    pub range: ByteRange,
    pub keyword: ByteRange,
    pub visibility: Visibility,
    pub alias: Option<Name>,
    pub reference: ReferenceSyntax,
}
#[derive(Clone, Debug)]
pub enum SyntaxNode {
    Declaration(DeclarationSyntax),
    NamespaceReference(NamespaceReferenceSyntax),
    Error { id: SyntaxNodeId, range: ByteRange },
}
/// Borrowed syntax AST view; no semantic identities or resolution results.
#[derive(Clone, Copy)]
pub struct Declaration<'a>(&'a DeclarationSyntax);
impl<'a> Declaration<'a> {
    pub fn cast(node: &'a SyntaxNode) -> Option<Self> {
        match node {
            SyntaxNode::Declaration(d) => Some(Self(d)),
            _ => None,
        }
    }
    pub fn syntax(self) -> &'a DeclarationSyntax {
        self.0
    }
    pub fn children(self) -> impl Iterator<Item = Declaration<'a>> {
        self.0.children.iter().filter_map(Self::cast)
    }
}
#[derive(Clone, Debug)]
pub struct SyntaxDocument {
    document: DocumentId,
    revision: SourceRevisionId,
    source: Arc<str>,
    pub(crate) root_id: SyntaxNodeId,
    pub(crate) tokens: Vec<Token>,
    pub(crate) nodes: Vec<SyntaxNode>,
    pub(crate) diagnostics: Vec<SyntaxDiagnostic>,
}
impl SyntaxDocument {
    pub fn document(&self) -> DocumentId {
        self.document
    }
    pub fn revision(&self) -> SourceRevisionId {
        self.revision
    }
    pub fn root_id(&self) -> SyntaxNodeId {
        self.root_id
    }
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }
    pub fn nodes(&self) -> &[SyntaxNode] {
        &self.nodes
    }
    pub fn declarations(&self) -> impl Iterator<Item = Declaration<'_>> {
        self.nodes.iter().filter_map(Declaration::cast)
    }
    pub fn diagnostics(&self) -> &[SyntaxDiagnostic] {
        &self.diagnostics
    }
    pub fn status(&self) -> SyntaxStatus {
        if self.diagnostics.is_empty() {
            SyntaxStatus::Success
        } else {
            SyntaxStatus::Recovered
        }
    }
    pub fn text(&self, range: ByteRange) -> Option<&str> {
        self.source
            .get(range.start() as usize..range.end() as usize)
    }
    pub fn token_text(&self, token: &Token) -> &str {
        self.text(token.range).expect("lexer ranges")
    }
    pub fn origin(
        &self,
        node: SyntaxNodeId,
        range: ByteRange,
    ) -> agq_kernel::provenance::SourceOrigin {
        agq_kernel::provenance::SourceOrigin {
            document: self.document,
            revision: self.revision,
            range,
            syntax_node: Some(node),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct ParseLimits {
    pub max_bytes: usize,
    pub max_tokens: usize,
    pub max_depth: usize,
}
impl Default for ParseLimits {
    fn default() -> Self {
        Self {
            max_bytes: 8 * 1024 * 1024,
            max_tokens: 200_000,
            max_depth: 128,
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("source exceeds {0} limit")]
    Limit(&'static str),
    #[error("edit range is outside the source or splits a UTF-8 character")]
    InvalidEdit,
}
pub fn parse(
    document: DocumentId,
    source: impl Into<Arc<str>>,
    limits: ParseLimits,
) -> Result<SyntaxDocument, SourceError> {
    let source = source.into();
    if source.len() > limits.max_bytes {
        return Err(SourceError::Limit("byte"));
    }
    let (tokens, diagnostics) = lexer::lex(&source, limits.max_tokens)?;
    let mut doc = SyntaxDocument {
        document,
        revision: SourceRevisionId::new(),
        source,
        root_id: SyntaxNodeId::new(),
        tokens,
        nodes: vec![],
        diagnostics,
    };
    parser::parse(&mut doc, limits.max_depth.min(128));
    Ok(doc)
}
pub(crate) fn range(start: usize, end: usize) -> ByteRange {
    ByteRange::new(start as u64, end as u64).expect("ordered parser range")
}
