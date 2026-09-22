//! Lossless grammar-production trees (ADR 0013).
//!
//! Grammar recognition has no semantic resolution, metaclass or library dependency.
//! The source/token partition is authoritative for round trips, including recovery.
mod chart;
mod generated;
mod generated_sysml;
mod generated_sysml_operational;
pub use generated::Production;

use crate::{
    ByteRange, DocumentId, ParseLimits, SourceError, SourceRevisionId, SyntaxDiagnostic,
    SyntaxNodeId, Token, TokenKind,
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug)]
enum Symbol {
    Nonterminal(u16),
    Text(&'static str),
    Name,
    String,
    Comment,
    Decimal,
    Exponential,
}
struct Rule {
    lhs: u16,
    kind: Option<Production>,
    rhs: &'static [Symbol],
}

/// The pinned textual dialect used by the shared lexer and production arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialect {
    /// KerML 1.0, including its separately recorded grammar discrepancies.
    KerMl,
    /// SysML 2.0; the document separately retains its selected syntax profile.
    SysMl,
}

/// Explicit textual authority selection, independent of semantic acceptance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SysmlSyntaxProfile {
    /// Exact published final grammar; the historical eight corpus failures remain.
    #[default]
    Published,
    /// Published grammar plus the four reviewed Systems Library compatibility decisions.
    OperationalV1,
    /// Operational v1 grammar plus the v2 semantic correction manifest.
    OperationalV2,
}

impl SysmlSyntaxProfile {
    /// Versioned authority identity shared with the SysML semantic profile.
    pub fn id(self) -> &'static str {
        match self {
            Self::Published => "omg-sysml-2.0-published/1",
            Self::OperationalV1 => "agentique-sysml-2.0-operational/1",
            Self::OperationalV2 => "agentique-sysml-2.0-operational/2",
        }
    }

    /// SHA-256 of the operational compatibility manifest's UTF-8 bytes with LF.
    /// Published selection applies no compatibility manifest.
    pub fn grammar_compatibility_manifest_sha256(self) -> Option<&'static str> {
        match self {
            Self::Published => None,
            Self::OperationalV1 => Some(generated_sysml_operational::COMPATIBILITY_MANIFEST_SHA256),
            Self::OperationalV2 => Some(generated_sysml_operational::COMPATIBILITY_MANIFEST_SHA256),
        }
    }
}

struct Grammar {
    root: u16,
    symbol_count: usize,
    rules: &'static [Rule],
    keywords: &'static [&'static str],
}

impl Dialect {
    fn grammar(self, profile: Option<SysmlSyntaxProfile>) -> Grammar {
        match (self, profile) {
            (Self::KerMl, _) => Grammar {
                root: generated::ROOT,
                symbol_count: generated::SYMBOL_COUNT,
                rules: generated::RULES,
                keywords: generated::KEYWORDS,
            },
            (
                Self::SysMl,
                Some(SysmlSyntaxProfile::OperationalV1 | SysmlSyntaxProfile::OperationalV2),
            ) => Grammar {
                root: generated_sysml_operational::ROOT,
                symbol_count: generated_sysml_operational::SYMBOL_COUNT,
                rules: generated_sysml_operational::RULES,
                keywords: generated_sysml_operational::KEYWORDS,
            },
            (Self::SysMl, _) => Grammar {
                root: generated_sysml::ROOT,
                symbol_count: generated_sysml::SYMBOL_COUNT,
                rules: generated_sysml::RULES,
                keywords: generated_sysml::KEYWORDS,
            },
        }
    }
}

/// Work limits are independent of semantic identity and source revision policy.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub source: ParseLimits,
    /// Maximum chart items, including unsuccessful alternatives, in one parse.
    pub max_chart_items: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            source: ParseLimits::default(),
            max_chart_items: 4_000_000,
        }
    }
}

/// An arena locator, never a semantic identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeIndex(pub(crate) usize);

#[derive(Clone, Debug)]
struct NodeData {
    kind: Production,
    id: SyntaxNodeId,
    range: ByteRange,
    children: Vec<NodeIndex>,
}

/// Published grammar discrepancies are separate from missing parser support.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrammarDiscrepancy {
    pub code: &'static str,
    pub range: ByteRange,
    pub production: Production,
    pub authority: &'static str,
}

