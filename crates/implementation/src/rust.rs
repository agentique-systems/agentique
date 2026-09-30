//! Just enough reading of Rust source for the supported checks: the structs
//! and enums a file declares (names, fields and their types as written,
//! variants), and which of the crate's modules a file refers to. It reads
//! tokens, not meaning: macros, `cfg` and re-exports are not followed, and a
//! check says so in its coverage.

use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeKind {
    Struct,
    Enum,
}

/// A struct or enum as written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustType {
    pub name: String,
    pub kind: TypeKind,
    /// Struct fields: (name, type as written without spaces).
    pub fields: Vec<(String, String)>,
    pub variants: Vec<String>,
    pub line: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Token {
    text: String,
    line: u32,
}

fn tokens(source: &str) -> Vec<Token> {
    let chars: Vec<char> = source.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 1u32;
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c.is_whitespace() {
            i += 1;
        } else if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && chars.get(i + 1) == Some(&'*') {
            let mut depth = 0;
            while i < chars.len() {
                if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    i += 2;
                } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    if chars[i] == '\n' {
                        line += 1;
                    }
                    i += 1;
                }
            }
        } else if c == 'r'
            && matches!(chars.get(i + 1), Some('"') | Some('#'))
            && raw_string(&chars, i).is_some()
        {
            let end = raw_string(&chars, i).expect("checked");
            line += chars[i..end].iter().filter(|c| **c == '\n').count() as u32;
            i = end;
        } else if c == '"' {
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' {
                    i += 1;
                }
                if chars.get(i) == Some(&'\n') {
                    line += 1;
                }
                i += 1;
            }
            i += 1;
        } else if c == '\'' {
            // A char literal ('a', '\n') or a lifetime ('a).
            if chars.get(i + 2) == Some(&'\'') {
                i += 3;
            } else if chars.get(i + 1) == Some(&'\\') {
                i += 2;
                while i < chars.len() && chars[i] != '\'' {
                    i += 1;
                }
                i += 1;
            } else {
                i += 1;
            }
        } else if c.is_alphanumeric() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push(Token {
                text: chars[start..i].iter().collect(),
                line,
            });
        } else if c == ':' && chars.get(i + 1) == Some(&':') {
            out.push(Token {
                text: "::".into(),
                line,
            });
            i += 2;
        } else {
            out.push(Token {
                text: c.to_string(),
                line,
            });
            i += 1;
        }
    }
    out
}

/// The end of a raw string starting at `start` (`r"…"`, `r#"…"#`), if it is one.
fn raw_string(chars: &[char], start: usize) -> Option<usize> {
    let mut i = start + 1;
    let mut hashes = 0;
    while chars.get(i) == Some(&'#') {
        hashes += 1;
        i += 1;
    }
    if chars.get(i) != Some(&'"') {
        return None;
    }
    i += 1;
    while i < chars.len() {
        if chars[i] == '"' && (0..hashes).all(|h| chars.get(i + 1 + h) == Some(&'#')) {
            return Some(i + 1 + hashes);
        }
        i += 1;
    }
    Some(chars.len())
}

