//! Run-time values: what attributes hold and what items carry across ports.
//!
//! Numbers follow KerML's scalar values in the subset: whole numbers
//! (`Integer`, `Natural`, `Positive`) are `Int`; `Real` is `Real`. Strings,
//! Booleans and `null` are as written. A value of an enum def is `Enum`;
//! `new T(a = …)` makes an `Item` (for item, part and attribute defs alike),
//! whose fields are keyed by feature identity and carry their names for
//! traces and for the harness.

use agq_language::ElementId;
use serde_json::{Map, Value as Json, json};
use std::fmt;

/// A value in a run.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Real(f64),
    Str(String),
    /// A value of an enum def: the enum def, the value's element and name.
    Enum {
        def: ElementId,
        value: ElementId,
        name: String,
    },
    /// An item (or part or attribute value) made by `new T(...)`.
    Item(Item),
}

/// The fields of an item, in the order of its type's features.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub ty: ElementId,
    pub type_name: String,
    pub fields: Vec<Field>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    /// The feature of the item's type (the effective one, after redefinition).
    pub feature: ElementId,
    /// Every feature this field stands for: the effective feature and the
    /// features it redefines, so a chain written against a general finds it.
    pub aliases: Vec<ElementId>,
    pub name: String,
    pub value: Value,
}

impl Item {
    /// The field for a feature (or one it redefines).
    pub fn field(&self, feature: ElementId) -> Option<&Field> {
        self.fields
            .iter()
            .find(|f| f.feature == feature || f.aliases.contains(&feature))
    }

    pub fn field_named(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name)
    }
}

impl Value {
    /// The value's kind in plain words, for messages.
    pub fn kind(&self) -> &'static str {
        match self {
            Value::Null => "nothing",
            Value::Bool(_) => "a Boolean",
            Value::Int(_) => "a whole number",
            Value::Real(_) => "a real number",
            Value::Str(_) => "a string",
            Value::Enum { .. } => "an enum value",
            Value::Item(_) => "an item",
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(n) => Some(*n as f64),
            Value::Real(r) => Some(*r),
            _ => None,
        }
    }

    /// The value as JSON for traces, recordings and the harness: items as
    /// `{"type": "T", "fields": {...}}`, enum values as their name.
    pub fn to_json(&self) -> Json {
        match self {
            Value::Null => Json::Null,
            Value::Bool(b) => json!(b),
            Value::Int(n) => json!(n),
            Value::Real(r) => json!(r),
            Value::Str(s) => json!(s),
            Value::Enum { name, .. } => json!(name),
            Value::Item(item) => {
                let mut fields = Map::new();
                for field in &item.fields {
                    fields.insert(field.name.clone(), field.value.to_json());
                }
                json!({ "type": item.type_name, "fields": fields })
            }
        }
    }

    /// Equality as `==` means it: numbers compare by value across `Int` and
    /// `Real`; items compare field by field.
    pub fn equals(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Real(b)) | (Value::Real(b), Value::Int(a)) => (*a as f64) == *b,
            (Value::Enum { value: a, .. }, Value::Enum { value: b, .. }) => a == b,
            (Value::Item(a), Value::Item(b)) => {
                a.ty == b.ty
                    && a.fields.len() == b.fields.len()
                    && a.fields
                        .iter()
                        .zip(&b.fields)
                        .all(|(x, y)| x.feature == y.feature && x.value.equals(&y.value))
            }
            _ => self == other,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => f.write_str("null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Int(n) => write!(f, "{n}"),
            Value::Real(r) => {
                if r.fract() == 0.0 && r.abs() < 1e15 {
                    write!(f, "{r:.1}")
                } else {
                    write!(f, "{r}")
                }
            }
            Value::Str(s) => write!(f, "\"{s}\""),
            Value::Enum { name, .. } => f.write_str(name),
            Value::Item(item) => {
                write!(f, "{}(", item.type_name)?;
                for (i, field) in item.fields.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{} = {}", field.name, field.value)?;
                }
                f.write_str(")")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_compare_by_value_and_items_by_field() {
        assert!(Value::Int(3).equals(&Value::Real(3.0)));
        assert!(!Value::Int(3).equals(&Value::Str("3".into())));
        let item = |ok: bool| {
            Value::Item(Item {
                ty: ElementId::from_raw(1),
                type_name: "Result".into(),
                fields: vec![Field {
                    feature: ElementId::from_raw(2),
                    aliases: vec![],
                    name: "ok".into(),
                    value: Value::Bool(ok),
                }],
            })
        };
        assert!(item(true).equals(&item(true)));
        assert!(!item(true).equals(&item(false)));
        assert_eq!(item(true).to_string(), "Result(ok = true)");
        assert_eq!(
            item(false).to_json(),
            json!({"type": "Result", "fields": {"ok": false}})
        );
    }
}
