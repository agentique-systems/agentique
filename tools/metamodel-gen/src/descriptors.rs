//! Normative Root/Core dependency closure and deterministic runtime code emission.
use crate::{Result, canonical_json, ir::*, pipeline::Bundle, sha256};
use agq_kernel::{metamodel::*, *};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub const VERSION: &str = "agq-kerml-descriptors/1";
pub const RUST_PATH: &str = "crates/kerml/src/generated/root_core.rs";
pub const GOLDEN_PATH: &str = "standards/generated/kerml-1.0/root-core.golden.json";
pub const SEEDS: &[&str] = &[
    "Root-Elements-Element",
    "Root-Elements-Relationship",
    "Root-Namespaces-Namespace",
    "Root-Namespaces-Membership",
    "Root-Namespaces-OwningMembership",
    "Core-Types-Type",
    "Core-Types-Specialization",
    "Core-Types-FeatureMembership",
    "Core-Features-Feature",
    "Core-Features-FeatureTyping",
    "Core-Features-Subsetting",
    "Core-Features-Redefinition",
];

#[derive(Debug, PartialEq, Eq)]
pub struct Closure {
    pub classifiers: BTreeSet<String>,
    pub properties: BTreeSet<String>,
    pub external_types: BTreeSet<String>,
}

/// Traverse every structural dependency, never subclasses or name-prefix matches.
pub fn closure(model: &Metamodel, seeds: &[&str]) -> Result<Closure> {
    let mut result = Closure {
        classifiers: BTreeSet::new(),
        properties: BTreeSet::new(),
        external_types: BTreeSet::new(),
    };
    let mut pending: Vec<String> = seeds.iter().map(|s| (*s).into()).collect();
    while let Some(id) = pending.pop() {
        if let Some(c) = model.classifiers.get(&id) {
            if !result.classifiers.insert(id) {
                continue;
            }
            pending.extend(
                c.generalizations
                    .iter()
                    .chain(&c.properties)
                    .chain(&c.member_ends)
                    .chain(&c.navigable_owned_ends)
                    .cloned(),
            );
        } else if let Some(p) = model.properties.get(&id) {
            if !result.properties.insert(id) {
                continue;
            }
            pending.push(p.owner.clone());
            pending.extend(
                p.redefines
                    .iter()
                    .chain(&p.subsets)
                    .chain(&p.opposite_ends)
                    .chain(p.association.iter())
                    .cloned(),
            );
            match &p.type_ref {
                TypeRef::Local(target) => pending.push(target.clone()),
                TypeRef::External(target) => {
                    result.external_types.insert(target.clone());
                }
            }
        } else {
            return Err(format!("unresolved descriptor dependency {id}"));
        }
    }
    Ok(result)
}

fn cid(model: &Metamodel, id: &str) -> Result<MetaclassId> {
    model.classifiers[id].entity.key.metaclass_id()
}
fn pid(model: &Metamodel, id: &str) -> Result<PropertyId> {
    model.properties[id].entity.key.property_id()
}
fn aid(model: &Metamodel, id: &str) -> AssociationId {
    AssociationId::from_u128(model.classifiers[id].entity.key.uuid().as_u128())
}
fn eid(model: &Metamodel, id: &str) -> EnumerationId {
    EnumerationId::from_u128(model.classifiers[id].entity.key.uuid().as_u128())
}
fn pids(model: &Metamodel, ids: &[String]) -> Result<BTreeSet<PropertyId>> {
    ids.iter().map(|id| pid(model, id)).collect()
}

pub fn descriptor_source(entity: &Entity) -> DescriptorSource {
    DescriptorSource {
        specification: entity.key.source.specification.clone(),
        version: entity.key.source.version.clone(),
        artifact_uri: entity.key.source.artifact_uri.clone(),
        sha256: entity.key.source.sha256.clone(),
        external_id: entity.key.external_id.clone(),
        byte_range: entity.source_range,
    }
}

fn source_code(source: &DescriptorSource) -> String {
    format!(
        "DescriptorSource {{ specification: {:?}.into(), version: {:?}.into(), artifact_uri: {:?}.into(), sha256: {:?}.into(), external_id: {:?}.into(), byte_range: {:?} }}",
        source.specification,
        source.version,
        source.artifact_uri,
        source.sha256,
        source.external_id,
        source.byte_range
    )
}

