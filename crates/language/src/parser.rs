//! Recursive-descent parser for the SysML subset.
//!
//! Every member either becomes an element of the subset or is kept verbatim as
//! an `Unsupported` or `SyntaxError` element, so nothing is lost on print and
//! every problem is reported at an element. Recovery skips to the end of the
//! member (a `;` or its closing `}` outside nested braces), never past the
//! enclosing body.

use crate::lexer::{self, Token, TokenKind};
use crate::tree::*;

/// One source document: a path (used in messages) and its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    pub path: String,
    pub text: String,
}

impl Source {
    pub fn new(path: impl Into<String>, text: impl Into<String>) -> Self {
        Source {
            path: path.into(),
            text: text.into(),
        }
    }
}

/// Parses documents into one tree and links its references (see
/// [`crate::link`]). Problems become `Unsupported` or `SyntaxError`
/// elements, which [`crate::validate`] reports.
pub fn parse(sources: &[Source]) -> Tree {
    let mut tree = Tree::new();
    parse_into(&mut tree, sources);
    crate::link(&mut tree);
    tree
}

pub(crate) fn parse_into(tree: &mut Tree, sources: &[Source]) {
    for source in sources {
        let document = tree.add_document(&source.path);
        let mut parser = Parser {
            text: &source.text,
            tokens: lexer::tokenize(&source.text),
            pos: 0,
            document,
        };
        let mut nodes = Vec::new();
        while parser.kind() != TokenKind::End {
            nodes.push(parser.member(None));
        }
        for node in nodes {
            insert(tree, Parent::Document(document), node);
        }
    }
}

/// An element with its children, built before ids are assigned so a failed
/// declaration leaves no trace in the tree.
struct Node {
    element: Element,
    children: Vec<Node>,
}

fn insert(tree: &mut Tree, parent: Parent, node: Node) {
    let id = tree
        .add(parent, node.element)
        .expect("the parent was just added");
    for child in node.children {
        insert(tree, Parent::Element(id), child);
    }
}

enum Failure {
    Unsupported(String),
    Syntax(String),
}

type Result<T> = std::result::Result<T, (Failure, usize)>;

struct Parser<'a> {
    text: &'a str,
    tokens: Vec<Token>,
    pos: usize,
    document: usize,
}

impl<'a> Parser<'a> {
    // ---- token helpers ----

    fn token(&self) -> Token {
        self.tokens[self.pos]
    }

    fn kind(&self) -> TokenKind {
        self.token().kind
    }