/// Skips `#[...]` and `#![...]` attributes starting at `i`.
fn skip_attribute(tokens: &[Token], mut i: usize) -> usize {
    if tokens.get(i).is_some_and(|t| t.text == "#") {
        i += 1;
        if tokens.get(i).is_some_and(|t| t.text == "!") {
            i += 1;
        }
        if tokens.get(i).is_some_and(|t| t.text == "[") {
            let mut depth = 0;
            while i < tokens.len() {
                match tokens[i].text.as_str() {
                    "[" => depth += 1,
                    "]" => {
                        depth -= 1;
                        if depth == 0 {
                            return i + 1;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
        }
    }
    i
}

/// Every struct and enum the source declares.
pub fn types(source: &str) -> Vec<RustType> {
    let tokens = tokens(source);
    let mut out = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let kind = match tokens[i].text.as_str() {
            "struct" => TypeKind::Struct,
            "enum" => TypeKind::Enum,
            _ => {
                i += 1;
                continue;
            }
        };
        let Some(name) = tokens.get(i + 1) else { break };
        let line = tokens[i].line;
        let mut j = i + 2;
        // Generics: `<...>`.
        if tokens.get(j).is_some_and(|t| t.text == "<") {
            let mut depth = 0;
            while j < tokens.len() {
                match tokens[j].text.as_str() {
                    "<" => depth += 1,
                    ">" => {
                        depth -= 1;
                        if depth == 0 {
                            j += 1;
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
        }
        // `where` clauses are rare in data types; skip to the body.
        while j < tokens.len() && !matches!(tokens[j].text.as_str(), "{" | ";" | "(") {
            j += 1;
        }
        let mut rust_type = RustType {
            name: name.text.clone(),
            kind: kind.clone(),
            fields: Vec::new(),
            variants: Vec::new(),
            line,
        };
        if tokens.get(j).is_some_and(|t| t.text == "{") {
            j += 1;
            loop {
                j = skip_attribute(&tokens, j);
                let Some(token) = tokens.get(j) else { break };
                if token.text == "}" {
                    j += 1;
                    break;
                }
                if token.text == "pub" {
                    j += 1;
                    if tokens.get(j).is_some_and(|t| t.text == "(") {
                        while j < tokens.len() && tokens[j].text != ")" {
                            j += 1;
                        }
                        j += 1;
                    }
                    continue;
                }
                let item = token.text.clone();
                j += 1;
                match kind {
                    TypeKind::Struct => {
                        if tokens.get(j).is_some_and(|t| t.text == ":") {
                            j += 1;
                            let mut ty = String::new();
                            let mut depth = 0i32;
                            while let Some(t) = tokens.get(j) {
                                match t.text.as_str() {
                                    "<" | "(" | "[" => depth += 1,
                                    ">" | ")" | "]" => depth -= 1,
                                    "," | "}" if depth <= 0 => break,
                                    _ => {}
                                }
                                ty.push_str(&t.text);
                                j += 1;
                            }
                            rust_type.fields.push((item, ty));
                        }
                    }
                    TypeKind::Enum => {
                        rust_type.variants.push(item);
                        // Skip a tuple or struct variant's body and a discriminant.
                        let mut depth = 0i32;
                        while let Some(t) = tokens.get(j) {
                            match t.text.as_str() {
                                "(" | "{" | "[" => depth += 1,
                                ")" | "}" | "]" if depth > 0 => depth -= 1,
                                "," | "}" if depth == 0 => break,
                                _ => {}
                            }
                            j += 1;
                        }
                    }
                }
                if tokens.get(j).is_some_and(|t| t.text == ",") {
                    j += 1;
                }
            }
        }
        out.push(rust_type);
        i = j.max(i + 1);
    }
    out
}

/// The crate's modules a source file refers to: `crate::m`, `use crate::m`,
/// `use crate::{a, b}`.
pub fn crate_references(source: &str) -> BTreeSet<String> {
    let tokens = tokens(source);
    let mut out = BTreeSet::new();
    let mut i = 0;
    while i + 2 < tokens.len() {
        if tokens[i].text == "crate" && tokens[i + 1].text == "::" {
            let next = &tokens[i + 2].text;
            if next == "{" {
                let mut depth = 0;
                let mut expect_name = true;
                let mut j = i + 2;
                while j < tokens.len() {
                    match tokens[j].text.as_str() {
                        "{" => {
                            depth += 1;
                            expect_name = depth == 1;
                        }
                        "}" => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        "," if depth == 1 => expect_name = true,
                        name if expect_name && depth == 1 && is_ident(name) => {
                            out.insert(name.to_string());
                            expect_name = false;
                        }
                        _ => {}
                    }
                    j += 1;
                }
                i = j;
            } else if is_ident(next) {
                out.insert(next.clone());
                i += 3;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    out
}

fn is_ident(text: &str) -> bool {
    text.chars()
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_')
        && text.chars().all(|c| c.is_alphanumeric() || c == '_')
        && !matches!(text, "self" | "super" | "crate")
}

/// `longUrl` → `long_url`.
pub fn snake_case(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// `allow` → `Allow`, `invalidOutput` → `InvalidOutput`.
pub fn pascal_case(name: &str) -> String {
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE: &str = r#"
use crate::model::{Decision, Verdict};
use crate::{store, screening::Screening};
// struct NotThis { x: u8 }
/// A link.
#[derive(Clone, Debug)]
pub struct ShortLink {
    pub code: String,
    #[serde(default)]
    pub long_url: String,
    pub(crate) status: LinkStatus,
    pub reason: Option<String>,
    pub scores: HashMap<String, Vec<f64>>,
}
pub enum LinkStatus { Active, Held = 2, Blocked(String), Other { why: String } }
fn f() { let s = "struct Fake { a: u8 }"; let c = '{'; crate::api::handle(); }
"#;

    #[test]
    fn reads_structs_enums_and_module_references() {
        let types = types(SOURCE);
        let names: Vec<&str> = types.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, ["ShortLink", "LinkStatus"]);
        assert_eq!(
            types[0].fields,
            [
                ("code".to_string(), "String".to_string()),
                ("long_url".into(), "String".into()),
                ("status".into(), "LinkStatus".into()),
                ("reason".into(), "Option<String>".into()),
                ("scores".into(), "HashMap<String,Vec<f64>>".into()),
            ]
        );
        assert_eq!(types[1].variants, ["Active", "Held", "Blocked", "Other"]);
        let references: Vec<String> = crate_references(SOURCE).into_iter().collect();
        assert_eq!(references, ["api", "model", "screening", "store"]);
        assert_eq!(snake_case("longUrl"), "long_url");
        assert_eq!(pascal_case("invalidOutput"), "InvalidOutput");
    }
}
