//! Implementation links (ROADMAP §4.5 item 7, §4.15): `model/links.json`,
//! relating model elements to code by element identity, many to many.
//!
//! ```json
//! {
//!   "format": 1,
//!   "repository": ".",
//!   "language": "rust",
//!   "harness": ["cargo", "run", "--quiet", "--bin", "agentique-harness"],
//!   "links": [
//!     { "element": 42, "name": "UrlShortener::LinkApi", "kind": "module", "path": "src/api.rs" },
//!     { "element": 17, "name": "UrlShortener::ShortLink", "kind": "type", "path": "src/model.rs", "symbol": "ShortLink" }
//!   ],
//!   "protected": ["tests/contract.rs"]
//! }
//! ```
//!
//! `name` is the element's qualified name when the link was saved: it is
//! for people and for reporting a link whose element is gone, never for
//! finding the element.

use agq_language::{ElementId, Tree};
use serde::{Deserialize, Serialize};

pub const FORMAT: u32 = 1;

/// What a link points at in the code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LinkKind {
    /// A module or source file that implements a part.
    Module,
    /// A function, method or constant.
    Symbol,
    /// A struct or enum that implements an item or enum def.
    Type,
    /// A test that checks the element.
    Test,
    /// Where the code starts (a binary, a handler).
    EntryPoint,
    /// A schema or data format.
    Schema,
    /// Configuration.
    Config,
    /// An agent's instructions (its prompt).
    Instructions,
    /// A crate that implements a part.
    Crate,
}

impl LinkKind {
    pub const ALL: [LinkKind; 9] = [
        LinkKind::Module,
        LinkKind::Symbol,
        LinkKind::Type,
        LinkKind::Test,
        LinkKind::EntryPoint,
        LinkKind::Schema,
        LinkKind::Config,
        LinkKind::Instructions,
        LinkKind::Crate,
    ];

    pub fn label(self) -> &'static str {
        match self {
            LinkKind::Module => "module",
            LinkKind::Symbol => "symbol",
            LinkKind::Type => "type",
            LinkKind::Test => "test",
            LinkKind::EntryPoint => "entry point",
            LinkKind::Schema => "schema",
            LinkKind::Config => "configuration",
            LinkKind::Instructions => "instructions",
            LinkKind::Crate => "crate",
        }
    }

    pub fn key(self) -> &'static str {
        match self {
            LinkKind::Module => "module",
            LinkKind::Symbol => "symbol",
            LinkKind::Type => "type",
            LinkKind::Test => "test",
            LinkKind::EntryPoint => "entryPoint",
            LinkKind::Schema => "schema",
            LinkKind::Config => "config",
            LinkKind::Instructions => "instructions",
            LinkKind::Crate => "crate",
        }
    }

    pub fn from_key(key: &str) -> Option<LinkKind> {
        LinkKind::ALL.into_iter().find(|k| k.key() == key)
    }
}

/// One link from an element to code.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub element: u64,
    pub name: String,
    pub kind: LinkKind,
    /// A path in the repository, with `/`.
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
}

impl Link {
    pub fn element(&self) -> ElementId {
        ElementId::from_raw(self.element)
    }

    /// `src/api.rs`, or `src/model.rs#ShortLink`.
    pub fn location(&self) -> String {
        match &self.symbol {
            Some(symbol) => format!("{}#{symbol}", self.path),
            None => self.path.clone(),
        }
    }
}

/// A project's implementation links and the harness binding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Links {
    pub format: u32,
    /// The implementation repository, relative to the project folder
    /// (`"."` when the project folder is the code repository).
    pub repository: String,
    #[serde(default = "rust")]
    pub language: String,
    /// The command that starts the harness, run in the repository.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub harness: Vec<String>,
    #[serde(default)]
    pub links: Vec<Link>,
    /// Paths an implementation task may never change: contract tests,
    /// scenario harness expectations, evaluation cases.
    #[serde(default)]
    pub protected: Vec<String>,
}

fn rust() -> String {
    "rust".into()
}

impl Default for Links {
    fn default() -> Self {
        Links {
            format: FORMAT,
            repository: ".".into(),
            language: rust(),
            harness: Vec::new(),
            links: Vec::new(),
            protected: Vec::new(),
        }
    }
}

