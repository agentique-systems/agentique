//! Recursive-descent parser for the SysML subset.
//!
//! Every member either becomes an element of the subset or is kept verbatim as
//! an `Unsupported` or `SyntaxError` element, so nothing is lost on print and
//! every problem is reported at an element. Recovery skips to the end of the
//! member (a `;` or its closing `}` outside nested braces), never past the
//! enclosing body.

use crate::expression::Argument;
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

/// One expression as it is written in a feature value, a guard or a check
/// (`a.b == 3`, `new T(x = 1)`), for the controls and tools that write
/// expressions without writing SysML text. Its names are not linked yet:
/// setting it on an element links them where it is placed.
pub fn parse_expression(text: &str) -> std::result::Result<Expression, String> {
    let mut parser = Parser {
        text,
        tokens: lexer::tokenize(text),
        pos: 0,
        document: 0,
    };
    let failure = |failure: Failure| match failure {
        Failure::Unsupported(what) => format!("{what} is not supported"),
        Failure::Syntax(message) => message,
    };
    let expression = parser.expression().map_err(|(f, _)| failure(f))?;
    if parser.kind() != TokenKind::End {
        return Err(format!(
            "unexpected `{}` after the expression",
            parser.text_of(parser.token())
        ));
    }
    Ok(expression)
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
            "assert" if self.peek_text(1) == "constraint" => {
                self.constraint(ElementKind::AssertConstraint)?
            }
            "assume" if self.peek_text(1) == "constraint" => {
                self.constraint(ElementKind::AssumeConstraint)?
            }
            "require" if self.peek_text(1) == "constraint" => {
                self.constraint(ElementKind::RequireConstraint)?
            }
            "assume" | "require" => {
                return self.unsupported(format!(
                    "`{}` naming a constraint declared elsewhere (write `{} constraint {{ ... }}`)",
                    self.peek_text(0),
                    self.peek_text(0)
                ));
            }
            "assert" | "satisfy" => self.satisfy()?,
            "dependency" => self.dependency()?,
            "exhibit" => self.exhibit()?,
            "state" if self.peek_text(1) != "def" => self.state()?,
            "transition" => self.transition()?,
            "entry" => self.state_action(StateAction::Entry)?,
            "exit" => self.state_action(StateAction::Exit)?,
            "do" => self.state_action(StateAction::Do)?,
            "then" => return self.then(owner),
            "action" if self.peek_text(1) != "def" => self.action()?,
            "send" => self.send()?,
            "assign" => self.assign()?,
            "if" => self.if_node()?,
            "accept" => self.accept_node()?,
            "verification" if self.peek_text(1) == "def" => self.verification_def()?,
            "objective" => self.objective()?,
            "verify" => self.verify()?,
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
                // `ref part x` / `ref item x`: a referential usage. `ref x`
                // (no kind keyword) is a reference usage, read below.
                "ref" if matches!(self.peek_text(1), "part" | "item") => element.referential = true,
                // `ref` comes right before the kind keyword (8.2.2.6.2).
                "ref"
                    if matches!(
                        self.peek_text(1),
                        "abstract"
                            | "in"
                            | "out"
                            | "inout"
                            | "end"
                            | "derived"
                            | "constant"
                            | "variation"
                    ) =>
                {
                    let message = match self.peek_text(1) {
                        "end" => {
                            "an `end` feature is always referential; write it without `ref`".into()
                        }
                        next => {
                            format!("`ref` comes after `{next}`, right before `part` or `item`")
                        }
                    };
                    return Err((Failure::Syntax(message), self.pos));
                }
                "#" => return self.unsupported("metadata `#`"),
                _ => break,
            }
            self.bump();
        }
        if element.referential && element.is_end {
            let message = "an `end` feature is always referential; write it without `ref`";
            return Err((Failure::Syntax(message.into()), self.pos));
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
            "enum" => (Some(ElementKind::EnumDef), ElementKind::Enum),
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
            // In an enum def, `a;` is an enumeration value (`enum` is optional).
            _ if owner == Some(ElementKind::EnumDef)
                && matches!(self.kind(), TokenKind::Word | TokenKind::QuotedName) =>
            {
                (None, ElementKind::Enum)
            }
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
            if element.referential {
                let message = "a definition cannot be `ref`; only a usage refers".to_string();
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
                // A single literal keeps the literal form; anything else is
                // an expression (C-50).
                match self.expression()? {
                    Expression::Literal(literal) => element.value = Some(literal),
                    expression => element.expression = Some(expression),
                }
                if !matches!(self.peek_text(0), ";" | "{") {
                    return self.unsupported("feature value expression");
                }
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

    // ---- expressions (C-50) ----

    /// An expression: KerML's operators from `if ? else` (loosest) to the
    /// unary operators, over literals, names, feature chains and `new`.
    pub(crate) fn expression(&mut self) -> Result<Expression> {
        self.binary_expression(0)
    }

    /// Operators binding tighter than `min`, all associating to the left.
    fn binary_expression(&mut self, min: u8) -> Result<Expression> {
        let mut left = self.prefix_expression()?;
        loop {
            let symbol = self.peek_text(0);
            if matches!(symbol, "|" | "&" | "===" | "!==" | "^" | "**" | "??" | "..") {
                return self.unsupported(format!("the operator `{symbol}`"));
            }
            if matches!(symbol, "istype" | "hastype" | "as" | "meta" | "@") {
                return self.unsupported(format!("`{symbol}` in an expression"));
            }
            let Some(op) = BinaryOp::from_symbol(symbol) else {
                return Ok(left);
            };
            if op.precedence() <= min {
                return Ok(left);
            }
            self.bump();
            let right = self.binary_expression(op.precedence())?;
            left = Expression::binary(op, left, right);
        }
    }

    fn prefix_expression(&mut self) -> Result<Expression> {
        match self.peek_text(0) {
            "if" => {
                self.bump();
                let condition = self.expression()?;
                self.expect("?")?;
                let then = self.expression()?;
                self.expect("else")?;
                let otherwise = self.expression()?;
                Ok(Expression::Conditional(
                    Box::new(condition),
                    Box::new(then),
                    Box::new(otherwise),
                ))
            }
            "not" => {
                self.bump();
                let operand = self.prefix_expression()?;
                Ok(Expression::Unary(UnaryOp::Not, Box::new(operand)))
            }
            "-" => {
                self.bump();
                // `-3` stays one literal, as feature values have always read it.
                let token = self.token();
                let text = self.text_of(token);
                match token.kind {
                    TokenKind::Integer => {
                        self.bump();
                        Ok(Expression::Literal(Literal::Integer(format!("-{text}"))))
                    }
                    TokenKind::Real => {
                        self.bump();
                        Ok(Expression::Literal(Literal::Real(format!("-{text}"))))
                    }
                    _ => {
                        let operand = self.prefix_expression()?;
                        Ok(Expression::Unary(UnaryOp::Negate, Box::new(operand)))
                    }
                }
            }
            "(" => {
                self.bump();
                if self.at(")") {
                    self.bump();
                    return Ok(Expression::Null);
                }
                let inner = self.expression()?;
                if self.at(",") {
                    return self.unsupported("a sequence expression `(a, b)`");
                }
                self.expect(")")?;
                Ok(inner)
            }
            "null" => {
                self.bump();
                Ok(Expression::Null)
            }
            "true" | "false" => {
                let value = self.at("true");
                self.bump();
                Ok(Expression::Literal(Literal::Boolean(value)))
            }
            "new" => self.new_expression(),
            "all" | "meta" | "{" => {
                self.unsupported(format!("`{}` in an expression", self.peek_text(0)))
            }
            _ => {
                let token = self.token();
                let text = self.text_of(token);
                let literal = match token.kind {
                    TokenKind::Integer => Some(Literal::Integer(text.to_string())),
                    TokenKind::Real => Some(Literal::Real(text.to_string())),
                    TokenKind::String => Some(Literal::String(text[1..text.len() - 1].to_string())),
                    _ => None,
                };
                if let Some(literal) = literal {
                    self.bump();
                    return Ok(Expression::Literal(literal));
                }
                if self.kind() == TokenKind::Word && lexer::is_keyword(text) {
                    return self.syntax("a value");
                }
                let name = self.chain()?;
                match self.peek_text(0) {
                    "(" => self.unsupported("calling a function or type `f(x)`"),
                    "->" => self.unsupported("`->` function calls"),
                    "#" | "[" => self.unsupported("indexing `#(i)` or `[i]`"),
                    ".?" => self.unsupported("`.?` selection"),
                    _ => Ok(Expression::Name(name)),
                }
            }
        }
    }

    /// `new T(a = value, ...)`: named arguments only.
    fn new_expression(&mut self) -> Result<Expression> {
        self.bump();
        let ty = self.reference()?;
        if self.at(".") {
            return self.unsupported("`new` with a feature chain");
        }
        self.expect("(")?;
        let mut arguments = Vec::new();
        if !self.eat(")") {
            loop {
                let is_named = matches!(self.kind(), TokenKind::Word | TokenKind::QuotedName)
                    && self.peek_text(1) == "=";
                if !is_named {
                    return self.unsupported("positional arguments of `new`");
                }
                let Some(name) = self.name() else {
                    return self.syntax("an argument name");
                };
                self.expect("=")?;
                let value = self.expression()?;
                arguments.push(Argument {
                    feature: crate::expression::argument_feature(&ty, &name),
                    value,
                });
                if self.eat(")") {
                    break;
                }
                self.expect(",")?;
            }
        }
        Ok(Expression::New { ty, arguments })
    }

    // ---- relationships and behaviour (C-50) ----

    /// `dependency [name from] A to B;` (one client, one supplier).
    fn dependency(&mut self) -> Result<Node> {
        self.bump();
        let mut element = Element::new(ElementKind::Dependency);
        if !self.eat("from") {
            element.name = Some(self.declared_identifier()?);
            self.expect("from")?;
        }
        let client = self.reference()?;
        if self.at(",") {
            return self.unsupported("a dependency with several clients");
        }
        self.expect("to")?;
        let supplier = self.reference()?;
        if self.at(",") {
            return self.unsupported("a dependency with several suppliers");
        }
        element.ends = vec![client, supplier];
        let children = self.body(ElementKind::Dependency)?;
        Ok(Node { element, children })
    }

    /// `exhibit state name { ... }`: the behaviour a part performs.
    fn exhibit(&mut self) -> Result<Node> {
        self.bump();
        if !self.at("state") {
            return self.unsupported("`exhibit` of a state defined elsewhere");
        }
        let mut node = self.state()?;
        node.element.exhibit = true;
        Ok(node)
    }

    /// `state name { ... }` or `state name;`.
    fn state(&mut self) -> Result<Node> {
        self.bump();
        let mut element = Element::new(ElementKind::State);
        self.usage_declaration(&mut element)?;
        if !element.typed_by.is_empty()
            || !element.specializes.is_empty()
            || element.multiplicity.is_some()
            || element.value.is_some()
            || element.expression.is_some()
        {
            return self.unsupported("a typed state (state defs are not supported)");
        }
        if self.at("parallel") {
            return self.unsupported("parallel states");
        }
        let children = self.body(ElementKind::State)?;
        Ok(Node { element, children })
    }

    /// `transition [name first] S [accept trigger] [if guard] [do effect] then T;`
    fn transition(&mut self) -> Result<Node> {
        self.bump();
        let mut element = Element::new(ElementKind::Transition);
        if !self.eat("first") && self.peek_text(1) == "first" {
            element.name = self.name();
            if element.name.is_none() {
                return self.syntax("a transition name");
            }
            self.expect("first")?;
        }
        let source = self.chain()?;
        let mut children = Vec::new();
        if self.eat("accept") {
            children.push(self.trigger()?);
        }
        if self.eat("if") {
            element.guard = Some(self.expression()?);
        }
        if self.eat("do") {
            children.push(self.effect()?);
        }
        self.expect("then")?;
        let target = self.chain()?;
        element.ends = vec![source, target];
        children.extend(self.body(ElementKind::Transition)?);
        Ok(Node { element, children })
    }

    /// What follows `accept`: `after duration`, or a payload `[name :] T via port`.
    fn trigger(&mut self) -> Result<Node> {
        let location = self.location();
        let mut element = Element::new(ElementKind::Accept);
        match self.peek_text(0) {
            "after" => {
                self.bump();
                element.after = true;
                element.expression = Some(self.expression()?);
            }
            "at" | "when" => {
                return self.unsupported(format!("`accept {}`", self.peek_text(0)));
            }
            _ => {
                if self.peek_text(1) == ":" {
                    element.name = self.name();
                    self.expect(":")?;
                }
                element.typed_by.push(self.reference()?);
                if self.at(".") {
                    return self.unsupported("a payload typed by a feature chain");
                }
                if self.eat("via") {
                    element.via = Some(self.chain()?);
                }
            }
        }
        element.location = Some(location);
        Ok(Node {
            element,
            children: Vec::new(),
        })
    }

    /// A transition's `do` effect: one `send`, `assign` or `action { ... }`.
    fn effect(&mut self) -> Result<Node> {
        let location = self.location();
        let mut node = match self.peek_text(0) {
            "send" => self.send_node(false)?,
            "assign" => self.assign_node(false)?,
            "action" => self.action()?,
            other => return self.unsupported(format!("`do {other}` as a transition effect")),
        };
        node.element.location = Some(location);
        Ok(node)
    }

    /// `entry`, `do` or `exit`, then `;` or one action node.
    fn state_action(&mut self, which: StateAction) -> Result<Node> {
        self.bump();
        let mut node = if self.eat(";") {
            Node {
                element: Element::new(ElementKind::Action),
                children: Vec::new(),
            }
        } else {
            match self.peek_text(0) {
                "action" => self.action()?,
                "send" => self.send()?,
                "assign" => self.assign()?,
                other => {
                    return self.unsupported(format!("`{} {other}`", which.keyword()));
                }
            }
        };
        node.element.state_action = Some(which);
        Ok(node)
    }

    /// `then S;` after an entry action (the state entered first), or
    /// `then` before the next step of an action body.
    fn then(&mut self, owner: Option<ElementKind>) -> Result<Node> {
        if matches!(
            self.peek_text(1),
            "send" | "assign" | "if" | "accept" | "action" | "assert"
        ) {
            self.bump();
            return self.declaration(owner);
        }
        let location = self.location();
        self.bump();
        let mut element = Element::new(ElementKind::Succession);
        element.target = Some(self.chain()?);
        element.location = Some(location);
        let children = self.body(ElementKind::Succession)?;
        Ok(Node { element, children })
    }

    /// `action [name] { ... }`: members run in the order written.
    fn action(&mut self) -> Result<Node> {
        self.bump();
        let mut element = Element::new(ElementKind::Action);
        if self.at("<") {
            return self.unsupported("short name `<...>`");
        }
        element.name = self.name();
        if matches!(self.peek_text(0), ":" | ":>" | ":>>" | "[" | "=") {
            return self.unsupported("an action typed by an action def");
        }
        let children = self.body(ElementKind::Action)?;
        Ok(Node { element, children })
    }

    fn send(&mut self) -> Result<Node> {
        self.send_node(true)
    }

    /// `send expression via port`, with a body when `body` (not as an effect).
    fn send_node(&mut self, body: bool) -> Result<Node> {
        self.bump();
        let mut element = Element::new(ElementKind::Send);
        element.expression = Some(self.expression()?);
        if self.eat("via") {
            element.via = Some(self.chain()?);
        }
        if self.at("to") {
            return self.unsupported("`send ... to` a receiver");
        }
        let children = if body {
            self.body(ElementKind::Send)?
        } else {
            Vec::new()
        };
        Ok(Node { element, children })
    }

    fn assign(&mut self) -> Result<Node> {
        self.assign_node(true)
    }

    /// `assign feature := expression`.
    fn assign_node(&mut self, body: bool) -> Result<Node> {
        self.bump();
        let mut element = Element::new(ElementKind::Assign);
        element.target = Some(self.chain()?);
        self.expect(":=")?;
        element.expression = Some(self.expression()?);
        let children = if body {
            self.body(ElementKind::Assign)?
        } else {
            Vec::new()
        };
        Ok(Node { element, children })
    }

    /// `if condition { ... } [else { ... } | else if ...]`
    fn if_node(&mut self) -> Result<Node> {
        self.bump();
        let mut element = Element::new(ElementKind::If);
        element.expression = Some(self.expression()?);
        let mut children = vec![self.branch()?];
        if self.eat("else") {
            if self.at("if") {
                let location = self.location();
                let mut nested = self.if_node()?;
                nested.element.location = Some(location);
                children.push(nested);
            } else {
                children.push(self.branch()?);
            }
        }
        Ok(Node { element, children })
    }

    /// One branch of an `if`: `{ ... }`, as an unnamed action.
    fn branch(&mut self) -> Result<Node> {
        let location = self.location();
        if !self.at("{") {
            return match self.peek_text(0) {
                "action" => self.unsupported("a named action as a branch of `if`"),
                _ => self.syntax("`{`"),
            };
        }
        let mut element = Element::new(ElementKind::Action);
        element.location = Some(location);
        let children = self.body(ElementKind::Action)?;
        Ok(Node { element, children })
    }

    /// `accept [name :] T via port;` or `accept after duration;` as a step.
    fn accept_node(&mut self) -> Result<Node> {
        self.bump();
        let mut node = self.trigger()?;
        if matches!(self.peek_text(0), "then" | "if" | "do") {
            return self.unsupported("a transition written as `accept ... then`; use `transition`");
        }
        node.children = self.body(ElementKind::Accept)?;
        Ok(node)
    }

    /// `verification def Name { ... }`: a scenario.
    fn verification_def(&mut self) -> Result<Node> {
        self.bump();
        self.bump();
        let mut element = Element::new(ElementKind::VerificationDef);
        element.name = Some(self.declared_identifier()?);
        while self.eat(":>") || self.eat("specializes") {
            self.references(&mut element.specializes)?;
        }
        let children = self.body(ElementKind::VerificationDef)?;
        Ok(Node { element, children })
    }

    /// `objective [name] { verify r; ... }`
    fn objective(&mut self) -> Result<Node> {
        self.bump();
        let mut element = Element::new(ElementKind::Objective);
        element.name = self.name();
        if matches!(self.peek_text(0), ":" | ":>" | ":>>" | "[" | "=") {
            return self.unsupported("a typed objective");
        }
        let children = self.body(ElementKind::Objective)?;
        Ok(Node { element, children })
    }

    /// `verify r;`
    fn verify(&mut self) -> Result<Node> {
        self.bump();
        if self.at("requirement") {
            return self.unsupported("`verify requirement` declaration");
        }
        let mut element = Element::new(ElementKind::Verify);
        element.target = Some(self.reference()?);
        if self.at(".") {
            return self.unsupported("verifying a feature chain");
        }
        let children = self.body(ElementKind::Verify)?;
        Ok(Node { element, children })
    }

    /// `assert constraint [name] { [doc] expression }`, and in a requirement
    /// `assume constraint` / `require constraint`, whose expression may be
    /// left out (an informal constraint, said by its doc), as may the body.
    fn constraint(&mut self, kind: ElementKind) -> Result<Node> {
        self.bump();
        self.bump();
        let mut element = Element::new(kind);
        element.name = self.name();
        if matches!(self.peek_text(0), ":" | ":>" | ":>>" | "[" | "=") {
            return self.unsupported("a typed constraint");
        }
        let informal = kind != ElementKind::AssertConstraint;
        if informal && self.eat(";") {
            return Ok(Node {
                element,
                children: Vec::new(),
            });
        }
        self.expect("{")?;
        let mut children = Vec::new();
        loop {
            match self.peek_text(0) {
                _ if self.kind() == TokenKind::Comment => {
                    let token = self.bump();
                    let mut comment = Element::new(ElementKind::Comment);
                    comment.text = Some(comment_text(self.text_of(token)));
                    children.push(Node {
                        element: comment,
                        children: Vec::new(),
                    });
                }
                "doc" => {
                    let location = self.location();
                    let mut doc = self.doc()?;
                    doc.element.location = Some(location);
                    children.push(doc);
                }
                _ => break,
            }
        }
        if !(informal && self.eat("}")) {
            element.expression = Some(self.expression()?);
            self.expect("}")?;
        }
        Ok(Node { element, children })
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