fn descriptor_id_code(id: DescriptorId) -> String {
    let (variant, ty, value) = match id {
        DescriptorId::Metamodel(id) => ("Metamodel", "MetamodelId", id.as_u128()),
        DescriptorId::Class(id) => ("Class", "MetaclassId", id.as_u128()),
        DescriptorId::Property(id) => ("Property", "PropertyId", id.as_u128()),
        DescriptorId::Association(id) => ("Association", "AssociationId", id.as_u128()),
        DescriptorId::Enumeration(id) => ("Enumeration", "EnumerationId", id.as_u128()),
        DescriptorId::Primitive(id) => ("Primitive", "PrimitiveDomainId", id.as_u128()),
        DescriptorId::Literal(id) => ("Literal", "EnumerationLiteralId", id.as_u128()),
    };
    format!("DescriptorId::{variant}({})", id_code(ty, value))
}

pub fn descriptor_set(bundle: &Bundle, selected: &Closure) -> Result<DescriptorSet> {
    let output = translate_descriptors(bundle, selected)?;
    MetamodelRegistry::from_descriptors(output.clone())
        .map_err(|e| format!("generated descriptor validation: {e}"))?;
    Ok(output)
}

/// Translate exact source metadata for inspection before registration. This is
/// deliberately NOT a runtime-readiness verdict. Production emission must use
/// `descriptor_set`, which also validates the complete registry atomically.
pub fn translate_descriptors(bundle: &Bundle, selected: &Closure) -> Result<DescriptorSet> {
    let model = &bundle.metamodel;
    let mut metamodels = BTreeMap::new();
    for root in model.packages.values().filter(|p| p.parent.is_none()) {
        let source = &root.entity.key.source;
        let parts = source
            .version
            .split('.')
            .map(str::parse::<u32>)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        if !(2..=3).contains(&parts.len()) {
            return Err("unsupported specification version shape".into());
        }
        let descriptor = MetamodelDescriptor {
            id: MetamodelId::from_u128(root.entity.key.uuid().as_u128()),
            name: source.specification.clone(),
            version: Version {
                major: parts[0],
                minor: parts[1],
                patch: parts.get(2).copied().unwrap_or(0),
            },
            uri: source.metamodel_uri.clone(),
        };
        metamodels.insert(source.artifact_uri.clone(), descriptor);
    }
    let mut output = DescriptorSet {
        models: metamodels.values().cloned().collect(),
        ..DescriptorSet::default()
    };
    for root in model.packages.values().filter(|p| p.parent.is_none()) {
        output.sources.insert(
            DescriptorId::Metamodel(MetamodelId::from_u128(root.entity.key.uuid().as_u128())),
            descriptor_source(&root.entity),
        );
    }
    output.reviews = crate::baseline_diagnostics::reviews(model, selected)?;
    for id in &selected.classifiers {
        let c = &model.classifiers[id];
        let descriptor_id = match c.entity.key.kind {
            Kind::Class => DescriptorId::Class(cid(model, id)?),
            Kind::Association => DescriptorId::Association(aid(model, id)),
            Kind::Enumeration => DescriptorId::Enumeration(eid(model, id)),
            _ => return Err(format!("unsupported classifier {id}")),
        };
        output
            .sources
            .insert(descriptor_id, descriptor_source(&c.entity));
        for literal in &c.literals {
            output.sources.insert(
                DescriptorId::Literal(EnumerationLiteralId::from_u128(
                    literal.key.uuid().as_u128(),
                )),
                descriptor_source(literal),
            );
        }
        let metamodel = metamodels[&c.entity.key.source.artifact_uri].id;
        let name = c.entity.name.clone();
        let package = c.entity.key.package_path.clone();
        match c.entity.key.kind {
            Kind::Class => output.classes.push(MetaclassDescriptor {
                id: cid(model, id)?,
                name,
                package,
                metamodel,
                direct_supertypes: c
                    .generalizations
                    .iter()
                    .map(|id| cid(model, id))
                    .collect::<Result<_>>()?,
                is_abstract: c.is_abstract,
            }),
            Kind::Association => {
                output.associations.push(AssociationDescriptor {
                    id: aid(model, id),
                    name,
                    package,
                    metamodel,
                    member_ends: c
                        .member_ends
                        .iter()
                        .map(|id| pid(model, id))
                        .collect::<Result<_>>()?,
                    navigable_owned_ends: pids(model, &c.navigable_owned_ends)?,
                    direct_supertypes: c.generalizations.iter().map(|id| aid(model, id)).collect(),
                    is_abstract: c.is_abstract,
                });
            }
            Kind::Enumeration => output.enumerations.push(EnumerationDescriptor {
                id: eid(model, id),
                name,
                package,
                metamodel,
                literals: c
                    .literals
                    .iter()
                    .map(|l| {
                        (
                            EnumerationLiteralId::from_u128(l.key.uuid().as_u128()),
                            l.name.clone(),
                        )
                    })
                    .collect(),
            }),
            _ => {
                return Err(format!(
                    "unsupported classifier in descriptor closure: {id}"
                ));
            }
        }
    }
    for id in &selected.properties {
        let p = &model.properties[id];
        output.sources.insert(
            DescriptorId::Property(pid(model, id)?),
            descriptor_source(&p.entity),
        );
        // No property with these semantics is silently approximated. All are
        // retained in the golden, including false/default values in this slice.
        if p.is_read_only || p.aggregation == Aggregation::Shared {
            return Err(format!("unsupported read-only/shared property: {id}"));
        }
        let owner = match model.classifiers[&p.owner].entity.key.kind {
            Kind::Class => PropertyOwner::Class(cid(model, &p.owner)?),
            Kind::Association => PropertyOwner::Association(aid(model, &p.owner)),
            _ => return Err(format!("unsupported property owner: {id}")),
        };
        let value_kind = match &p.type_ref {
            TypeRef::Local(target) => match model.classifiers[target].entity.key.kind {
                Kind::Class => ValueKind::Reference(cid(model, target)?),
                Kind::Enumeration => ValueKind::Enumeration(eid(model, target)),
                _ => return Err(format!("unsupported property target: {id}")),
            },
            TypeRef::External(target) => {
                let primitive = bundle
                    .primitive_types
                    .classifiers
                    .values()
                    .find(|c| {
                        format!(
                            "{}#{}",
                            c.entity.key.source.artifact_uri, c.entity.key.external_id
                        ) == *target
                    })
                    .ok_or_else(|| format!("unresolved primitive {target}"))?;
                let representation = match primitive.entity.key.external_id.as_str() {
                    "Boolean" => PrimitiveRepresentation::Boolean,
                    "String" => PrimitiveRepresentation::String,
                    "Integer" => PrimitiveRepresentation::Integer,
                    "Real" => PrimitiveRepresentation::Real,
                    _ => return Err(format!("unsupported primitive storage domain: {target}")),
                };
                let domain = PrimitiveDomainId::from_u128(primitive.entity.key.uuid().as_u128());
                if !output.primitives.iter().any(|d| d.id == domain) {
                    output.primitives.push(PrimitiveDescriptor {
                        id: domain,
                        name: primitive.entity.name.clone(),
                        representation,
                    });
                    output.sources.insert(
                        DescriptorId::Primitive(domain),
                        descriptor_source(&primitive.entity),
                    );
                }
                ValueKind::Primitive(domain)
            }
        };
        let bound = |v| usize::try_from(v).map_err(|_| format!("bound not representable: {id}"));
        output.properties.push(PropertyDescriptor {
            id: pid(model, id)?,
            name: p.entity.name.clone(),
            owner,
            value_kind,
            multiplicity: Multiplicity {
                lower: bound(p.lower)?,
                upper: match p.upper {
                    Upper::Finite(n) => Some(bound(n)?),
                    Upper::Unlimited => None,
                },
            },
            ordered: p.is_ordered,
            unique: p.is_unique,
            derived: p.is_derived,
            composite: p.aggregation == Aggregation::Composite,
            redefines: pids(model, &p.redefines)?,
            subsets: pids(model, &p.subsets)?,
            derived_union: p.is_derived_union,
            association: p.association.as_ref().map(|id| aid(model, id)),
            opposite_ends: pids(model, &p.opposite_ends)?,
        });
    }
    output.primitives.sort_by_key(|d| d.id);
    Ok(output)
}