impl Links {
    /// Reads `links.json`, refusing other formats.
    pub fn parse(text: &str) -> Result<Links, String> {
        let value: serde_json::Value =
            serde_json::from_str(text).map_err(|e| format!("links.json: {e}"))?;
        let format = value.get("format").and_then(serde_json::Value::as_u64);
        if format != Some(u64::from(FORMAT)) {
            return Err(format!(
                "links.json has format {}; this version of Agentique reads format {FORMAT} only",
                format.map_or("none".into(), |f| f.to_string())
            ));
        }
        let links: Links = serde_json::from_value(value).map_err(|e| format!("links.json: {e}"))?;
        for link in &links.links {
            if link.path.contains("..") || link.path.starts_with('/') || link.path.contains(':') {
                return Err(format!(
                    "links.json: `{}` is not a path inside the repository",
                    link.path
                ));
            }
        }
        Ok(links)
    }

    /// The text to save: stable order, two-space indentation, a final newline.
    pub fn to_text(&self) -> String {
        let mut links = self.clone();
        links.links.sort_by(|a, b| {
            (a.element, a.kind.key(), &a.path, &a.symbol).cmp(&(
                b.element,
                b.kind.key(),
                &b.path,
                &b.symbol,
            ))
        });
        links.links.dedup();
        links.protected.sort();
        links.protected.dedup();
        let mut text = serde_json::to_string_pretty(&links).expect("links are JSON");
        text.push('\n');
        text
    }

    pub fn for_element(&self, element: ElementId) -> Vec<&Link> {
        self.links
            .iter()
            .filter(|l| l.element == element.raw())
            .collect()
    }

    /// Links whose path is `path` (code to model).
    pub fn for_path(&self, path: &str) -> Vec<&Link> {
        let path = path.replace('\\', "/");
        self.links.iter().filter(|l| l.path == path).collect()
    }

    /// Adds a link (the element's current qualified name is recorded).
    pub fn add(
        &mut self,
        tree: &Tree,
        element: ElementId,
        kind: LinkKind,
        path: &str,
        symbol: Option<&str>,
    ) {
        let link = Link {
            element: element.raw(),
            name: tree.qualified_name(element),
            kind,
            path: path.replace('\\', "/"),
            symbol: symbol.map(str::to_string),
        };
        if !self.links.contains(&link) {
            self.links.push(link);
        }
    }

    pub fn remove(&mut self, element: ElementId, path: &str, symbol: Option<&str>) -> bool {
        let before = self.links.len();
        self.links.retain(|l| {
            !(l.element == element.raw() && l.path == path && l.symbol.as_deref() == symbol)
        });
        self.links.len() != before
    }

    /// Links whose element no longer exists.
    pub fn dangling<'a>(&'a self, tree: &Tree) -> Vec<&'a Link> {
        self.links
            .iter()
            .filter(|l| !tree.contains(l.element()))
            .collect()
    }

    /// Refreshes the recorded names from the model.
    pub fn refresh_names(&mut self, tree: &Tree) {
        for link in &mut self.links {
            if tree.contains(link.element()) {
                link.name = tree.qualified_name(link.element());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agq_language::{Source, parse};

    #[test]
    fn links_round_trip_and_find_both_ways() {
        let tree = parse(&[Source::new(
            "p.sysml",
            "package P { part def Api; item def Link; }",
        )]);
        let api = tree.find("P::Api").unwrap();
        let link = tree.find("P::Link").unwrap();
        let mut links = Links {
            harness: vec!["cargo".into(), "run".into()],
            ..Links::default()
        };
        links.add(&tree, api, LinkKind::Module, "src/api.rs", None);
        links.add(
            &tree,
            link,
            LinkKind::Type,
            "src/model.rs",
            Some("ShortLink"),
        );
        links.add(
            &tree,
            link,
            LinkKind::Type,
            "src/model.rs",
            Some("ShortLink"),
        );
        let text = links.to_text();
        let again = Links::parse(&text).unwrap();
        assert_eq!(again.links.len(), 2);
        assert_eq!(again.for_element(api)[0].path, "src/api.rs");
        assert_eq!(again.for_path("src\\model.rs")[0].name, "P::Link");
        assert!(again.dangling(&tree).is_empty());
        assert!(
            Links::parse(r#"{"format": 2}"#)
                .unwrap_err()
                .contains("format 2")
        );
        assert!(
            Links::parse(r#"{"format": 1, "repository": ".", "links": [{"element": 1, "name": "x", "kind": "module", "path": "../etc"}]}"#)
                .unwrap_err()
                .contains("not a path inside")
        );
    }
}