    fn text_of(&self, token: Token) -> &'a str {
        &self.text[token.start..token.end]
    }

    fn peek_text(&self, ahead: usize) -> &'a str {
        let token = self.tokens[(self.pos + ahead).min(self.tokens.len() - 1)];
        match token.kind {
            TokenKind::Word | TokenKind::Symbol => self.text_of(token),
            _ => "",
        }
    }

    fn at(&self, text: &str) -> bool {
        self.peek_text(0) == text
    }

    fn bump(&mut self) -> Token {
        let token = self.token();
        if token.kind != TokenKind::End {
            self.pos += 1;
        }
        token
    }

    fn eat(&mut self, text: &str) -> bool {
        let found = self.at(text);
        if found {
            self.bump();
        }
        found
    }

    fn location(&self) -> Location {
        let token = self.token();
        Location {
            document: self.document,
            line: token.line,
            column: token.column,
        }
    }

    fn syntax<T>(&self, expected: &str) -> Result<T> {
        let found = match self.kind() {
            TokenKind::End => "the end of the file".to_string(),
            TokenKind::Invalid => format!("unreadable text `{}`", self.text_of(self.token())),
            _ => format!("`{}`", self.text_of(self.token())),
        };
        Err((
            Failure::Syntax(format!("expected {expected}, found {found}")),
            self.pos,
        ))
    }

    fn unsupported<T>(&self, construct: impl Into<String>) -> Result<T> {
        Err((Failure::Unsupported(construct.into()), self.pos))
    }

    fn expect(&mut self, text: &str) -> Result<()> {
        if self.eat(text) {
            Ok(())
        } else {
            self.syntax(&format!("`{text}`"))
        }
    }

    /// A name: a non-keyword word or a quoted name.
    fn name(&mut self) -> Option<String> {
        let token = self.token();
        let is_name = match token.kind {
            TokenKind::QuotedName => true,
            TokenKind::Word => !lexer::is_keyword(self.text_of(token)),
            _ => false,
        };
        is_name.then(|| {
            self.bump();
            lexer::name_value(self.text_of(token))
        })
    }

    fn qualified_name(&mut self) -> Result<QualifiedName> {
        let mut segments = Vec::new();
        loop {
            match self.name() {
                Some(segment) => segments.push(segment),
                None => return self.syntax("a name"),
            }
            if self.at("::") && matches!(self.peek_kind(1), TokenKind::Word | TokenKind::QuotedName)
            {
                self.bump();
            } else {
                return Ok(QualifiedName { segments });
            }
        }
    }

    fn peek_kind(&self, ahead: usize) -> TokenKind {
        self.tokens[(self.pos + ahead).min(self.tokens.len() - 1)].kind
    }

    /// A name reference `A::B`, not yet linked.
    fn reference(&mut self) -> Result<Reference> {
        let name = self.qualified_name()?;
        Ok(Reference {
            steps: vec![Step { name, target: None }],
        })
    }

    /// A feature chain `a.b.c` (or a single name).
    fn chain(&mut self) -> Result<Reference> {
        let mut reference = self.reference()?;
        while self.eat(".") {
            let name = self.qualified_name()?;
            reference.steps.push(Step { name, target: None });
        }
        Ok(reference)
    }

    /// `A, B::C` after `:>` or `:>>`.
    fn references(&mut self, into: &mut Vec<Reference>) -> Result<()> {
        loop {
            into.push(self.reference()?);
            if self.at(".") {
                return self.unsupported("subsetting or redefining a feature chain");
            }
            if !self.eat(",") {
                return Ok(());
            }
        }
    }

    // ---- members ----

    /// Parses one member; never fails. `owner` is the kind of the enclosing element.
    fn member(&mut self, owner: Option<ElementKind>) -> Node {
        let start = self.pos;
        match self.declaration(owner) {
            Ok(node) => node,
            Err((failure, at)) => self.recover(start, failure, at),
        }
    }

    /// Skips the failed member: up to a `;` outside braces, or its closing
    /// `}`; a `}` that closes the enclosing body is left for the body.
    fn recover(&mut self, start: usize, failure: Failure, at: usize) -> Node {
        self.pos = start;
        let mut braces = 0usize;
        loop {
            match self.peek_text(0) {
                _ if self.kind() == TokenKind::End => break,
                "{" => braces += 1,
                "}" if braces == 0 && self.pos > start => break,
                "}" if braces <= 1 => {
                    self.bump();
                    break;
                }
                "}" => braces -= 1,
                ";" if braces == 0 => {
                    self.bump();
                    break;
                }
                _ => {}
            }
            self.bump();
        }
        let end = self.tokens[self.pos.max(start + 1) - 1].end;
        let at = self.tokens[at.min(self.tokens.len() - 1)];
        let (kind, note) = match failure {
            Failure::Unsupported(construct) => (ElementKind::Unsupported, construct),
            Failure::Syntax(message) => (ElementKind::SyntaxError, message),
        };
        let mut element = Element::new(kind);
        element.text = Some(self.text[self.tokens[start].start..end].to_string());
        element.note = Some(note);
        element.location = Some(Location {
            document: self.document,
            line: at.line,
            column: at.column,
        });
        if kind == ElementKind::Unsupported {
            element.name = self.declared_name(start, self.pos);
        }
        Node {
            element,
            children: Vec::new(),
        }
    }

    /// The name an unsupported declaration introduces: the first name after
    /// the keywords when the keyword before it declares something (`action
    /// def X`, `perform action x`), or a leading name followed by a
    /// declaration symbol (`x = 1 + 2;`). Forms such as `perform x;` or
    /// `bind x = y;` only refer to names and introduce none.
    fn declared_name(&self, start: usize, end: usize) -> Option<String> {
        #[rustfmt::skip]
        const DECLARING: &[&str] = &[
            "action", "actor", "alias", "allocation", "analysis", "attribute", "binding", "calc",
            "case", "comment", "concern", "connection", "constraint", "def", "dependency", "enum",
            "event", "flow", "individual", "interface", "item", "message", "metadata",
            "objective", "occurrence", "package", "part", "port", "ref", "rendering",
            "requirement", "snapshot", "stakeholder", "state", "subject", "succession",
            "timeslice", "verification", "view", "viewpoint",
        ];
        let tokens = &self.tokens[start..end];
        let first = tokens
            .iter()
            .position(|t| t.kind != TokenKind::Word || !lexer::is_keyword(self.text_of(*t)))?;
        let token = tokens[first];
        if !matches!(token.kind, TokenKind::Word | TokenKind::QuotedName) {
            return None;
        }
        let declares = match first {
            0 => {
                let follows = tokens.get(1).map_or("", |t| self.text_of(*t));
                matches!(follows, ":" | ";" | "{" | "[" | ":>" | ":>>" | "=")
            }
            _ => DECLARING.contains(&self.text_of(tokens[first - 1])),
        };
        declares.then(|| lexer::name_value(self.text_of(token)))
    }

    fn declaration(&mut self, owner: Option<ElementKind>) -> Result<Node> {
        if self.at("}") {
            return self.syntax("a declaration");
        }
        let visibility = match self.peek_text(0) {
            "public" => Visibility::Public,
            "private" => Visibility::Private,
            "protected" => Visibility::Protected,
            _ => Visibility::Public,
        };
        if matches!(self.peek_text(0), "public" | "private" | "protected") {
            self.bump();
        }
        let location = self.location();
        if matches!(self.peek_text(0), "#" | "@") {
            return self.unsupported("metadata `#` / `@`");
        }
        let mut node = match self.peek_text(0) {
            _ if self.kind() == TokenKind::Comment => {
                let token = self.bump();
                let mut element = Element::new(ElementKind::Comment);
                element.text = Some(comment_text(self.text_of(token)));
                Node {
                    element,
                    children: Vec::new(),
                }
            }
            "package" => self.package()?,
            "import" => self.import()?,
            "doc" => self.doc()?,
            "assert" | "satisfy" => self.satisfy()?,
            _ => self.definition_or_usage(owner)?,
        };
        node.element.visibility = visibility;
        node.element.location = Some(location);
        Ok(node)
    }

    fn body(&mut self, owner: ElementKind) -> Result<Vec<Node>> {
        if self.eat(";") {
            return Ok(Vec::new());
        }
        if !self.eat("{") {
            return self.syntax("`;` or `{`");
        }
        let mut children = Vec::new();
        while !self.eat("}") {
            if self.kind() == TokenKind::End {
                return self.syntax("`}`");
            }
            children.push(self.member(Some(owner)));
        }
        Ok(children)
    }

    fn package(&mut self) -> Result<Node> {
        self.bump();
        let mut element = Element::new(ElementKind::Package);
        element.name = Some(self.declared_identifier()?);
        let children = self.body(ElementKind::Package)?;
        Ok(Node { element, children })
    }

    /// The name of a definition or package, which the subset requires.
    fn declared_identifier(&mut self) -> Result<String> {
        if self.at("<") {
            return self.unsupported("short name `<...>`");
        }
        match self.name() {
            Some(name) => Ok(name),
            None => self.syntax("a name"),
        }
    }

    fn import(&mut self) -> Result<Node> {
        self.bump();
        if self.at("all") {
            return self.unsupported("`import all`");
        }
        let mut element = Element::new(ElementKind::Import);
        element.target = Some(self.reference()?);
        if self.at("::") && self.peek_text(1) == "*" {
            self.pos += 2;
            element.wildcard = true;
        }
        if self.at("::") && self.peek_text(1) == "**" {
            return self.unsupported("recursive import `::**`");
        }
        if self.at("[") {
            return self.unsupported("import filter `[...]`");
        }
        if self.at("{") {
            return self.unsupported("import body");
        }
        self.expect(";")?;
        Ok(Node {
            element,
            children: Vec::new(),
        })
    }

    fn doc(&mut self) -> Result<Node> {
        self.bump();
        if self.kind() != TokenKind::Comment {
            return match self.peek_text(0) {
                "locale" => self.unsupported("`doc locale`"),
                _ if self.at("<") || self.name().is_some() => self.unsupported("named doc"),
                _ => self.syntax("a `/* comment */`"),
            };
        }
        let token = self.bump();
        let mut element = Element::new(ElementKind::Doc);
        element.text = Some(comment_text(self.text_of(token)));
        Ok(Node {
            element,
            children: Vec::new(),
        })
    }

    fn satisfy(&mut self) -> Result<Node> {
        // `assert` is implied: a satisfy requirement usage is always an assertion.
        self.eat("assert");
        if self.at("not") {
            return self.unsupported("negated satisfy `not satisfy`");
        }
        self.expect("satisfy")?;
        if self.at("requirement") {
            return self.unsupported("`satisfy requirement` declaration");
        }
        let mut element = Element::new(ElementKind::Satisfy);
        element.target = Some(self.reference()?);
        if self.at(".") {
            return self.unsupported("satisfying a feature chain");
        }
        if matches!(self.peek_text(0), ":" | ":>" | ":>>" | "[" | "=") {
            return self.unsupported("declaration part on `satisfy`");
        }
        if self.eat("by") {
            element.by = Some(self.chain()?);
        }
        let children = self.body(ElementKind::Satisfy)?;
        Ok(Node { element, children })
    }

    fn definition_or_usage(&mut self, owner: Option<ElementKind>) -> Result<Node> {
        let mut element = Element::new(ElementKind::Part);
        loop {
            match self.peek_text(0) {
                "abstract" => element.is_abstract = true,
                "in" => element.direction = Some(Direction::In),
                "out" => element.direction = Some(Direction::Out),
                "inout" => element.direction = Some(Direction::InOut),
                "end" => element.is_end = true,
                "#" => return self.unsupported("metadata `#`"),
                _ => break,
            }
            self.bump();
        }
        let keyword = self.peek_text(0);
        let is_word = self.kind() == TokenKind::Word && lexer::is_keyword(keyword);
        let (definition, usage) = match keyword {
            "part" => (Some(ElementKind::PartDef), ElementKind::Part),
            "port" => (Some(ElementKind::PortDef), ElementKind::Port),
            "item" => (Some(ElementKind::ItemDef), ElementKind::Item),
            "attribute" => (Some(ElementKind::AttributeDef), ElementKind::Attribute),
            "connection" => (Some(ElementKind::ConnectionDef), ElementKind::Connection),
            "interface" => (Some(ElementKind::InterfaceDef), ElementKind::Interface),
            "requirement" => (Some(ElementKind::RequirementDef), ElementKind::Requirement),
            "subject" => (None, ElementKind::Subject),
            // `ref x : T;` is the same reference usage as `x : T;`.
            "ref"
                if self.peek_kind(1) == TokenKind::QuotedName
                    || !lexer::is_keyword(self.peek_text(1)) =>
            {
                (None, ElementKind::Reference)
            }
            "connect" => {
                // `connect a to b;`: an anonymous connection usage.
                self.bump();
                element.kind = ElementKind::Connection;
                self.connector(&mut element)?;
                let children = self.body(ElementKind::Connection)?;
                return Ok(Node { element, children });
            }
            _ if is_word => {
                let construct = match (keyword, self.peek_text(1)) {
                    ("ref", next) | (_, next @ "def") => format!("`{keyword} {next}`"),
                    _ => format!("`{keyword}`"),
                };
                return self.unsupported(construct);
            }
            // An interface definition's `end name : P;` is a port end.
            _ if element.is_end && owner == Some(ElementKind::InterfaceDef) => {
                (None, ElementKind::Port)
            }
            _ if element.is_end => return self.unsupported("`end` feature without a kind keyword"),
            // A usage without a kind keyword: `x : T;`, `:>> x = 5;`.
            ":>>" | "redefines" | ":>" | "subsets" => (None, ElementKind::Reference),
            _ if matches!(self.kind(), TokenKind::Word | TokenKind::QuotedName) => {
                (None, ElementKind::Reference)
            }
            _ => return self.syntax("a declaration"),
        };
        if is_word {
            self.bump();
        }
        if self.at("def") {
            let Some(definition) = definition else {
                return self.syntax("a name");
            };
            if element.direction.is_some() || element.is_end {
                let message = "a definition cannot have a direction or be an `end`".to_string();
                return Err((Failure::Syntax(message), self.pos));
            }
            self.bump();
            element.kind = definition;
            element.name = Some(self.declared_identifier()?);
            while self.eat(":>") || self.eat("specializes") {
                self.references(&mut element.specializes)?;
            }
            if matches!(self.peek_text(0), ":" | ":>>" | "[" | "=") {
                return self.unsupported(format!("`{}` on a definition", self.peek_text(0)));
            }
            let children = self.body(definition)?;
            return Ok(Node { element, children });
        }
        element.kind = usage;
        self.usage_declaration(&mut element)?;
        if matches!(usage, ElementKind::Connection | ElementKind::Interface) && self.eat("connect")
        {
            self.connector(&mut element)?;
        }
        let children = self.body(usage)?;
        Ok(Node { element, children })
    }

    /// Name, typing, subsetting, redefinition, multiplicity and value.
    fn usage_declaration(&mut self, element: &mut Element) -> Result<()> {
        if self.at("<") {
            return self.unsupported("short name `<...>`");
        }
        element.name = self.name();
        loop {
            match self.peek_text(0) {
                ":" => {
                    self.bump();
                    self.typings(element)?;
                }
                "defined" | "typed" if self.peek_text(1) == "by" => {
                    self.pos += 2;
                    self.typings(element)?;
                }
                ":>" | "subsets" => {
                    self.bump();
                    self.references(&mut element.specializes)?;
                }
                ":>>" | "redefines" => {
                    self.bump();
                    self.references(&mut element.redefines)?;
                }
                "[" if element.multiplicity.is_none() => {
                    element.multiplicity = Some(self.multiplicity()?);
                }
                "::>" | "references" => return self.unsupported("reference subsetting `::>`"),
                "=>" | "crosses" => return self.unsupported("cross subsetting `=>`"),
                "ordered" | "nonunique" => return self.unsupported("`ordered` / `nonunique`"),
                _ => break,
            }
        }
        match self.peek_text(0) {
            "=" => {
                self.bump();
                element.value = Some(self.literal()?);
            }
            ":=" | "default" => return self.unsupported("initial or default value"),
            _ => {}
        }
        Ok(())
    }

    fn typings(&mut self, element: &mut Element) -> Result<()> {
        loop {
            if self.eat("~") {
                element.conjugated = true;
            }
            element.typed_by.push(self.reference()?);
            if self.at(".") {
                return self.unsupported("typing by a feature chain");
            }
            if element.conjugated && element.typed_by.len() > 1 {
                return self.unsupported("a conjugated port with several types");
            }
            if !self.eat(",") {
                return Ok(());
            }
        }
    }

    fn multiplicity(&mut self) -> Result<Multiplicity> {
        self.expect("[")?;
        let first = self.bound()?;
        let (lower, upper) = if self.eat("..") {
            (first.unwrap_or(0), self.bound()?)
        } else {
            (first.unwrap_or(0), first)
        };
        if first.is_none() && upper.is_some() {
            return self.syntax("a number before `..`");
        }
        self.expect("]")?;
        Ok(Multiplicity { lower, upper })
    }

    /// A number, or `None` for `*`.
    fn bound(&mut self) -> Result<Option<u64>> {
        if self.eat("*") {
            return Ok(None);
        }
        if self.kind() != TokenKind::Integer {
            return self.unsupported("multiplicity expression");
        }
        let token = self.token();
        match self.text_of(token).parse() {
            Ok(n) => {
                self.bump();
                Ok(Some(n))
            }
            Err(_) => self.syntax("a smaller multiplicity bound"),
        }
    }

    fn literal(&mut self) -> Result<Literal> {
        let negative = self.eat("-");
        let token = self.token();
        let text = self.text_of(token);
        let sign = if negative { "-" } else { "" };
        let literal = match token.kind {
            TokenKind::Integer => Literal::Integer(format!("{sign}{text}")),
            TokenKind::Real => Literal::Real(format!("{sign}{text}")),
            TokenKind::String if !negative => Literal::String(text[1..text.len() - 1].to_string()),
            TokenKind::Word if !negative && (text == "true" || text == "false") => {
                Literal::Boolean(text == "true")
            }
            _ => return self.unsupported("feature value expression"),
        };
        self.bump();
        if !matches!(self.peek_text(0), ";" | "{") {
            return self.unsupported("feature value expression");
        }
        Ok(literal)
    }

    /// `a.b to c.d` after `connect`.
    fn connector(&mut self, element: &mut Element) -> Result<()> {
        if self.at("(") {
            return self.unsupported("n-ary connection `connect (a, b, c)`");
        }
        element.ends.push(self.connector_end()?);
        self.expect("to")?;
        element.ends.push(self.connector_end()?);
        Ok(())
    }

    fn connector_end(&mut self) -> Result<Reference> {
        if self.at("[") {
            return self.unsupported("connection end multiplicity");
        }
        if matches!(self.peek_text(1), "::>" | "references") {
            return self.unsupported("named connection end");
        }
        self.chain()
    }
}

/// The text of a `/* ... */` comment without delimiters and leading `*`s.
pub(crate) fn comment_text(raw: &str) -> String {
    let inner = raw
        .strip_prefix("/*")
        .and_then(|t| t.strip_suffix("*/"))
        .unwrap_or(raw);
    let lines: Vec<&str> = inner
        .lines()
        .enumerate()
        .map(|(i, line)| {
            let line = line.trim();
            if i == 0 {
                line
            } else {
                line.strip_prefix('*')
                    .map_or(line, |rest| rest.strip_prefix(' ').unwrap_or(rest))
            }
            .trim_end()
        })
        .collect();
    let first = lines
        .iter()
        .position(|l| !l.is_empty())
        .unwrap_or(lines.len());
    let last = lines
        .iter()
        .rposition(|l| !l.is_empty())
        .map_or(first, |i| i + 1);
    lines[first..last].join("\n")
}