/// Independent source graph traversal for the golden: no kernel resolver call.
pub fn effective(model: &Metamodel, class: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut ancestors = BTreeSet::new();
    let mut pending = vec![class.to_owned()];
    while let Some(id) = pending.pop() {
        if ancestors.insert(id.clone()) {
            pending.extend(model.classifiers[&id].generalizations.clone());
        }
    }
    let mut properties: BTreeSet<_> = ancestors
        .iter()
        .flat_map(|id| model.classifiers[id].properties.clone())
        .collect();
    let mut removed = BTreeSet::new();
    let mut pending: Vec<_> = properties
        .iter()
        .flat_map(|id| model.properties[id].redefines.clone())
        .collect();
    while let Some(id) = pending.pop() {
        if removed.insert(id.clone()) {
            pending.extend(model.properties[&id].redefines.clone());
        }
    }
    properties.retain(|id| !removed.contains(id));
    (ancestors, properties)
}

pub fn golden(bundle: &Bundle, selected: &Closure) -> Value {
    let model = &bundle.metamodel;
    let mut classifiers = BTreeMap::new();
    for id in &selected.classifiers {
        let c = &model.classifiers[id];
        let (ancestors, properties) = if c.entity.key.kind == Kind::Class {
            effective(model, id)
        } else {
            Default::default()
        };
        classifiers.insert(id, json!({
            "descriptor_id": c.entity.key.uuid().to_string(), "entity": c.entity,
            "package": c.package, "abstract": c.is_abstract, "direct_supertypes": c.generalizations,
            "effective_supertypes_including_self": ancestors, "declared_properties": c.properties,
            "effective_properties": properties, "member_ends": c.member_ends,
            "navigable_owned_ends": c.navigable_owned_ends,
            "literals": c.literals.iter().map(|l| json!({"descriptor_id": l.key.uuid().to_string(), "entity": l})).collect::<Vec<_>>(),
        }));
    }
    let properties: BTreeMap<_, _> = selected
        .properties
        .iter()
        .map(|id| {
            let p = &model.properties[id];
            // Retain complete property evidence, including exact source bounds,
            // default nodes, flags and comments; no regenerated runtime projection.
            (
                id,
                json!({"descriptor_id": p.entity.key.uuid().to_string(), "source": p}),
            )
        })
        .collect();
    json!({
        "_generated": "GENERATED FILE. DO NOT EDIT. cargo run --locked --offline -p agq-metamodel-gen",
        "generator": VERSION, "format": "agentique-root-core-golden/1",
        "inputs": [model.source, bundle.primitive_types.source, bundle.cross_check.source],
        "seeds": SEEDS, "classifiers": classifiers, "properties": properties, "external_types": selected.external_types,
        "scope": "Descriptor structure only. Operations, constraints, defaults and union evaluation are not executed. Association link storage is not implemented.",
    })
}

