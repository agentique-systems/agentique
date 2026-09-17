use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    pub specification: String,
    pub version: String,
    pub metamodel_uri: String,
    pub artifact_uri: String,
    pub sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Package,
    Class,
    Association,
    Enumeration,
    PrimitiveType,
    Property,
    EnumerationLiteral,
    Operation,
}

/// Source-qualified identity. Paths are arrays, so separators cannot collide.
/// A changed source digest or language version deliberately changes the mapping.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DescriptorKey {
    pub source: Source,
    pub package_path: Vec<String>,
    pub external_id: String,
    pub kind: Kind,
}

impl DescriptorKey {
    /// Frozen v1 encoding: compact JSON tuple with an explicit scheme domain.
    pub fn encoded(&self) -> Vec<u8> {
        serde_json::to_vec(&(
            "agentique-descriptor-key/1",
            &self.source.specification,
            &self.source.version,
            &self.source.metamodel_uri,
            &self.source.artifact_uri,
            &self.source.sha256,
            &self.package_path,
            &self.external_id,
            &self.kind,
        ))
        .expect("strings and arrays serialize infallibly")
    }

    pub fn uuid(&self) -> uuid::Uuid {
        // Agentique policy, explicitly NOT an OMG-mandated UUID formula.
        const DOMAIN: uuid::Uuid = uuid::Uuid::from_u128(0xa63be8e52305437788be764db0bd5da6);
        uuid::Uuid::new_v5(&DOMAIN, &self.encoded())
    }

    pub fn metaclass_id(&self) -> Result<agq_kernel::MetaclassId> {
        if self.kind != Kind::Class {
            return Err("metaclass ID requires a class key".into());
        }
        Ok(agq_kernel::MetaclassId::from_u128(self.uuid().as_u128()))
    }

    pub fn property_id(&self) -> Result<agq_kernel::PropertyId> {
        if self.kind != Kind::Property {
            return Err("property ID requires a property key".into());
        }
        Ok(agq_kernel::PropertyId::from_u128(self.uuid().as_u128()))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entity {
    pub key: DescriptorKey,
    pub name: String,
    pub qualified_path: Vec<String>,
    /// UTF-8 byte range in the exact source artifact identified by key.source.
    pub source_range: [usize; 2],
    /// Explicit attributes retained separately from UML defaults.
    pub source_attributes: BTreeMap<String, String>,
}

/// Recognized documentary/behavioral content, retained but never executed.
/// Expanded names and attributes preserve namespace identity; order is source order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceNode {
    pub tag: String,
    pub attributes: BTreeMap<String, String>,
    pub text: Option<String>,
    pub children: Vec<SourceNode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Package {
    pub entity: Entity,
    pub parent: Option<String>,
    pub uri: Option<String>,
    pub imports: Vec<String>,
    pub retained: Vec<SourceNode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Classifier {
    pub entity: Entity,
    pub package: String,
    pub is_abstract: bool,
    pub generalizations: Vec<String>,
    pub properties: Vec<String>,
    pub literals: Vec<Entity>,
    pub member_ends: Vec<String>,
    pub navigable_owned_ends: Vec<String>,
    pub retained: Vec<SourceNode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "target", rename_all = "snake_case")]
pub enum TypeRef {
    Local(String),
    External(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Upper {
    Finite(u64),
    Unlimited,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Aggregation {
    None,
    Shared,
    Composite,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Property {
    pub entity: Entity,
    /// A class OR an association. Association-owned ends are not class slots.
    pub owner: String,
    pub type_ref: TypeRef,
    pub lower: u64,
    pub upper: Upper,
    pub is_ordered: bool,
    pub is_unique: bool,
    pub is_derived: bool,
    pub is_derived_union: bool,
    pub is_read_only: bool,
    pub is_id: bool,
    pub aggregation: Aggregation,
    pub redefines: Vec<String>,
    pub subsets: Vec<String>,
    pub association: Option<String>,
    /// Other member ends, not an invented binary-only opposite rule.
    pub opposite_ends: Vec<String>,
    pub default_value: Option<SourceNode>,
    pub retained: Vec<SourceNode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metamodel {
    pub format: String,
    pub source: Source,
    /// The map keys below are document-local XMI IDs; they are never descriptor IDs.
    pub packages: BTreeMap<String, Package>,
    pub classifiers: BTreeMap<String, Classifier>,
    pub properties: BTreeMap<String, Property>,
    pub retained: Vec<SourceNode>,
}
