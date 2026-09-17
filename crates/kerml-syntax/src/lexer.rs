use crate::*;

pub(crate) fn lex(
    source: &str,
    limit: usize,
) -> Result<(Vec<Token>, Vec<SyntaxDiagnostic>), SourceError> {
    let mut tokens = vec![];
    let mut diagnostics = vec![];
    let mut pos = 0;
    while pos < source.len() {
        if tokens.len() >= limit {
            return Err(SourceError::Limit("token"));
        }
        let start = pos;
        let rest = &source[pos..];
        let ch = rest.chars().next().unwrap();
        let mut problem = None;
        let kind = if matches!(ch, ' ' | '\t' | '\n' | '\r' | '\u{c}') {
            pos += rest
                .chars()
                .take_while(|c| matches!(c, ' ' | '\t' | '\n' | '\r' | '\u{c}'))
                .map(char::len_utf8)
                .sum::<usize>();
            TokenKind::Whitespace
        } else if rest.starts_with("//*") || rest.starts_with("/*") {
            let note = rest.starts_with("//*");
            let prefix = if note { 3 } else { 2 };
            if let Some(end) = rest[prefix..].find("*/") {
                pos += prefix + end + 2;
            } else {
                pos = source.len();
                problem = Some("Unterminated comment or note");
            }
            if note {
                TokenKind::Note
            } else {
                TokenKind::Comment
            }
        } else if rest.starts_with("//") {
            pos += rest.find(['\r', '\n']).unwrap_or(rest.len());
            TokenKind::Note
        } else if ch == '"' {
            // Unsupported expressions still need opaque string tokens, otherwise
            // recovery could interpret semicolons/keywords inside strings.
            pos += 1;
            let mut closed = false;
            while pos < source.len() {
                let c = source[pos..].chars().next().unwrap();
                pos += c.len_utf8();
                if c == '"' {
                    closed = true;
                    break;
                }
                if c == '\\'
                    && let Some(escaped) = source[pos..].chars().next()
                {
                    pos += escaped.len_utf8();
                }
            }
            if !closed {
                problem = Some("Unterminated string value");
            }
            TokenKind::StringValue
        } else if ch == '\'' {
            pos += 1;
            let mut closed = false;
            while pos < source.len() {
                let c = source[pos..].chars().next().unwrap();
                pos += c.len_utf8();
                if c == '\'' {
                    closed = true;
                    break;
                }
                if c == '\\' {
                    if let Some(escape) = source[pos..].chars().next() {
                        pos += escape.len_utf8();
                        if !matches!(escape, '\'' | '"' | 'b' | 'f' | 't' | 'n' | '\\') {
                            problem = Some("Invalid name escape");
                        }
                    }
                } else if c.is_control() {
                    problem = Some("Unescaped control character in name");
                }
            }
            if !closed {
                problem = Some("Unterminated unrestricted name");
            }
            if problem.is_none() {
                TokenKind::QuotedName
            } else {
                TokenKind::Invalid
            }
        } else if ch.is_ascii_alphabetic() || ch == '_' {
            pos += rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .map(char::len_utf8)
                .sum::<usize>();
            TokenKind::Word
        } else if let Some(symbol) = [
            ":>>", "::>", "===", "!==", "**", "->", "..", "::", ":>", "=>", "<=", ":=", "==", "!=",
            ">=", "??", ".?",
        ]
        .into_iter()
        .find(|s| rest.starts_with(s))
        {
            pos += symbol.len();
            TokenKind::Symbol
        } else if "(){}[];,~@#%&^|*+-/$.:<=>?".contains(ch) {
            pos += ch.len_utf8();
            TokenKind::Symbol
        } else {
            pos += ch.len_utf8();
            problem = Some("Unsupported lexical token");
            TokenKind::Invalid
        };
        let span = range(start, pos);
        if let Some(message) = problem {
            diagnostics.push(SyntaxDiagnostic {
                code: "KS_LEXICAL",
                range: span,
                message: message.into(),
            });
        }
        tokens.push(Token { kind, range: span });
    }
    Ok((tokens, diagnostics))
}

pub(crate) fn name(token: &Token, text: &str) -> Option<String> {
    if token.kind == TokenKind::QuotedName {
        let mut value = String::new();
        let mut chars = text[1..text.len() - 1].chars();
        while let Some(ch) = chars.next() {
            value.push(if ch == '\\' {
                match chars.next()? {
                    'b' => '\u{8}',
                    'f' => '\u{c}',
                    't' => '\t',
                    'n' => '\n',
                    c => c,
                }
            } else {
                ch
            });
        }
        return Some(value);
    }
    if token.kind == TokenKind::Word && !KEYWORDS.split_whitespace().any(|w| w == text) {
        Some(text.into())
    } else {
        None
    }
}
// KerML 1.0 8.2.2.6; reserved words are not BASIC_NAMEs in declarations.
const KEYWORDS: &str = "about abstract alias all and as assoc behavior binding bool by chains class classifier comment composite conjugate conjugates conjugation connector const crosses datatype default dependency derived differences disjoining disjoint doc else end expr false feature featured featuring filter first flow for from function hastype if implies import in inout interaction intersects inv inverse inverting istype language library locale member meta metaclass metadata multiplicity namespace nonunique not null of or ordered out package portion predicate private protected public redefines redefinition references rep return specialization specializes standard step struct subclassifier subset subsets subtype succession then to true type typed typing unions var xor";