fn id_code(kind: &str, value: u128) -> String {
    format!("{kind}::from_u128(0x{value:032x})")
}
fn list(values: impl IntoIterator<Item = String>) -> String {
    format!("[{}]", values.into_iter().collect::<Vec<_>>().join(", "))
}
fn strings(values: &[String]) -> String {
    format!(
        "vec!{}",
        list(values.iter().map(|s| format!("{s:?}.into()")))
    )
}
fn property_set(values: &BTreeSet<PropertyId>) -> String {
    format!(
        "BTreeSet::from({})",
        list(values.iter().map(|v| id_code("PropertyId", v.as_u128())))
    )
}

pub fn rust_source(
    bundle: &Bundle,
    selected: &Closure,
    set: &DescriptorSet,
    golden_bytes: &[u8],
) -> String {
    let mut out = format!(
        "// GENERATED FILE. DO NOT EDIT.\n// Generator: {VERSION}\n// Reproduce: cargo run --locked --offline -p agq-metamodel-gen\n// Metamodel XMI SHA-256: {}\n// PrimitiveTypes.xmi SHA-256: {}\n// Metamodel JSON SHA-256: {}\n// Golden SHA-256: {}\n\nuse agq_kernel::{{metamodel::*, *}};\nuse std::collections::{{BTreeMap, BTreeSet}};\n\n#[rustfmt::skip]\npub fn descriptors() -> DescriptorSet {{\n    let mut output = DescriptorSet::default();\n",
        bundle.metamodel.source.sha256,
        bundle.primitive_types.source.sha256,
        bundle.cross_check.source.sha256,
        sha256(golden_bytes)
    );

    for p in &set.primitives {
        out.push_str(&format!("    output.primitives.push(PrimitiveDescriptor {{ id: {}, name: {:?}.into(), representation: PrimitiveRepresentation::{:?} }});\n", id_code("PrimitiveDomainId", p.id.as_u128()), p.name, p.representation));
    }

    for (id, source) in &set.sources {
        out.push_str(&format!(
            "    output.sources.insert({}, {});\n",
            descriptor_id_code(*id),
            source_code(source)
        ));
    }

    for r in &set.reviews {
        out.push_str(&format!("    output.reviews.push(DiagnosticReview {{ rule: MetamodelRule::{:?}, subject: {}, related: {}, source: {}, related_source: {}, evidence: {:?}.into() }});\n", r.rule, descriptor_id_code(r.subject), descriptor_id_code(r.related), source_code(&r.source), source_code(&r.related_source), r.evidence));
    }

    for m in &set.models {
        out.push_str(&format!("    output.models.push(MetamodelDescriptor {{ id: {}, name: {:?}.into(), version: Version {{ major: {}, minor: {}, patch: {} }}, uri: {:?}.into() }});\n", id_code("MetamodelId",m.id.as_u128()),m.name,m.version.major,m.version.minor,m.version.patch,m.uri));
    }

    for c in &set.classes {
        let mm = id_code("MetamodelId", c.metamodel.as_u128());
        out.push_str(&format!("    output.classes.push(MetaclassDescriptor {{ id: {}, name: {:?}.into(), package: {}, metamodel: {mm}, direct_supertypes: BTreeSet::from({}), is_abstract: {} }});\n", id_code("MetaclassId", c.id.as_u128()), c.name, strings(&c.package), list(c.direct_supertypes.iter().map(|v| id_code("MetaclassId", v.as_u128()))), c.is_abstract));
    }

    for p in &set.properties {
        let owner = match p.owner {
            PropertyOwner::Class(id) => format!(
                "PropertyOwner::Class({})",
                id_code("MetaclassId", id.as_u128())
            ),
            PropertyOwner::Association(id) => format!(
                "PropertyOwner::Association({})",
                id_code("AssociationId", id.as_u128())
            ),
        };
        let kind = match p.value_kind {
            ValueKind::Primitive(id) => format!(
                "ValueKind::Primitive({})",
                id_code("PrimitiveDomainId", id.as_u128())
            ),
            ValueKind::Reference(id) => format!(
                "ValueKind::Reference({})",
                id_code("MetaclassId", id.as_u128())
            ),
            ValueKind::Enumeration(id) => format!(
                "ValueKind::Enumeration({})",
                id_code("EnumerationId", id.as_u128())
            ),
            other => format!("ValueKind::{other:?}"),
        };
        let association = p.association.map_or("None".into(), |id| {
            format!("Some({})", id_code("AssociationId", id.as_u128()))
        });
        out.push_str(&format!("    output.properties.push(PropertyDescriptor {{ id: {}, name: {:?}.into(), owner: {owner}, value_kind: {kind}, multiplicity: Multiplicity {{ lower: {}, upper: {:?} }}, ordered: {}, unique: {}, derived: {}, composite: {}, redefines: {}, subsets: {}, derived_union: {}, association: {association}, opposite_ends: {} }});\n", id_code("PropertyId", p.id.as_u128()), p.name, p.multiplicity.lower, p.multiplicity.upper, p.ordered, p.unique, p.derived, p.composite, property_set(&p.redefines), property_set(&p.subsets), p.derived_union, property_set(&p.opposite_ends)));
    }

    for a in &set.associations {
        let mm = id_code("MetamodelId", a.metamodel.as_u128());
        out.push_str(&format!("    output.associations.push(AssociationDescriptor {{ id: {}, name: {:?}.into(), package: {}, metamodel: {mm}, member_ends: vec!{}, navigable_owned_ends: {}, direct_supertypes: BTreeSet::from({}), is_abstract: {} }});\n", id_code("AssociationId", a.id.as_u128()), a.name, strings(&a.package), list(a.member_ends.iter().map(|v| id_code("PropertyId", v.as_u128()))), property_set(&a.navigable_owned_ends), list(a.direct_supertypes.iter().map(|v| id_code("AssociationId", v.as_u128()))), a.is_abstract));
    }

    for e in &set.enumerations {
        let mm = id_code("MetamodelId", e.metamodel.as_u128());
        let literals = list(e.literals.iter().map(|(id, name)| {
            format!(
                "({}, {name:?}.into())",
                id_code("EnumerationLiteralId", id.as_u128())
            )
        }));
        out.push_str(&format!("    output.enumerations.push(EnumerationDescriptor {{ id: {}, name: {:?}.into(), package: {}, metamodel: {mm}, literals: BTreeMap::from({literals}) }});\n", id_code("EnumerationId", e.id.as_u128()), e.name, strings(&e.package)));
    }
    out.push_str(
        "    output\n}\n\n#[rustfmt::skip]\npub const CLASS_IDS: &[(&str, MetaclassId)] = &[\n",
    );
    for id in &selected.classifiers {
        let c = &bundle.metamodel.classifiers[id];
        if c.entity.key.kind == Kind::Class {
            out.push_str(&format!(
                "    ({:?}, {}),\n",
                c.entity.key.external_id,
                id_code("MetaclassId", c.entity.key.uuid().as_u128())
            ));
        }
    }
    out.push_str("];\n\n#[rustfmt::skip]\npub const PROPERTY_IDS: &[(&str, PropertyId)] = &[\n");
    for id in &selected.properties {
        out.push_str(&format!(
            "    ({:?}, {}),\n",
            bundle.metamodel.properties[id].entity.key.external_id,
            id_code(
                "PropertyId",
                bundle.metamodel.properties[id].entity.key.uuid().as_u128()
            )
        ));
    }
    out.push_str("];\n");
    out
}

