//! Typed slot values; missing slots represent absence, never a magic null value.
use crate::{ElementId, EnumerationLiteralId};
use std::collections::BTreeSet;

/// One scalar primitive or semantic reference. Integer is explicitly bounded.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Value {
    Boolean(bool),
    Integer(i64),
    String(String),
    Enumeration(EnumerationLiteralId),
    Reference(ElementId),
}

/// Collection shape is part of the value contract, even for singleton collections.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SlotValue {
    Scalar(Value),
    Ordered(Vec<Value>),
    Set(BTreeSet<Value>),
    /// Unordered, nonunique values; normalized to sorted order when submitted.
    Bag(Vec<Value>),
}
impl SlotValue {
    /// Values in semantic order, or canonical value order for unordered collections.
    pub fn values(&self) -> impl Iterator<Item = &Value> {
        let (slice, set): (&[Value], Option<&BTreeSet<Value>>) = match self {
            Self::Scalar(value) => (std::slice::from_ref(value), None),
            Self::Ordered(values) | Self::Bag(values) => (values, None),
            Self::Set(values) => (&[], Some(values)),
        };
        slice.iter().chain(set.into_iter().flatten())
    }
    pub(crate) fn normalize(&mut self) {
        if let Self::Bag(values) = self {
            values.sort();
        }
    }
}
