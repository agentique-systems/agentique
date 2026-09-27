//! Splits SysML text into tokens. `//` notes are dropped; `/* */` comments are
//! kept as tokens because `doc` uses them.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TokenKind {
    /// An identifier or keyword.
    Word,
    /// `'a quoted name'`; never a keyword.
    QuotedName,
    Integer,
    Real,
    /// `"text"`
    String,
    /// `/* ... */`
    Comment,
    /// Punctuation such as `{`, `::`, `:>>`.
    Symbol,
    /// A character or unterminated literal the lexer cannot read.
    Invalid,
    End,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Token {
    pub kind: TokenKind,
    pub start: usize,
    pub end: usize,
    pub line: u32,
    pub column: u32,
}

const SYMBOLS: [&str; 11] = [
    ":>>", "::>", "::", ":>", ":=", "..", "**", "=>", "->", "==", "!=",
];

pub(crate) fn tokenize(source: &str) -> Vec<Token> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let (mut i, mut line, mut line_start) = (0usize, 1u32, 0usize);
    while i < bytes.len() {
        let start = i;
        let (start_line, column) = (line, (i - line_start) as u32 + 1);
        let c = bytes[i];
        let kind = if c == b'\n' {
            i += 1;
            line += 1;
            line_start = i;
            continue;
        } else if c.is_ascii_whitespace() {
            i += 1;
            continue;
        } else if source[i..].starts_with("//") {
            if source[i..].starts_with("//*") {
                // A multi-line note: skipped like a line note.
                let Some(close) = source[i + 3..].find("*/") else {
                    i = bytes.len();
                    continue;
                };
                let end = i + 3 + close + 2;
                count_lines(source, i, end, &mut line, &mut line_start);
                i = end;
            } else {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            continue;
        } else if source[i..].starts_with("/*") {
            match source[i + 2..].find("*/") {
                Some(close) => {
                    let end = i + 2 + close + 2;
                    count_lines(source, i, end, &mut line, &mut line_start);
                    i = end;
                    TokenKind::Comment
                }
                None => {
                    i = bytes.len();
                    TokenKind::Invalid
                }
            }
        } else if c.is_ascii_alphabetic() || c == b'_' {
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            TokenKind::Word
        } else if c.is_ascii_digit() {
            lex_number(bytes, &mut i)
        } else if c == b'\'' || c == b'"' {
            i += 1;
            let mut closed = false;
            while i < bytes.len() && bytes[i] != b'\n' {
                if bytes[i] == b'\\' {
                    i += 2;
                    continue;
                }
                i += 1;
                if bytes[i - 1] == c {
                    closed = true;
                    break;
                }
            }
            i = i.min(bytes.len());
            match (closed, c) {
                (false, _) => TokenKind::Invalid,
                (true, b'\'') => TokenKind::QuotedName,
                (true, _) => TokenKind::String,
            }
        } else {
            let len = SYMBOLS
                .iter()
                .find(|s| source[i..].starts_with(*s))
                .map_or_else(
                    || source[i..].chars().next().map_or(1, char::len_utf8),
                    |s| s.len(),
                );
            i += len;
            if c.is_ascii_punctuation() {
                TokenKind::Symbol
            } else {
                TokenKind::Invalid
            }
        };
        tokens.push(Token {
            kind,
            start,
            end: i,
            line: start_line,
            column,
        });
    }
    tokens.push(Token {
        kind: TokenKind::End,
        start: source.len(),
        end: source.len(),
        line,
        column: (source.len() - line_start) as u32 + 1,
    });
    tokens
}

/// Decimal digits, optionally `.digits` and an exponent. `1..2` stays `1` `..` `2`.
fn lex_number(bytes: &[u8], i: &mut usize) -> TokenKind {
    let digits = |i: &mut usize| {
        while *i < bytes.len() && bytes[*i].is_ascii_digit() {
            *i += 1;
        }
    };
    digits(i);
    let mut kind = TokenKind::Integer;
    if bytes.get(*i) == Some(&b'.') && bytes.get(*i + 1).is_some_and(u8::is_ascii_digit) {
        *i += 1;
        digits(i);
        kind = TokenKind::Real;
    }
    if matches!(bytes.get(*i), Some(b'e' | b'E')) {
        let mut j = *i + 1;
        if matches!(bytes.get(j), Some(b'+' | b'-')) {
            j += 1;
        }
        if bytes.get(j).is_some_and(u8::is_ascii_digit) {
            *i = j;
            digits(i);
            kind = TokenKind::Real;
        }
    }
    kind
}

fn count_lines(source: &str, from: usize, to: usize, line: &mut u32, line_start: &mut usize) {
    for (offset, b) in source.as_bytes()[from..to].iter().enumerate() {
        if *b == b'\n' {
            *line += 1;
            *line_start = from + offset + 1;
        }
    }
}

/// The value of a name token: quoted names lose their quotes and escapes.
pub(crate) fn name_value(text: &str) -> String {
    let Some(inner) = text.strip_prefix('\'').and_then(|t| t.strip_suffix('\'')) else {
        return text.to_string();
    };
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                out.push(match next {
                    'n' => '\n',
                    't' => '\t',
                    other => other,
                });
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// SysML 2.0 reserved words (textual notation keywords and expression words).
#[rustfmt::skip]
const KEYWORDS: &[&str] = &[
    "about", "abstract", "accept", "action", "actor", "after", "alias", "all", "allocate",
    "allocation", "analysis", "and", "as", "assert", "assign", "assume", "at", "attribute", "bind",
    "binding", "by", "calc", "case", "comment", "concern", "connect", "connection", "constant",
    "constraint", "crosses", "decide", "def", "default", "defined", "dependency", "derived", "do",
    "doc", "else", "end", "entry", "enum", "event", "exhibit", "exit", "expose", "false", "filter",
    "first", "flow", "for", "fork", "frame", "from", "hastype", "if", "implies", "import", "in",
    "include", "individual", "inout", "interface", "istype", "item", "join", "language", "library",
    "locale", "loop", "merge", "message", "meta", "metadata", "new", "nonunique", "not", "null",
    "objective", "occurrence", "of", "or", "ordered", "out", "package", "parallel", "part",
    "perform", "port", "private", "protected", "public", "redefines", "ref", "references",
    "render", "rendering", "rep", "require", "requirement", "return", "satisfy", "send",
    "snapshot", "specializes", "stakeholder", "standard", "state", "subject", "subsets",
    "succession", "terminate", "then", "timeslice", "to", "transition", "true", "typed", "until",
    "use", "variant", "variation", "verification", "verify", "via", "view", "viewpoint", "when",
    "while", "xor",
];

pub(crate) fn is_keyword(word: &str) -> bool {
    KEYWORDS.binary_search(&word).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keywords_are_sorted_for_binary_search() {
        assert!(KEYWORDS.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn splits_symbols_numbers_and_comments() {
        let source = "a :>> b[1..*] = 1.5e3; // note\n/* doc */ 'x y'";
        let kinds: Vec<_> = tokenize(source)
            .iter()
            .map(|t| (t.kind, &source[t.start..t.end]))
            .collect();
        use TokenKind::*;
        assert_eq!(
            kinds,
            [
                (Word, "a"),
                (Symbol, ":>>"),
                (Word, "b"),
                (Symbol, "["),
                (Integer, "1"),
                (Symbol, ".."),
                (Symbol, "*"),
                (Symbol, "]"),
                (Symbol, "="),
                (Real, "1.5e3"),
                (Symbol, ";"),
                (Comment, "/* doc */"),
                (QuotedName, "'x y'"),
                (End, ""),
            ]
        );
    }
}
