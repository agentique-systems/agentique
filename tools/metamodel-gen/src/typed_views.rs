//! Rust conveniences over the existing descriptor slice; names never allocate IDs.
use crate::{Result, descriptors::Closure, pipeline::Bundle, sha256};
use agq_kernel::metamodel::*;
use std::collections::{BTreeMap, BTreeSet};

pub const PATH: &str = "crates/kerml/src/generated/typed_views.rs";
pub const VERSION: &str = "agq-kerml-typed-views/1";

fn snake(name: &str) -> Result<String> {
    if name.is_empty()
        || !name.starts_with(|c: char| c.is_ascii_alphabetic())
        || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return Err(format!("unsupported Rust convenience name: {name}"));
    }
    let chars: Vec<_> = name.chars().collect();
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_ascii_uppercase()
            && i > 0
            && chars[i - 1] != '_'
            && (chars[i - 1].is_ascii_lowercase()
                || chars[i - 1].is_ascii_digit()
                || chars.get(i + 1).is_some_and(char::is_ascii_lowercase))
        {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
    }
    Ok(out)
}
fn method(name: &str) -> Result<String> {
    let name = snake(name)?;
    // This bounded slice uses only `type`. Refuse future collisions/keywords
    // rather than silently emitting an unusable or ambiguous public API.
    if name == "type" {
        return Ok("r#type".into());
    }
    if [
        "self", "super", "crate", "mod", "fn", "struct", "enum", "use", "pub", "impl", "trait",
        "match", "ref", "move", "const", "static", "async", "await", "loop", "in", "where", "dyn",
        "let", "return", "as", "break", "continue", "else", "extern", "false", "for", "if", "mut",
        "true", "unsafe", "while", "abstract", "become", "box", "do", "final", "gen", "macro",
        "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
    ]
    .contains(&name.as_str())
    {
        return Err(format!("unsupported Rust method keyword: {name}"));
    }
    Ok(name)
}
fn unique(names: &mut BTreeSet<String>, name: &str) -> Result<()> {
    if !names.insert(name.into()) {
        return Err(format!("colliding generated Rust name: {name}"));
    }
    Ok(())
}

