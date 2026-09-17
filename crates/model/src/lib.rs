//! Source-linked subset of KerML Element/Namespace/Type/Feature and SysML definitions/usages.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
pub type Id = String;
pub fn new_id() -> Id {
    uuid::Uuid::new_v4().to_string()
}
pub fn derived_id(owner: &str, role: &str) -> Id {
    uuid::Uuid::new_v5(
        &uuid::Uuid::NAMESPACE_OID,
        format!("{owner}/{role}").as_bytes(),
    )
    .to_string()
}
pub fn digest(bytes: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(bytes.as_ref()))
}
pub fn json_digest<T: Serialize>(value: &T) -> String {
    digest(serde_json::to_vec(value).expect("serializable domain object"))
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Span {
    pub file: String,
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    pub severity: String,
    pub span: Option<Span>,
    pub element_id: Option<Id>,
}
impl Diagnostic {
    pub fn error(code: &str, message: impl Into<String>, span: Option<Span>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            severity: "error".into(),
            span,
            element_id: None,
        }
    }
    pub fn at(mut self, id: &str) -> Self {
        self.element_id = Some(id.into());
        self
    }
    pub fn warning(mut self) -> Self {
        self.severity = "warning".into();
        self
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, thiserror::Error)]
#[error("{code}: {message}")]
pub struct Error {
    pub code: String,
    pub message: String,
    pub diagnostics: Vec<Diagnostic>,
}
impl Error {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            diagnostics: vec![],
        }
    }
    pub fn diagnostics(code: &str, diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            code: code.into(),
            message: diagnostics
                .first()
                .map(|d| d.message.clone())
                .unwrap_or_default(),
            diagnostics,
        }
    }
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Reference {
    pub role: String,
    pub path: String,
    pub span: Span,
    pub target: Option<Id>,
    pub segments: Vec<(Span, Id)>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Relationship {
    pub id: Id,
    pub kind: String,
    pub source: Id,
    pub target: Id,
    pub implied: bool,
    pub rule: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Value {
    Boolean(bool),
    Integer(String),
    String(String),
}
impl Value {
    pub fn valid(&self) -> bool {
        match self {
            Self::Integer(s) => s.len() <= 4096 && s.parse::<num_bigint::BigInt>().is_ok(),
            _ => true,
        }
    }
    pub fn scalar_type(&self) -> &str {
        match self {
            Self::Boolean(_) => "Boolean",
            Self::Integer(_) => "Integer",
            Self::String(_) => "String",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Expr {
    Literal {
        value: Value,
    },
    Name {
        path: String,
    },
    Unary {
        op: String,
        arg: Box<Expr>,
    },
    Binary {
        op: String,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Unsupported {
        source: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Element {
    #[serde(default)]
    pub is_implied: bool,
    pub id: Id,
    pub kind: String,
    pub name: Option<String>,
    pub short_name: Option<String>,
    pub owner: Option<Id>,
    pub qualified_name: String,
    pub span: Span,
    pub name_span: Option<Span>,
    pub body_end: Option<usize>,
    pub references: Vec<Reference>,
    pub modifiers: Vec<String>,
    pub multiplicity: Option<(u64, Option<u64>)>,
    pub value: Option<Expr>,
    pub guard: Option<Expr>,
    pub documentation: String,
    pub unsupported: Vec<String>,
    pub library: bool,
}
impl Element {
    /// Mapping from compact syntax roles to OMG abstract-syntax metaclasses.
    pub fn metaclass(&self) -> &str {
        match self.kind.as_str() {
            "EntryActionUsage" => "ActionUsage",
            "PayloadParameter" => "ReferenceUsage",
            "Alias" => "Membership",
            "Unsupported" => "Element",
            other => other,
        }
    }
    pub fn definition(&self) -> bool {
        self.kind.ends_with("Definition")
            || matches!(
                self.kind.as_str(),
                "Class" | "DataType" | "Structure" | "Behavior" | "Type"
            )
    }
    pub fn reference(&self, role: &str) -> Option<&Reference> {
        self.references.iter().find(|r| r.role == role)
    }
    pub fn target(&self, role: &str) -> Option<&str> {
        self.reference(role).and_then(|r| r.target.as_deref())
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Model {
    pub sources: BTreeMap<String, String>,
    pub elements: BTreeMap<Id, Element>,
    pub relationships: Vec<Relationship>,
    pub diagnostics: Vec<Diagnostic>,
    pub library_digest: String,
}
impl Model {
    pub fn accepted(&self) -> bool {
        !self.diagnostics.iter().any(|d| d.severity == "error")
    }
    pub fn by_path(&self, path: &str) -> Option<&Element> {
        self.elements.values().find(|e| e.qualified_name == path)
    }
    pub fn children<'a>(&'a self, id: &'a str) -> impl Iterator<Item = &'a Element> {
        self.elements
            .values()
            .filter(move |e| e.owner.as_deref() == Some(id))
    }
    pub fn element(&self, id: &str) -> Result<&Element> {
        self.elements
            .get(id)
            .ok_or_else(|| Error::new("not_found", format!("Unknown element {id}")))
    }
    pub fn source_digest(&self) -> String {
        json_digest(&self.sources)
    }
}
mod work;
pub use work::{Progress, WorkControl};