/// Immutable syntax only. All semantic records belong to a kernel Snapshot.
#[derive(Clone, Debug)]
pub struct Document {
    dialect: Dialect,
    sysml_profile: Option<SysmlSyntaxProfile>,
    document: DocumentId,
    revision: SourceRevisionId,
    source: Arc<str>,
    tokens: Vec<Token>,
    nodes: Vec<NodeData>,
    roots: Vec<NodeIndex>,
    recovery: Vec<ByteRange>,
    diagnostics: Vec<SyntaxDiagnostic>,
    discrepancies: Vec<GrammarDiscrepancy>,
}
impl Document {
    /// Reparse one edit against this exact source revision. Unchanged nodes outside
    /// the edited range retain identity; overlapping or ambiguous nodes are fresh.
    /// This conservative syntax policy does not reconcile semantic library IDs.
    pub fn edit(&self, edit: &crate::TextEdit, limits: Limits) -> Result<Self, SourceError> {
        if self.text(edit.range).is_none() {
            return Err(SourceError::InvalidEdit);
        }
        let remaining = self.source.len() - (edit.range.end() - edit.range.start()) as usize;
        if remaining > limits.source.max_bytes
            || edit.replacement.len() > limits.source.max_bytes.saturating_sub(remaining)
        {
            return Err(SourceError::Limit("byte"));
        }
        let mut source = self.source.to_string();
        source.replace_range(
            edit.range.start() as usize..edit.range.end() as usize,
            &edit.replacement,
        );
        let mut next = parse_with_profile(
            self.dialect,
            self.sysml_profile,
            self.document,
            SourceRevisionId::new(),
            source,
            limits,
        )?;
        let delta = edit.replacement.len() as i64 - (edit.range.end() - edit.range.start()) as i64;
        let mut candidates = std::collections::BTreeMap::<_, Vec<_>>::new();
        for node in &self.nodes {
            let span = if node.range.end() <= edit.range.start() {
                Some(node.range)
            } else if node.range.start() >= edit.range.end() {
                ByteRange::new(
                    (node.range.start() as i64 + delta) as u64,
                    (node.range.end() as i64 + delta) as u64,
                )
                .ok()
            } else {
                None
            };
            if let Some(span) = span {
                candidates
                    .entry((node.kind, span.start(), span.end()))
                    .or_default()
                    .push(node.id);
            }
        }
        for node in &mut next.nodes {
            if let Some(ids) = candidates.get(&(node.kind, node.range.start(), node.range.end()))
                && let [id] = ids.as_slice()
            {
                node.id = *id;
            }
        }
        Ok(next)
    }
    pub fn document(&self) -> DocumentId {
        self.document
    }
    /// The grammar and reserved-name policy retained by subsequent edits.
    pub fn dialect(&self) -> Dialect {
        self.dialect
    }
    /// The selected SysML grammar authority; absent for KerML documents.
    pub fn sysml_profile(&self) -> Option<SysmlSyntaxProfile> {
        self.sysml_profile
    }
    pub fn revision(&self) -> SourceRevisionId {
        self.revision
    }
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }
    pub fn diagnostics(&self) -> &[SyntaxDiagnostic] {
        &self.diagnostics
    }
    pub fn discrepancies(&self) -> &[GrammarDiscrepancy] {
        &self.discrepancies
    }
    pub fn recovery(&self) -> &[ByteRange] {
        &self.recovery
    }
    pub fn is_complete(&self) -> bool {
        self.diagnostics.is_empty() && self.recovery.is_empty()
    }
    pub fn roots(&self) -> impl Iterator<Item = Node<'_>> {
        self.roots.iter().map(|&index| self.node(index))
    }
    pub fn nodes(&self) -> impl Iterator<Item = Node<'_>> {
        (0..self.nodes.len()).map(|i| self.node(NodeIndex(i)))
    }
    pub fn node(&self, index: NodeIndex) -> Node<'_> {
        Node {
            document: self,
            index,
        }
    }
    pub fn text(&self, range: ByteRange) -> Option<&str> {
        self.source
            .get(range.start() as usize..range.end() as usize)
    }
    /// No allocation or normalization: concatenating these slices exactly restores source.
    pub fn token_text(&self, token: &Token) -> &str {
        self.text(token.range).expect("lexer range")
    }
}