pub fn source(
    bundle: &Bundle,
    selected: &Closure,
    set: &DescriptorSet,
    golden: &[u8],
) -> Result<String> {
    let spec = &bundle.metamodel.source.specification;
    let dependent = spec == "SysML";
    let mm = set
        .models
        .iter()
        .find(|m| m.name == *spec)
        .ok_or("missing owning model")?;
    let registry = MetamodelRegistry::from_descriptors(set.clone()).map_err(|e| e.to_string())?;
    let mut out = format!(
        "// GENERATED FILE. DO NOT EDIT.\n// Generator: {VERSION}\n// Reproduce: cargo run --locked --offline -p agq-metamodel-gen\n// Metamodel XMI SHA-256: {}\n// PrimitiveTypes.xmi SHA-256: {}\n// Metamodel JSON SHA-256: {}\n// Golden SHA-256: {}\n\n",
        bundle.metamodel.source.sha256,
        bundle.primitive_types.source.sha256,
        bundle.cross_check.source.sha256,
        sha256(golden)
    );
    out.push_str(&format!("/// Identity of the pinned normative metamodel.\n#[rustfmt::skip]\npub mod metamodel {{\n    pub const {}: agq_kernel::MetamodelId = agq_kernel::MetamodelId::from_u128(0x{:032x});\n}}\n", spec.to_ascii_uppercase(), mm.id.as_u128()));
    let mut class_names = BTreeMap::new();
    let mut property_names = BTreeMap::new();
    let mut assertions = String::new();
    out.push_str("/// Class IDs from source-qualified normative keys; Rust names are conveniences.\n#[rustfmt::skip]\npub mod classes {\n");
    if dependent {
        out.push_str("    pub use agq_kerml::classes::*;\n");
    }
    let mut names = BTreeSet::new();
    for id in &selected.classifiers {
        let c = &bundle.metamodel.classifiers[id];
        if c.entity.key.kind != crate::ir::Kind::Class {
            continue;
        }
        let name = snake(&c.entity.name)?.to_ascii_uppercase();
        unique(&mut names, &name)?;
        let class_id = c.entity.key.metaclass_id()?;
        class_names.insert(class_id, name.clone());
        if c.entity.key.source.specification != *spec {
            continue;
        }
        out.push_str(&format!("    /// XMI identity: `{id}`.\n    pub const {name}: agq_kernel::MetaclassId = agq_kernel::MetaclassId::from_u128(0x{:032x});\n", class_id.as_u128()));
        let id = &c.entity.key.external_id;
        assertions.push_str(&format!("        assert_eq!(crate::CLASS_IDS.iter().find(|(key, _)| *key == {id:?}).unwrap().1, classes::{name});\n        assert_eq!(views::{}::CLASS, classes::{name});\n", c.entity.name));
    }
    out.push_str("}\n\n/// Property IDs, including association-owned metadata ends (not class slots).\n#[rustfmt::skip]\npub mod properties {\n");
    if dependent {
        out.push_str("    pub use agq_kerml::properties::*;\n");
    }
    let mut names = BTreeSet::new();
    for id in &selected.properties {
        let p = &bundle.metamodel.properties[id];
        let owner = &bundle.metamodel.classifiers[&p.owner].entity.name;
        let name = format!(
            "{}_{}",
            snake(owner)?,
            if p.entity.name.is_empty() {
                "unnamed_end".into()
            } else {
                snake(&p.entity.name)?
            }
        )
        .to_ascii_uppercase();
        unique(&mut names, &name)?;
        let property_id = p.entity.key.property_id()?;
        property_names.insert(property_id, name.clone());
        if p.entity.key.source.specification != *spec {
            continue;
        }
        out.push_str(&format!("    /// XMI identity: `{id}`.\n    pub const {name}: agq_kernel::PropertyId = agq_kernel::PropertyId::from_u128(0x{:032x});\n", property_id.as_u128()));
        let id = &p.entity.key.external_id;
        assertions.push_str(&format!("        assert_eq!(crate::PROPERTY_IDS.iter().find(|(key, _)| *key == {id:?}).unwrap().1, properties::{name});\n"));
    }
    out.push_str("}\n\n/// Borrowed views for the complete pinned language abstract syntax.\n#[rustfmt::skip]\npub mod views {\n    use super::{classes, properties};\n    use agq_kernel::{ElementId, EnumerationLiteralId, EnumerationId, metamodel::ValueKind};\n    use crate::{Values, ViewError, view::{define_view, read}};\n");
    if dependent {
        out.push_str("    use agq_kerml::views::*;\n");
    }
    for c in set.classes.iter().filter(|c| c.metamodel == mm.id) {
        let name = &c.name;
        out.push_str(&format!(
            "\n    define_view!({name}, classes::{});\n    impl<'m> {name}<'m> {{\n",
            class_names[&c.id]
        ));
        let mut names =
            BTreeSet::from_iter(["try_new", "id", "model", "record", "cast"].map(String::from));
        for base in &set.classes {
            if base.id != c.id
                && registry
                    .is_subtype(c.id, base.id)
                    .map_err(|e| e.to_string())?
            {
                let method = format!("as_{}", snake(&base.name)?);
                unique(&mut names, &method)?;
                out.push_str(&format!("        /// View the same record through its normative supertype.\n        pub fn {method}(self) -> Result<{}<'m>, ViewError> {{ {}::try_new(self.id(), self.model()) }}\n", base.name, base.name));
            }
        }
        for p in registry
            .effective_properties(c.id)
            .map_err(|e| e.to_string())?
        {
            let method = method(&p.name)?;
            unique(&mut names, &method)?;
            let (ty, mut kind) = match registry
                .storage_kind(p.value_kind)
                .map_err(|e| e.to_string())?
            {
                ValueKind::Integer => (
                    "&'m agq_kernel::numeric::Integer",
                    "ValueKind::Integer".into(),
                ),
                ValueKind::Real => (
                    "&'m agq_kernel::numeric::ExactDecimal",
                    "ValueKind::Real".into(),
                ),
                ValueKind::Boolean => ("bool", "ValueKind::Boolean".into()),
                ValueKind::String => ("&'m str", "ValueKind::String".into()),
                ValueKind::Reference(target) => (
                    "ElementId",
                    format!("ValueKind::Reference(classes::{})", class_names[&target]),
                ),
                ValueKind::Enumeration(domain) => (
                    "EnumerationLiteralId",
                    format!(
                        "ValueKind::Enumeration(EnumerationId::from_u128(0x{:032x}))",
                        domain.as_u128()
                    ),
                ),
                other => return Err(format!("unsupported typed-view domain: {other:?}")),
            };
            if let ValueKind::Primitive(id) = p.value_kind {
                kind = format!(
                    "ValueKind::Primitive(agq_kernel::PrimitiveDomainId::from_u128(0x{:032x}))",
                    id.as_u128()
                );
            }
            let scalar = p.multiplicity.upper.is_some_and(|n| n <= 1);
            let optional = p.multiplicity.lower == 0;
            let (reader, result) = match (scalar, optional) {
                (true, true) => ("optional", format!("Option<{ty}>")),
                (true, false) => ("required", ty.into()),
                (false, true) => ("many", format!("Option<Values<'m, {ty}>>")),
                (false, false) => ("required_many", format!("Values<'m, {ty}>")),
            };
            let property = &property_names[&p.id];
            out.push_str(&format!("        /// Read [`properties::{property}`]; resolves effective redefinitions by identity.\n        pub fn {method}(self) -> Result<{result}, ViewError> {{ read::{reader}(self.id(), self.model(), properties::{property}, {kind}) }}\n"));
        }
        out.push_str("    }\n");
    }
    out.push_str("}\n\n#[cfg(test)]\n#[rustfmt::skip]\nmod tests {\n    use super::*;\n    #[test]\n    fn all_named_ids_match_descriptor_inventory() {\n");
    out.push_str(&assertions);
    out.push_str("    }\n}\n");
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn names_are_readable_and_collisions_fail() {
        assert_eq!(snake("ElementId").unwrap(), "element_id");
        assert_eq!(snake("URLValue").unwrap(), "url_value");
        assert_eq!(method("type").unwrap(), "r#type");
        assert!(snake("not-a-name").is_err());
        let mut names = BTreeSet::new();
        unique(&mut names, "one").unwrap();
        assert!(unique(&mut names, "one").is_err());
    }
}
