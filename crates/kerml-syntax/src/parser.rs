use crate::*;

pub(crate) fn parse(doc: &mut SyntaxDocument, depth: usize) {
    let significant = doc
        .tokens
        .iter()
        .enumerate()
        .filter_map(|(i, t)| (!t.kind.is_trivia()).then_some(i))
        .collect();
    let mut parser = Parser {
        doc,
        significant,
        pos: 0,
        max_depth: depth,
    };
    parser.doc.nodes = parser.body(0, false);
}
struct Parser<'a> {
    doc: &'a mut SyntaxDocument,
    significant: Vec<usize>,
    pos: usize,
    max_depth: usize,
}
impl Parser<'_> {
    fn token(&self) -> Option<&Token> {
        self.significant.get(self.pos).map(|&i| &self.doc.tokens[i])
    }
    fn text(&self) -> &str {
        self.token().map(|t| self.doc.token_text(t)).unwrap_or("")
    }
    fn offset(&self) -> usize {
        self.token()
            .map(|t| t.range.start() as usize)
            .unwrap_or(self.doc.source.len())
    }
    fn end(&self) -> usize {
        self.pos
            .checked_sub(1)
            .and_then(|i| self.significant.get(i))
            .map(|&i| self.doc.tokens[i].range.end() as usize)
            .unwrap_or(0)
    }
    fn eat(&mut self, text: &str) -> bool {
        if self.text() == text {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn problem(&mut self, code: &'static str, span: ByteRange, message: &str) {
        self.doc.diagnostics.push(SyntaxDiagnostic {
            code,
            range: span,
            message: message.into(),
        });
    }
    fn name(&mut self) -> Option<Name> {
        let token = self.token()?;
        let value = lexer::name(token, self.text())?;
        let name = Name {
            value,
            range: token.range,
        };
        self.pos += 1;
        Some(name)
    }
    fn starts_decl(&self) -> bool {
        matches!(
            self.text(),
            "namespace"
                | "type"
                | "feature"
                | "abstract"
                | "public"
                | "private"
                | "protected"
                | "alias"
                | "import"
        )
    }
    fn visibility(&mut self) -> Option<Visibility> {
        let visibility = match self.text() {
            "public" => Visibility::Public,
            "protected" => Visibility::Protected,
            "private" => Visibility::Private,
            _ => return None,
        };
        self.pos += 1;
        Some(visibility)
    }
    fn body(&mut self, depth: usize, nested: bool) -> Vec<SyntaxNode> {
        let mut nodes = vec![];
        while self.token().is_some() {
            if nested && self.text() == "}" {
                break;
            }
            if self.starts_decl() && depth < self.max_depth {
                let start = self.offset();
                let visibility = self.visibility();
                if matches!(self.text(), "import" | "alias") {
                    nodes.push(self.namespace_reference(start, visibility));
                } else {
                    nodes.push(SyntaxNode::Declaration(self.declaration(
                        depth,
                        start,
                        visibility.unwrap_or_default(),
                    )));
                }
            } else {
                let start = self.offset();
                let code = if depth >= self.max_depth {
                    "KS_DEPTH"
                } else {
                    "KS_UNSUPPORTED"
                };
                if self.token().is_some_and(|t| t.kind == TokenKind::Comment) {
                    self.pos += 1;
                } else {
                    self.recover();
                }
                let span = range(start, self.end());
                self.problem(
                    code,
                    span,
                    "Construct is outside the supported KerML grammar slice",
                );
                nodes.push(SyntaxNode::Error {
                    id: SyntaxNodeId::new(),
                    range: span,
                });
            }
        }
        nodes
    }
    // Balanced unknown bodies are consumed as one error, so their contents cannot
    // accidentally become declarations in the surrounding namespace.
    fn recover(&mut self) {
        let mut braces = 0;
        let start = self.pos;
        while self.token().is_some() {
            if self.pos > start && braces == 0 && self.text() == "}" {
                break;
            }
            let text = self.text();
            if text == "{" {
                braces += 1;
            } else if text == "}" && braces > 0 {
                braces -= 1;
                self.pos += 1;
                if braces == 0 {
                    break;
                }
                continue;
            } else if text == ";" && braces == 0 {
                self.pos += 1;
                break;
            }
            self.pos += 1;
        }
    }
    fn namespace_reference(&mut self, start: usize, visibility: Option<Visibility>) -> SyntaxNode {
        let keyword = self.token().expect("namespace keyword").range;
        let import = self.eat("import");
        let mut valid = !import || visibility.is_some();
        let alias = if import {
            None
        } else {
            self.eat("alias");
            let name = self.name();
            valid &= name.is_some() && self.eat("for");
            name
        };
        let ref_start = self.offset();
        let absolute = self.eat("$");
        if absolute {
            valid &= self.eat("::");
        }
        let mut segments = vec![];
        let mut wildcard = false;
        loop {
            if let Some(name) = self.name() {
                segments.push(name);
            } else {
                valid = false;
                break;
            }
            if !self.eat("::") {
                break;
            }
            if import && self.eat("*") {
                wildcard = true;
                break;
            }
        }
        let reference = ReferenceSyntax {
            id: SyntaxNodeId::new(),
            kind: if import {
                ReferenceKind::NamespaceImport
            } else {
                ReferenceKind::Alias
            },
            range: range(ref_start, self.end().max(ref_start)),
            absolute,
            segments,
        };
        valid &= !import || wildcard;
        if !self.eat(";") {
            valid = false;
            if self.token().is_some() && self.text() != "}" && !self.starts_decl() {
                self.recover();
            }
        }
        let span = range(start, self.end().max(start));
        if !valid {
            self.problem("KS_NAMESPACE_REFERENCE", span, "Expected a named alias or visibility-prefixed nonrecursive namespace import ending with a semicolon");
            return SyntaxNode::Error {
                id: SyntaxNodeId::new(),
                range: span,
            };
        }
        SyntaxNode::NamespaceReference(NamespaceReferenceSyntax {
            id: SyntaxNodeId::new(),
            range: span,
            keyword,
            visibility: visibility.unwrap_or_default(),
            alias,
            reference,
        })
    }
    fn declaration(
        &mut self,
        depth: usize,
        start: usize,
        visibility: Visibility,
    ) -> DeclarationSyntax {
        let diagnostic_start = self.doc.diagnostics.len();
        let is_abstract = self.eat("abstract");
        let keyword = self
            .token()
            .map(|t| t.range)
            .unwrap_or(range(self.offset(), self.offset()));
        let kind = match self.text() {
            "namespace" => DeclarationKind::Namespace,
            "feature" => DeclarationKind::Feature,
            _ => DeclarationKind::Type,
        };
        let mut valid = matches!(self.text(), "namespace" | "type" | "feature")
            && (!is_abstract || kind == DeclarationKind::Type);
        if self.token().is_some() {
            self.pos += 1;
        }
        let name = self.name();
        if name.is_none() {
            valid = false;
            self.problem(
                "KS_NAME",
                range(self.offset(), self.offset()),
                "This slice requires a declaration name",
            );
        }
        let mut references = vec![];
        while let Some(rel) = self.relationship(kind) {
            loop {
                let ref_start = self.offset();
                let absolute = self.eat("$");
                if absolute && !self.eat("::") {
                    valid = false;
                }
                let mut segments = vec![];
                loop {
                    if let Some(name) = self.name() {
                        segments.push(name);
                    } else {
                        valid = false;
                        self.problem(
                            "KS_REFERENCE",
                            range(self.offset(), self.offset()),
                            "Expected a qualified reference name",
                        );
                        break;
                    }
                    if !self.eat("::") {
                        break;
                    }
                }
                if !segments.is_empty() && valid {
                    references.push(ReferenceSyntax {
                        id: SyntaxNodeId::new(),
                        kind: rel,
                        range: range(ref_start, self.end()),
                        absolute,
                        segments,
                    });
                }
                if !self.eat(",") {
                    break;
                }
            }
        }
        if kind == DeclarationKind::Type && references.is_empty() {
            valid = false;
            self.problem(
                "KS_TYPE_SPECIALIZATION",
                keyword,
                "KerML TypeDeclaration requires specialization or conjugation",
            );
        }
        valid &= diagnostic_start == self.doc.diagnostics.len();
        let header = range(start, self.end().max(start));
        if !valid {
            self.problem(
                "KS_HEADER",
                header,
                "Malformed or unsupported declaration header; no declaration semantics asserted",
            );
        }
        let mut children = vec![];
        let complete;
        if self.eat(";") {
            complete = true;
        } else if self.eat("{") {
            children = self.body(depth + 1, true);
            complete = self.eat("}");
            if !complete {
                self.problem(
                    "KS_BODY",
                    range(self.offset(), self.offset()),
                    "Expected closing brace",
                );
            }
        } else if self.token().is_none() || self.text() == "}" || self.starts_decl() {
            complete = false;
            self.problem(
                "KS_BODY",
                range(self.offset(), self.offset()),
                "Expected semicolon or declaration body",
            );
        } else {
            valid = false;
            complete = false;
            let bad_start = self.offset();
            self.recover();
            let span = range(bad_start, self.end());
            children.push(SyntaxNode::Error {
                id: SyntaxNodeId::new(),
                range: span,
            });
            self.problem(
                "KS_UNSUPPORTED_HEADER",
                span,
                "Unsupported declaration suffix; no declaration semantics asserted",
            );
        }
        DeclarationSyntax {
            id: SyntaxNodeId::new(),
            kind,
            range: range(start, self.end().max(start)),
            keyword,
            header,
            name,
            is_abstract,
            visibility,
            header_valid: valid,
            complete,
            references,
            children,
        }
    }
    fn relationship(&mut self, kind: DeclarationKind) -> Option<ReferenceKind> {
        let rel = match (kind, self.text()) {
            (DeclarationKind::Type, ":>" | "specializes") => ReferenceKind::Specialization,
            (DeclarationKind::Feature, ":" | "typed") => ReferenceKind::Typing,
            (DeclarationKind::Feature, ":>" | "subsets") => ReferenceKind::Subsetting,
            (DeclarationKind::Feature, ":>>" | "redefines") => ReferenceKind::Redefinition,
            _ => return None,
        };
        let typed = self.text() == "typed";
        self.pos += 1;
        if typed && !self.eat("by") {
            self.problem(
                "KS_TYPED_BY",
                range(self.offset(), self.offset()),
                "Expected 'by' after 'typed'",
            );
            // Leave an invalid token for declaration suffix recovery.
            return None;
        }
        Some(rel)
    }
}
