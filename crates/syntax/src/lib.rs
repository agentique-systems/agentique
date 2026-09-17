//! Bounded recursive descent grammar, lossless source storage and Pratt expressions.
use agq_model::*;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct Token {
    pub text: String,
    pub span: Span,
    pub quoted: bool,
}
pub fn lex(file: &str, source: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    lex_controlled(file, source, &WorkControl::default())
}
fn lex_controlled(file: &str, source: &str, work: &WorkControl) -> (Vec<Token>, Vec<Diagnostic>) {
    let mut tokens = vec![];
    let mut errors = vec![];
    let mut pos = 0;
    let mut line = 1;
    let mut column = 1;
    while pos < source.len() {
        if tokens.len() >= 200_000 || work.update("lexing", pos, source.len()).is_err() {
            errors.push(Diagnostic::error(
                if work.check().is_err() {
                    "cancelled"
                } else {
                    "resource_limit"
                },
                "Lexing cancelled or exceeded 200,000 tokens per file",
                None,
            ));
            break;
        }
        let start = pos;
        let start_line = line;
        let start_col = column;
        let rest = &source[pos..];
        let c = rest.chars().next().unwrap();
        let mut quoted = false;
        let mut skip = false;
        if c.is_whitespace() {
            pos += c.len_utf8();
            skip = true;
        } else if rest.starts_with("//") {
            pos += rest.find('\n').unwrap_or(rest.len());
            skip = true;
        } else if rest.starts_with("/*") {
            if let Some(end) = rest.find("*/") {
                pos += end + 2;
            } else {
                pos = source.len();
                errors.push(Diagnostic::error(
                    "parse_error",
                    "Unterminated block comment",
                    Some(Span {
                        file: file.into(),
                        start,
                        end: pos,
                        line,
                        column,
                    }),
                ));
            }
        } else if c == '\'' || c == '"' {
            quoted = true;
            pos += 1;
            let mut closed = false;
            while pos < source.len() {
                let ch = source[pos..].chars().next().unwrap();
                pos += ch.len_utf8();
                if ch == '\\' {
                    if pos < source.len() {
                        pos += source[pos..].chars().next().unwrap().len_utf8();
                    }
                } else if ch == c {
                    closed = true;
                    break;
                }
            }
            if !closed {
                errors.push(Diagnostic::error(
                    "parse_error",
                    "Unterminated quoted token",
                    Some(Span {
                        file: file.into(),
                        start,
                        end: pos,
                        line,
                        column,
                    }),
                ));
            }
        } else if c.is_alphabetic() || c == '_' {
            pos += c.len_utf8();
            while pos < source.len() {
                let x = source[pos..].chars().next().unwrap();
                if !(x.is_alphanumeric() || x == '_') {
                    break;
                }
                pos += x.len_utf8();
            }
        } else if c.is_ascii_digit() {
            pos += 1;
            while pos < source.len() && source.as_bytes()[pos].is_ascii_digit() {
                pos += 1;
            }
        } else if let Some(op) = [
            ":>>", "::>", "::", ":>", "..", "==", "!=", "<=", ">=", "&&", "||", "**", "->", ":=",
        ]
        .iter()
        .find(|op| rest.starts_with(**op))
        {
            pos += op.len();
        } else {
            pos += c.len_utf8();
        }
        let text = &source[start..pos];
        if !skip {
            tokens.push(Token {
                text: text.into(),
                span: Span {
                    file: file.into(),
                    start,
                    end: pos,
                    line: start_line,
                    column: start_col,
                },
                quoted,
            });
        }
        for ch in text.chars() {
            if ch == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
    }
    (tokens, errors)
}
fn unquote(s: &str) -> String {
    if s.starts_with('\'') && s.ends_with('\'') {
        s[1..s.len() - 1].replace("\\'", "'").replace("\\\\", "\\")
    } else {
        s.into()
    }
}
pub fn identifier(s: &str) -> String {
    if !s.is_empty()
        && s.chars()
            .enumerate()
            .all(|(i, c)| c == '_' || c.is_alphabetic() || (i > 0 && c.is_ascii_digit()))
    {
        s.into()
    } else {
        format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'"))
    }
}

pub fn parse(sources: BTreeMap<String, String>, identities: &BTreeMap<String, Id>) -> Model {
    parse_controlled(sources, identities, &WorkControl::default())
}
pub fn parse_controlled(
    sources: BTreeMap<String, String>,
    identities: &BTreeMap<String, Id>,
    work: &WorkControl,
) -> Model {
    let mut model = Model {
        sources: sources.clone(),
        ..Model::default()
    };
    if sources.len() > 2000 || sources.values().map(String::len).sum::<usize>() > 32 * 1024 * 1024 {
        model.diagnostics.push(Diagnostic::error(
            "resource_limit",
            "Project exceeds 2,000 files or 32 MiB source budget",
            None,
        ));
        return model;
    }
    for (file, source) in &sources {
        if source.len() > 8 * 1024 * 1024 {
            model.diagnostics.push(Diagnostic::error(
                "resource_limit",
                "Source exceeds 8 MiB",
                None,
            ));
            continue;
        }
        let (tokens, errors) = lex_controlled(file, source, work);
        let stopped = errors
            .iter()
            .any(|e| e.code == "cancelled" || e.code == "resource_limit");
        model.diagnostics.extend(errors);
        if stopped {
            break;
        }
        let mut parser = Parser {
            tokens,
            pos: 0,
            source,
            model: &mut model,
            identities,
            depth: 0,
            work,
        };
        parser.body(None, "", false);
    }
    model
}
struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    source: &'a str,
    model: &'a mut Model,
    identities: &'a BTreeMap<String, Id>,
    depth: usize,
    work: &'a WorkControl,
}
impl Parser<'_> {
    fn at(&self, s: &str) -> bool {
        self.tokens.get(self.pos).is_some_and(|t| t.text == s)
    }
    fn take(&mut self, s: &str) -> bool {
        if self.at(s) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn diagnostic(&mut self, msg: impl Into<String>) {
        self.model.diagnostics.push(Diagnostic::error(
            "parse_error",
            msg,
            self.tokens.get(self.pos).map(|t| t.span.clone()),
        ));
    }
    fn body(&mut self, owner: Option<Id>, prefix: &str, nested: bool) {
        self.depth += 1;
        if self.depth > 128 {
            self.diagnostic("Nesting exceeds 128");
            self.pos = self.tokens.len();
            self.depth -= 1;
            return;
        }
        while self.pos < self.tokens.len() && !self.at("}") {
            if self.model.elements.len() >= 100_000
                || self
                    .work
                    .update("parsing", self.pos, self.tokens.len())
                    .is_err()
            {
                self.model.diagnostics.push(Diagnostic::error(
                    if self.work.check().is_err() {
                        "cancelled"
                    } else {
                        "resource_limit"
                    },
                    "Parsing cancelled or exceeded 100,000 project declarations",
                    self.tokens.get(self.pos).map(|t| t.span.clone()),
                ));
                self.pos = self.tokens.len();
                break;
            }
            let before = self.pos;
            if self.at("doc") {
                self.pos += 1;
                if let Some(t) = self
                    .tokens
                    .get(self.pos)
                    .filter(|t| t.text.starts_with("/*"))
                {
                    if let Some(e) = owner
                        .as_ref()
                        .and_then(|id| self.model.elements.get_mut(id))
                    {
                        e.documentation
                            .push_str(&t.text[2..t.text.len().saturating_sub(2)]);
                    }
                    self.pos += 1;
                } else {
                    self.diagnostic("Expected documentation block");
                }
                continue;
            }
            if self.tokens[self.pos].text.starts_with("/*") || self.take(";") {
                if self.pos == before {
                    self.pos += 1;
                }
                continue;
            }
            self.declaration(owner.clone(), prefix);
            if self.pos == before {
                self.pos += 1;
            }
        }
        if nested {
            if !self.take("}") {
                self.diagnostic("Expected closing '}'");
            }
        } else if self.take("}") {
            self.diagnostic("Unexpected closing '}'");
        }
        self.depth -= 1;
    }
    fn declaration(&mut self, owner: Option<Id>, prefix: &str) {
        let start = self.pos;
        let mut modifiers = vec![];
        while self.tokens.get(self.pos).is_some_and(|t| {
            [
                "private",
                "public",
                "protected",
                "abstract",
                "standard",
                "library",
                "in",
                "out",
                "inout",
                "ref",
                "readonly",
                "derived",
                "constant",
                "composite",
                "individual",
                "variation",
                "exhibit",
                "end",
                "member",
            ]
            .contains(&t.text.as_str())
        }) {
            modifiers.push(self.tokens[self.pos].text.clone());
            self.pos += 1;
        }
        // KerML end declarations may introduce an owned cross feature before `feature`.
        // Index the actual end name, retaining the cross-feature form as unsupported semantics.
        let mut cross_feature = false;
        if modifiers.iter().any(|m| m == "end")
            && !self.at("feature")
            && self
                .tokens
                .get(self.pos)
                .is_some_and(|t| t.span.file.ends_with(".kerml"))
            && let Some(offset) = self.tokens[self.pos..]
                .iter()
                .take(64)
                .take_while(|t| ![";", "{", "}"].contains(&t.text.as_str()))
                .position(|t| t.text == "feature")
        {
            cross_feature = true;
            self.pos += offset;
        }
        let Some(key) = self.tokens.get(self.pos).cloned() else {
            self.diagnostic("Missing declaration");
            return;
        };
        self.pos += 1;
        let definition = self.take("def");
        if cross_feature {
            modifiers.push("unsupported_cross_feature".into());
        }
        let mut kind = match (key.text.as_str(), definition) {
            ("package", _) => "Package",
            ("import", _) => "NamespaceImport",
            ("alias", _) => "Alias",
            ("part", true) => "PartDefinition",
            ("part", false) => "PartUsage",
            ("item", true) => "ItemDefinition",
            ("item", false) => "ItemUsage",
            ("port", true) => "PortDefinition",
            ("port", false) => "PortUsage",
            ("state", true) => "StateDefinition",
            ("state", false) if modifiers.iter().any(|x| x == "exhibit") => "ExhibitStateUsage",
            ("state", false) => "StateUsage",
            ("attribute", true) => "AttributeDefinition",
            ("attribute", false) => "AttributeUsage",
            ("transition", _) => "TransitionUsage",
            ("then", _) => "SuccessionAsUsage",
            ("entry", _) => "EntryActionUsage",
            ("connect", _) => "ConnectionUsage",
            ("connection", true) => "ConnectionDefinition",
            ("connection", false) => "ConnectionUsage",
            ("requirement", true) => "RequirementDefinition",
            ("requirement", false) => "RequirementUsage",
            ("verification", true) => "VerificationCaseDefinition",
            ("verification", false) => "VerificationCaseUsage",
            ("objective", _) => "ObjectiveMembership",
            ("subject", _) => "ReferenceUsage",
            ("verify", _) => "RequirementVerificationMembership",
            ("action", true) => "ActionDefinition",
            ("action", false) => "ActionUsage",
            ("view", true) => "ViewDefinition",
            ("view", false) => "ViewUsage",
            ("class", _) => "Class",
            ("datatype", _) => "DataType",
            ("struct", _) => "Structure",
            ("type", _) => "Type",
            ("feature", _) => "Feature",
            ("behavior", _) => "Behavior",
            ("step", _) => "Step",
            ("bool", _) | ("predicate", _) => "Predicate",
            ("succession", _) => "Succession",
            ("binding", _) => "BindingConnector",
            ("function", _) => "Function",
            ("assoc", _) => "Association",
            _ => "Unsupported",
        }
        .to_string();
        // SysML reference usage permits omission of the feature keyword after direction/ref.
        if kind == "Unsupported"
            && modifiers
                .iter()
                .any(|m| ["in", "out", "inout", "ref", "end"].contains(&m.as_str()))
            && (is_name(&key) || [":>>", ":>", ":"].contains(&key.text.as_str()))
        {
            kind = if key.span.file.ends_with(".kerml") {
                "Feature"
            } else {
                "ReferenceUsage"
            }
            .into();
            self.pos -= 1;
        }
        if kind == "EntryActionUsage" {
            self.take("action");
        }
        if self.take("all") {
            modifiers.push("all".into());
        }
        let mut short_name = None;
        if self.take("<") {
            if let Some(t) = self.tokens.get(self.pos) {
                short_name = Some(unquote(&t.text));
                self.pos += 1;
            }
            if !self.take(">") {
                self.diagnostic("Expected '>' after short name");
            }
        }
        let mut name = None;
        let mut name_span = None;
        let anonymous = [
            "NamespaceImport",
            "SuccessionAsUsage",
            "ObjectiveMembership",
            "RequirementVerificationMembership",
            "Unsupported",
        ]
        .contains(&kind.as_str())
            || key.text == "connect";
        if !anonymous
            && self
                .tokens
                .get(self.pos)
                .is_some_and(|t| is_name(t) && !keywords().contains(&t.text.as_str()))
        {
            let t = &self.tokens[self.pos];
            name = Some(unquote(&t.text));
            name_span = Some(t.span.clone());
            self.pos += 1;
        }
        if (definition || kind == "Package") && name.is_none() {
            self.diagnostic("Definition/package requires a name");
        }
        let ordinal = self
            .model
            .elements
            .values()
            .filter(|e| {
                e.owner == owner
                    && e.kind == kind
                    && e.name.is_none()
                    && e.span.file == key.span.file
            })
            .count();
        let synthetic = format!("@{kind}:{ordinal}");
        let qn = if prefix.is_empty() {
            name.clone().unwrap_or(synthetic)
        } else {
            format!("{prefix}::{}", name.clone().unwrap_or(synthetic))
        };
        let file = self.tokens[start].span.file.clone();
        let mut id = self
            .identities
            .get(&format!("{file}|{qn}"))
            .cloned()
            .unwrap_or_else(new_id);
        if self.model.elements.contains_key(&id) {
            self.diagnostic(format!("Identity/name collision at {qn}"));
            id = new_id();
        }
        let mut e = Element {
            is_implied: false,
            id: id.clone(),
            kind: kind.clone(),
            name,
            short_name,
            owner: owner.clone(),
            qualified_name: qn.clone(),
            span: self.tokens[start].span.clone(),
            name_span,
            body_end: None,
            references: vec![],
            modifiers,
            multiplicity: None,
            value: None,
            guard: None,
            documentation: String::new(),
            unsupported: if cross_feature {
                vec!["owned cross feature".into()]
            } else {
                vec![]
            },
            library: false,
        };
        if kind == "NamespaceImport" {
            self.reference(&mut e, "import");
            if self.take("::") {
                self.take("*");
            }
        }
        if kind == "SuccessionAsUsage" {
            self.reference(&mut e, "target");
            if let Some(previous) = self
                .model
                .elements
                .values()
                .filter(|x| x.owner == owner && x.span.file == file && x.span.end <= e.span.start)
                .max_by_key(|x| x.span.end)
                && previous.kind == "EntryActionUsage"
            {
                e.modifiers.push(format!("initial_source:{}", previous.id));
            }
        }
        if kind == "RequirementVerificationMembership" {
            self.reference(&mut e, "requirement");
        }
        if key.text == "connect" {
            self.reference(&mut e, "source");
            if !self.take("to") {
                self.diagnostic("Connection requires 'to'");
            }
            self.reference(&mut e, "target");
        }
        while self.pos < self.tokens.len() && !self.at("{") && !self.at(";") && !self.at("}") {
            if kind == "Unsupported" {
                e.unsupported.push(self.tokens[self.pos].text.clone());
                self.pos += 1;
                continue;
            }
            if self.take(":") {
                if self.take("~") {
                    e.modifiers.push("conjugated".into());
                }
                self.reference(&mut e, "type");
            } else if self.take(":>") || self.take("specializes") || self.take("subsets") {
                self.reference(&mut e, "specialization");
                while self.take(",") {
                    self.reference(&mut e, "specialization");
                }
            } else if self.take(":>>") || self.take("redefines") {
                self.reference(&mut e, "redefinition");
            } else if self.take("for") && kind == "Alias" {
                self.reference(&mut e, "alias");
            } else if self.take("first") {
                self.reference(&mut e, "source");
            } else if self.take("then") {
                self.reference(&mut e, "target");
            } else if self.take("accept") {
                if self.at("after") || self.at("at") || self.at("when") {
                    e.unsupported.push("temporal/change trigger".into());
                } else {
                    let save = self.pos;
                    let count = e.references.len();
                    self.reference(&mut e, "payload_type");
                    if self.take(":") {
                        if e.references.len() > count {
                            let old = e.references.pop().unwrap();
                            e.modifiers.push(format!("payload:{}", old.path));
                        }
                        self.reference(&mut e, "payload_type");
                    }
                    if self.pos == save {
                        self.pos += 1;
                    }
                }
            } else if self.take("via") {
                self.reference(&mut e, "receiver");
            } else if self.take("if") {
                e.guard =
                    Some(self.expression_with_refs(&mut e, &["then", "do", "accept", ";", "{"]));
            } else if self.take("=") || self.take(":=") {
                e.value = Some(self.expression_with_refs(&mut e, &[";", "{"]));
            } else if self.take("[") {
                let min = self.number();
                let max = if self.take("..") {
                    if self.take("*") { None } else { self.number() }
                } else {
                    min
                };
                if !self.take("]") {
                    self.diagnostic("Expected ']' after multiplicity");
                }
                e.multiplicity = min.map(|a| (a, max));
            } else if self.take("parallel") {
                e.modifiers.push("parallel".into());
            } else {
                let t = self.tokens[self.pos].clone();
                e.unsupported.push(t.text);
                self.pos += 1;
            }
        }
        if kind == "Unsupported" {
            e.unsupported.push(key.text.clone());
        }
        if kind == "EntryActionUsage" && self.pos > start + 1 {
            e.unsupported.push("nonempty entry".into());
        }
        self.model.elements.insert(id.clone(), e);
        let transition = self.model.elements[&id].clone();
        if let Some(payload_name) = transition
            .modifiers
            .iter()
            .find_map(|m| m.strip_prefix("payload:"))
            && let Some(reference) = transition.reference("payload_type")
        {
            let mut payload = transition.clone();
            payload.id = derived_id(&id, "payload");
            payload.kind = "PayloadParameter".into();
            payload.name = Some(payload_name.into());
            payload.owner = Some(id.clone());
            payload.qualified_name = format!("{qn}::{payload_name}");
            payload.references = vec![Reference {
                role: "type".into(),
                ..reference.clone()
            }];
            payload.modifiers.clear();
            payload.guard = None;
            payload.unsupported.clear();
            payload.name_span = None;
            self.model.elements.insert(payload.id.clone(), payload);
        }
        if self.take("{") {
            self.body(Some(id.clone()), &qn, true);
            let end = self
                .tokens
                .get(self.pos.saturating_sub(1))
                .map(|t| t.span.start);
            self.model.elements.get_mut(&id).unwrap().body_end = end;
            self.take(";");
        } else if !self.take(";") {
            self.diagnostic(format!("Expected ';' or body after {kind}"));
        }
        let end = self
            .tokens
            .get(self.pos.saturating_sub(1))
            .map(|t| t.span.end)
            .unwrap_or(self.source.len());
        self.model.elements.get_mut(&id).unwrap().span.end = end;
    }
    fn number(&mut self) -> Option<u64> {
        let number = self.tokens.get(self.pos).and_then(|t| t.text.parse().ok());
        if number.is_some() {
            self.pos += 1
        } else {
            self.diagnostic("Expected unsigned multiplicity bound");
        }
        number
    }
    fn reference(&mut self, e: &mut Element, role: &str) {
        let start = self.pos;
        if !self.tokens.get(self.pos).is_some_and(is_name) {
            self.diagnostic(format!("Expected {role} reference"));
            return;
        }
        let mut parts = vec![unquote(&self.tokens[self.pos].text)];
        self.pos += 1;
        while (self.at("::") || self.at(".")) && self.tokens.get(self.pos + 1).is_some_and(is_name)
        {
            let sep = self.tokens[self.pos].text.clone();
            self.pos += 1;
            parts.push(sep);
            parts.push(unquote(&self.tokens[self.pos].text));
            self.pos += 1;
        }
        let mut span = self.tokens[start].span.clone();
        span.end = self.tokens[self.pos - 1].span.end;
        if self.at("::") && self.tokens.get(self.pos + 1).is_some_and(|t| t.text == "*") {
            self.pos += 2;
            e.modifiers.push("wildcard".into());
        }
        e.references.push(Reference {
            role: role.into(),
            path: parts.concat(),
            span,
            target: None,
            segments: vec![],
        });
    }
    fn expression_until(&mut self, stops: &[&str]) -> Expr {
        let start = self.pos;
        let mut depth = 0;
        while self.pos < self.tokens.len() {
            let t = self.tokens[self.pos].text.as_str();
            if t == "}" || (depth == 0 && stops.contains(&t)) {
                break;
            }
            if t == "(" {
                depth += 1
            }
            if t == ")" {
                if depth == 0 {
                    break;
                }
                depth -= 1
            }
            self.pos += 1;
        }
        parse_expression(&self.tokens[start..self.pos])
    }
    fn expression_with_refs(&mut self, e: &mut Element, stops: &[&str]) -> Expr {
        let start = self.pos;
        let expr = self.expression_until(stops);
        let end = self.pos;
        if !matches!(expr, Expr::Unsupported { .. }) {
            self.pos = start;
            while self.pos < end {
                let t = &self.tokens[self.pos];
                if is_name(t)
                    && !["true", "false", "and", "or", "xor", "not"].contains(&t.text.as_str())
                {
                    self.reference(e, "expression")
                } else {
                    self.pos += 1
                }
            }
            self.pos = end;
        }
        expr
    }
}
fn keywords() -> &'static [&'static str] {
    &[
        "first", "then", "accept", "via", "if", "do", "parallel", "to", "def",
    ]
}
fn is_name(t: &Token) -> bool {
    t.text.starts_with('\'')
        || t.text
            .chars()
            .next()
            .is_some_and(|c| c.is_alphabetic() || c == '_')
}
pub fn parse_expression(tokens: &[Token]) -> Expr {
    let mut nesting = 0usize;
    for t in tokens {
        if t.text == "(" {
            nesting += 1;
        }
        if t.text == ")" {
            nesting = nesting.saturating_sub(1);
        }
        if nesting > 128 || tokens.len() > 4096 {
            return Expr::Unsupported {
                source: "Expression exceeds depth/token budget".into(),
            };
        }
    }
    fn parse(tokens: &[Token], pos: &mut usize, min: u8) -> Option<Expr> {
        let t = tokens.get(*pos)?;
        *pos += 1;
        let mut left = match t.text.as_str() {
            "true" => Expr::Literal {
                value: Value::Boolean(true),
            },
            "false" => Expr::Literal {
                value: Value::Boolean(false),
            },
            "not" => Expr::Unary {
                op: "not".into(),
                arg: Box::new(parse(tokens, pos, 7)?),
            },
            "-" => {
                let t = tokens.get(*pos)?;
                *pos += 1;
                if t.text.parse::<u8>().is_ok() || t.text.chars().all(|c| c.is_ascii_digit()) {
                    Expr::Literal {
                        value: Value::Integer(format!("-{}", t.text)),
                    }
                } else {
                    return None;
                }
            }
            "(" => {
                let e = parse(tokens, pos, 0)?;
                if tokens.get(*pos)?.text != ")" {
                    return None;
                }
                *pos += 1;
                e
            }
            _ if t.text.starts_with('"') => Expr::Literal {
                value: Value::String(serde_json::from_str(&t.text).ok()?),
            },
            _ if t.text.chars().all(|c| c.is_ascii_digit()) => Expr::Literal {
                value: Value::Integer(t.text.clone()),
            },
            _ if is_name(t) => {
                let mut path = unquote(&t.text);
                while tokens
                    .get(*pos)
                    .is_some_and(|t| t.text == "." || t.text == "::")
                {
                    path.push_str(&tokens[*pos].text);
                    *pos += 1;
                    path.push_str(&unquote(&tokens.get(*pos)?.text));
                    *pos += 1;
                }
                Expr::Name { path }
            }
            _ => return None,
        };
        while let Some(op) = tokens.get(*pos) {
            let precedence = match op.text.as_str() {
                "or" | "|" => 1,
                "xor" => 2,
                "and" | "&" => 3,
                "==" | "!=" => 4,
                "<" | ">" | "<=" | ">=" => 5,
                _ => break,
            };
            if precedence < min {
                break;
            }
            *pos += 1;
            left = Expr::Binary {
                op: op.text.clone(),
                left: Box::new(left),
                right: Box::new(parse(tokens, pos, precedence + 1)?),
            };
        }
        Some(left)
    }
    let mut pos = 0;
    if let Some(e) = parse(tokens, &mut pos, 0)
        && pos == tokens.len()
    {
        return e;
    }
    Expr::Unsupported {
        source: tokens
            .iter()
            .map(|t| t.text.as_str())
            .collect::<Vec<_>>()
            .join(" "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grammar_and_recovery() {
        let mut s = BTreeMap::new();
        s.insert(
            "x.sysml".into(),
            "package P { part def A; part 'multi word' : A; state def S parallel {state x;} }"
                .into(),
        );
        let m = parse(s, &BTreeMap::new());
        assert!(m.accepted(), "{:?}", m.diagnostics);
        assert!(m.by_path("P::multi word").is_some());
        assert!(
            m.by_path("P::S")
                .unwrap()
                .modifiers
                .contains(&"parallel".into())
        );
    }
    #[test]
    fn malformed_is_not_accepted() {
        let m = parse(
            BTreeMap::from([("x.sysml".into(), "package P { part def".into())]),
            &BTreeMap::new(),
        );
        assert!(!m.accepted());
    }
    #[test]
    fn exact_expression() {
        let (t, _) = lex("x", "9999999999999999999999 > 8 and not false");
        assert!(matches!(parse_expression(&t), Expr::Binary { .. }));
    }
}