pub fn artifacts(bundle: &Bundle) -> Result<BTreeMap<&'static str, Vec<u8>>> {
    let selected = closure(&bundle.metamodel, SEEDS)?;
    let set = descriptor_set(bundle, &selected)?;
    let manifest = canonical_json(&golden(bundle, &selected))?;
    let rust = rust_source(bundle, &selected, &set, &manifest).into_bytes();
    let views = crate::typed_views::source(bundle, &selected, &set, &manifest)?.into_bytes();
    Ok(BTreeMap::from([
        (RUST_PATH, rust),
        (GOLDEN_PATH, manifest),
        ("crates/kerml/src/generated/root_core_views.rs", views),
    ]))
}

/// Complete production output. Dependency descriptors are imported at runtime,
/// never emitted a second time in the dependent language crate.
pub fn complete_artifacts(bundle: &Bundle) -> Result<Vec<(String, Vec<u8>)>> {
    let combined = crate::closure_audit::runtime_bundle(bundle)?;
    let selected = crate::full_audit::selection(&combined.metamodel);
    let set = descriptor_set(&combined, &selected)?;
    let manifest = canonical_json(&crate::full_audit::manifest(bundle)?)?;
    let spec = &bundle.metamodel.source.specification;
    let owned = |id: &String| {
        combined.metamodel.classifiers[id]
            .entity
            .key
            .source
            .specification
            == *spec
    };
    let own_selection = Closure {
        classifiers: selected
            .classifiers
            .iter()
            .filter(|id| owned(id))
            .cloned()
            .collect(),
        properties: selected
            .properties
            .iter()
            .filter(|id| {
                combined.metamodel.properties[*id]
                    .entity
                    .key
                    .source
                    .specification
                    == *spec
            })
            .cloned()
            .collect(),
        external_types: selected.external_types.clone(),
    };
    let mut own = set.clone();
    let models: BTreeSet<_> = set
        .models
        .iter()
        .filter(|m| m.name == *spec)
        .map(|m| m.id)
        .collect();
    own.models.retain(|m| models.contains(&m.id));
    own.classes.retain(|c| models.contains(&c.metamodel));
    own.associations.retain(|a| models.contains(&a.metamodel));
    own.enumerations.retain(|e| models.contains(&e.metamodel));
    own.properties
        .retain(|p| set.sources[&DescriptorId::Property(p.id)].specification == *spec);
    own.sources
        .retain(|_, s| s.specification == *spec || spec == "KerML" && s.specification == "UML");
    own.reviews.retain(|r| r.source.specification == *spec);
    if spec != "KerML" {
        own.primitives.clear();
    }
    let directory = if spec == "KerML" { "kerml" } else { "sysml" };
    Ok(vec![
        (
            format!("crates/{directory}/src/generated/complete.rs"),
            rust_source(&combined, &own_selection, &own, &manifest).into_bytes(),
        ),
        (
            format!("crates/{directory}/src/generated/typed_views.rs"),
            crate::typed_views::source(&combined, &selected, &set, &manifest)?.into_bytes(),
        ),
    ])
}
