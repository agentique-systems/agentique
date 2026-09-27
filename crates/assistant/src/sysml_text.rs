//! Recognising SysML text in what the Operator reads (C-4: the Operator never
//! sees SysML text). The Conversation hides such lines in shown reasoning,
//! and the evaluation set fails a reply that contains them.

/// The keywords a SysML declaration line of the subset starts with.
const KEYWORDS: [&str; 26] = [
    "package ",
    "part def ",
    "port def ",
    "item def ",
    "attribute def ",
    "interface def ",
    "connection def ",
    "requirement def ",
    "abstract part def ",
    "part ",
    "port ",
    "item ",
    "attribute ",
    "in item ",
    "out item ",
    "inout item ",
    "interface ",
    "connection ",
    "requirement ",
    "satisfy ",
    "connect ",
    "end port ",
    "subject ",
    "doc /*",
    "import ",
    "private import ",
];

/// Whether `line` reads as SysML text: a `sysml` code fence, or a declaration
/// (`part def X {`, `port p : P;`, `connect a to b;`, a lone `}`), also
/// inside list markers or backticks.
pub fn is_sysml_line(line: &str) -> bool {
    let line = line.trim().trim_start_matches(['-', '*', '>', ' ']);
    if line.starts_with("```") && line.to_lowercase().contains("sysml") {
        return true;
    }
    let code = line.trim_matches('`').trim();
    code == "}"
        || ((code.ends_with('{') || code.ends_with(';'))
            && KEYWORDS.iter().any(|keyword| code.starts_with(keyword)))
}

/// Whether any line of `text` reads as SysML text.
pub fn shows_sysml(text: &str) -> bool {
    text.lines().any(is_sysml_line)
}

/// `text` with every run of SysML lines replaced by one line saying so, for
/// showing reasoning to the Operator.
pub fn without_sysml(text: &str) -> String {
    let mut lines: Vec<&str> = Vec::new();
    let mut hiding = false;
    for line in text.lines() {
        if is_sysml_line(line) {
            if !hiding {
                lines.push("[model text not shown]");
            }
            hiding = true;
        } else {
            hiding = false;
            lines.push(line);
        }
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declarations_are_recognised_and_prose_is_not() {
        assert!(is_sysml_line("part def LinkStore {"));
        assert!(is_sysml_line("- `in item save : ShortLink;`"));
        assert!(is_sysml_line("```sysml"));
        assert!(is_sysml_line("satisfy uniqueCodes by shortener.store;"));
        assert!(!is_sysml_line(
            "The store keeps a port `links` for lookups."
        ));
        assert!(!is_sysml_line("- a part def named `LinkStore`"));
        assert!(!shows_sysml("Added `UrlShortener::LinkStore`, a part def."));
    }

    #[test]
    fn runs_of_sysml_lines_are_hidden_once() {
        let text = "Plan the store.\npart def LinkStore {\n    port links : LinkStorePort;\n}\nThen connect it.";
        assert_eq!(
            without_sysml(text),
            "Plan the store.\n[model text not shown]\nThen connect it."
        );
    }
}