/// Uniform borrowed typed-node strategy; `kind` names the actual grammar production.
#[derive(Clone, Copy)]
pub struct Node<'a> {
    document: &'a Document,
    index: NodeIndex,
}
impl<'a> Node<'a> {
    pub fn index(self) -> NodeIndex {
        self.index
    }
    pub fn kind(self) -> Production {
        self.document.nodes[self.index.0].kind
    }
    pub fn id(self) -> SyntaxNodeId {
        self.document.nodes[self.index.0].id
    }
    pub fn range(self) -> ByteRange {
        self.document.nodes[self.index.0].range
    }
    pub fn text(self) -> &'a str {
        self.document.text(self.range()).expect("syntax range")
    }
    pub fn children(self) -> impl Iterator<Item = Node<'a>> {
        self.document.nodes[self.index.0]
            .children
            .iter()
            .map(move |&index| self.document.node(index))
    }
    pub fn child(self, kind: Production) -> Option<Node<'a>> {
        self.children().find(|n| n.kind() == kind)
    }
    /// Preorder descendants, iteratively, without recursion proportional to source depth.
    pub fn descendants(self) -> impl Iterator<Item = Node<'a>> {
        let mut pending = vec![self.index];
        std::iter::from_fn(move || {
            let index = pending.pop()?;
            pending.extend(self.document.nodes[index.0].children.iter().rev().copied());
            Some(self.document.node(index))
        })
    }
    pub fn tokens(self) -> impl Iterator<Item = &'a Token> {
        let span = self.range();
        let first = self
            .document
            .tokens
            .partition_point(|t| t.range.start() < span.start());
        self.document.tokens[first..]
            .iter()
            .take_while(move |t| t.range.end() <= span.end())
    }
    pub fn names(self) -> impl Iterator<Item = crate::Name> + 'a {
        self.tokens().filter_map(move |t| {
            let text = self.document.token_text(t);
            let value = if t.kind == TokenKind::QuotedName {
                crate::lexer::name(t, text)
            } else if t.kind == TokenKind::Word
                && self
                    .document
                    .dialect
                    .grammar(self.document.sysml_profile)
                    .keywords
                    .binary_search(&text)
                    .is_err()
            {
                Some(text.to_owned())
            } else {
                None
            };
            value.map(|value| crate::Name {
                value,
                range: t.range,
            })
        })
    }
    pub fn origin(self) -> agq_kernel::provenance::SourceOrigin {
        agq_kernel::provenance::SourceOrigin {
            document: self.document.document,
            revision: self.document.revision,
            range: self.range(),
            syntax_node: Some(self.id()),
        }
    }
}

/// Parse an immutable source revision. Caller-supplied source identity is independent
/// of syntax nodes and cannot allocate semantic library identities.
pub fn parse(
    document: DocumentId,
    revision: SourceRevisionId,
    source: impl Into<Arc<str>>,
    limits: Limits,
) -> Result<Document, SourceError> {
    parse_with_dialect(Dialect::KerMl, document, revision, source, limits)
}

/// Parse final SysML 2.0 with the same lexer, limits, source identities and arena
/// used by KerML. Published grammar gaps remain recovery; no semantic records
/// or implied library declarations are created by syntax recognition.
pub fn parse_sysml(
    document: DocumentId,
    revision: SourceRevisionId,
    source: impl Into<Arc<str>>,
    limits: Limits,
) -> Result<Document, SourceError> {
    parse_with_dialect(Dialect::SysMl, document, revision, source, limits)
}

/// Parse final SysML with an explicit published or operational grammar authority.
/// Operational recognition never rewrites sources or counts recovery as success.
pub fn parse_sysml_with_profile(
    profile: SysmlSyntaxProfile,
    document: DocumentId,
    revision: SourceRevisionId,
    source: impl Into<Arc<str>>,
    limits: Limits,
) -> Result<Document, SourceError> {
    parse_with_profile(
        Dialect::SysMl,
        Some(profile),
        document,
        revision,
        source,
        limits,
    )
}

/// Parse an immutable revision using an explicit pinned grammar dialect.
pub fn parse_with_dialect(
    dialect: Dialect,
    document: DocumentId,
    revision: SourceRevisionId,
    source: impl Into<Arc<str>>,
    limits: Limits,
) -> Result<Document, SourceError> {
    parse_with_profile(
        dialect,
        (dialect == Dialect::SysMl).then_some(SysmlSyntaxProfile::Published),
        document,
        revision,
        source,
        limits,
    )
}

fn parse_with_profile(
    dialect: Dialect,
    sysml_profile: Option<SysmlSyntaxProfile>,
    document: DocumentId,
    revision: SourceRevisionId,
    source: impl Into<Arc<str>>,
    limits: Limits,
) -> Result<Document, SourceError> {
    let source = source.into();
    if source.len() > limits.source.max_bytes {
        return Err(SourceError::Limit("byte"));
    }
    let (tokens, diagnostics) = crate::lexer::lex(&source, limits.source.max_tokens)?;
    let mut out = Document {
        dialect,
        sysml_profile,
        document,
        revision,
        source,
        tokens,
        nodes: vec![],
        roots: vec![],
        recovery: vec![],
        diagnostics,
        discrepancies: vec![],
    };
    // Check structural nesting before chart construction. The chart itself is iterative.
    let mut depth = 0usize;
    for token in &out.tokens {
        match out.token_text(token) {
            "{" | "(" | "[" => {
                depth += 1;
                if depth > limits.source.max_depth {
                    return Err(SourceError::Limit("nesting"));
                }
            }
            "}" | ")" | "]" => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    chart::parse(&mut out, limits.max_chart_items)?;
    let discrepancies: Vec<_> = out.nodes().filter(|n| n.kind() == Production::Invariant && n.child(Production::FeatureDeclaration).is_none()).map(|n| GrammarDiscrepancy {
        code: "KG_ANONYMOUS_INVARIANT", range: n.range(), production: n.kind(),
        authority: "KerML 1.0 8.2.5.7.4 requires FeatureDeclaration; 7.4.8.5 page 57 and pinned libraries use anonymous invariants (ADR 0013)",
    }).collect();
    out.discrepancies = discrepancies;
    Ok(out)
}
