use agq_kernel::{metamodel::*, *};
use agq_metamodel_gen::{descriptors::*, ir::*, pipeline};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    process::Command,
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn property_id(m: &Metamodel, id: &str) -> PropertyId {
    m.properties[id].entity.key.property_id().unwrap()
}
fn class_id(m: &Metamodel, id: &str) -> MetaclassId {
    m.classifiers[id].entity.key.metaclass_id().unwrap()
}
fn property_ids(m: &Metamodel, ids: &[String]) -> BTreeSet<PropertyId> {
    ids.iter().map(|id| property_id(m, id)).collect()
}

#[test]
fn every_compiled_descriptor_matches_fresh_authoritative_import() {
    let bundle = pipeline::generate(&root()).unwrap();
    let m = &bundle.metamodel;
    let selection = closure(m, SEEDS).unwrap();
    let actual = agq_kerml::descriptors();
    let r = agq_kerml::registry().unwrap();
    assert_eq!(
        (
            actual.classes.len(),
            actual.properties.len(),
            actual.associations.len(),
            actual.enumerations.len()
        ),
        (29, 196, 80, 2)
    );
    assert_eq!(
        actual
            .properties
            .iter()
            .filter(|p| matches!(p.owner, PropertyOwner::Class(_)))
            .count(),
        146
    );
    assert_eq!(selection.classifiers.len(), 111);
    assert!(
        selection
            .classifiers
            .iter()
            .all(|id| id.starts_with("Root-") || id.starts_with("Core-"))
    );
    assert_eq!(actual.properties.len(), selection.properties.len());
    for id in &selection.classifiers {
        let c = &m.classifiers[id];
        match c.entity.key.kind {
            Kind::Class => {
                let compiled = r.class(class_id(m, id)).unwrap();
                assert_eq!(compiled.id.as_u128(), c.entity.key.uuid().as_u128());
                assert_eq!(compiled.name, c.entity.name);
                assert_eq!(compiled.package, c.entity.key.package_path);
                assert_eq!(compiled.is_abstract, c.is_abstract);
                assert_eq!(
                    compiled.direct_supertypes,
                    c.generalizations.iter().map(|id| class_id(m, id)).collect()
                );
                let (ancestors, properties) = effective(m, id);
                for base in &actual.classes {
                    let expected = ancestors.iter().any(|id| class_id(m, id) == base.id);
                    assert_eq!(r.is_subtype(compiled.id, base.id).unwrap(), expected);
                }
                assert_eq!(
                    r.effective_properties(compiled.id)
                        .unwrap()
                        .map(|p| p.id)
                        .collect::<BTreeSet<_>>(),
                    properties.iter().map(|id| property_id(m, id)).collect()
                );
                assert_eq!(
                    r.declared_properties(compiled.id)
                        .unwrap()
                        .map(|p| p.id)
                        .collect::<BTreeSet<_>>(),
                    property_ids(m, &c.properties)
                );
            }
            Kind::Association => {
                let a = r
                    .association(AssociationId::from_u128(c.entity.key.uuid().as_u128()))
                    .unwrap();
                assert_eq!(a.name, c.entity.name);
                assert_eq!(a.package, c.entity.key.package_path);
                assert_eq!(
                    a.member_ends,
                    c.member_ends
                        .iter()
                        .map(|id| property_id(m, id))
                        .collect::<Vec<_>>()
                );
                assert_eq!(
                    a.navigable_owned_ends,
                    property_ids(m, &c.navigable_owned_ends)
                );
            }
            Kind::Enumeration => {
                let e = r
                    .enumeration(EnumerationId::from_u128(c.entity.key.uuid().as_u128()))
                    .unwrap();
                assert_eq!(e.name, c.entity.name);
                assert_eq!(e.package, c.entity.key.package_path);
                assert_eq!(
                    e.literals,
                    c.literals
                        .iter()
                        .map(|l| (
                            EnumerationLiteralId::from_u128(l.key.uuid().as_u128()),
                            l.name.clone()
                        ))
                        .collect()
                );
            }
            _ => panic!("unexpected classifier"),
        }
    }
    for id in &selection.properties {
        let p = &m.properties[id];
        let compiled = r.property(property_id(m, id)).unwrap();
        assert_eq!(compiled.id.as_u128(), p.entity.key.uuid().as_u128());
        assert_eq!(compiled.name, p.entity.name);
        let owner = &m.classifiers[&p.owner].entity.key;
        assert_eq!(
            compiled.owner,
            match owner.kind {
                Kind::Class => PropertyOwner::Class(owner.metaclass_id().unwrap()),
                Kind::Association =>
                    PropertyOwner::Association(AssociationId::from_u128(owner.uuid().as_u128())),
                _ => panic!("unexpected owner"),
            }
        );
        assert_eq!(
            compiled.value_kind,
            match &p.type_ref {
                TypeRef::Local(id) => {
                    let key = &m.classifiers[id].entity.key;
                    match key.kind {
                        Kind::Class => ValueKind::Reference(key.metaclass_id().unwrap()),
                        Kind::Enumeration => {
                            ValueKind::Enumeration(EnumerationId::from_u128(key.uuid().as_u128()))
                        }
                        _ => panic!("unexpected domain"),
                    }
                }
                TypeRef::External(uri)
                    if uri
                        == "https://www.omg.org/spec/UML/20161101/PrimitiveTypes.xmi#Boolean" =>
                    ValueKind::Boolean,
                TypeRef::External(uri)
                    if uri == "https://www.omg.org/spec/UML/20161101/PrimitiveTypes.xmi#String" =>
                    ValueKind::String,
                _ => panic!("unexpected primitive"),
            }
        );
        assert_eq!(compiled.multiplicity.lower as u64, p.lower);
        assert_eq!(
            compiled.multiplicity.upper.map(|n| n as u64),
            match p.upper {
                Upper::Finite(n) => Some(n),
                Upper::Unlimited => None,
            }
        );
        assert_eq!(compiled.ordered, p.is_ordered);
        assert_eq!(compiled.unique, p.is_unique);
        assert_eq!(compiled.derived, p.is_derived);
        assert_eq!(compiled.composite, p.aggregation == Aggregation::Composite);
        assert_eq!(compiled.derived_union, p.is_derived_union);
        assert_eq!(compiled.redefines, property_ids(m, &p.redefines));
        assert_eq!(compiled.subsets, property_ids(m, &p.subsets));
        assert_eq!(compiled.opposite_ends, property_ids(m, &p.opposite_ends));
        assert_eq!(
            compiled.association,
            p.association
                .as_ref()
                .map(|id| AssociationId::from_u128(m.classifiers[id].entity.key.uuid().as_u128()))
        );
    }
}

#[test]
fn source_golden_matches_raw_xmi_flags_bounds_and_references() {
    // A second, deliberately small reader checks fields directly against XML;
    // it does not reuse the importer default/bound/reference functions.
    let text = fs::read_to_string(root().join("standards/normative/kerml-1.0/KerML.xmi")).unwrap();
    let doc = roxmltree::Document::parse(&text).unwrap();
    let ns = "http://www.omg.org/spec/XMI/20161101";
    let nodes: BTreeMap<_, _> = doc
        .descendants()
        .filter_map(|n| n.attribute((ns, "id")).map(|id| (id, n)))
        .collect();
    let g: serde_json::Value =
        serde_json::from_slice(&fs::read(root().join(GOLDEN_PATH)).unwrap()).unwrap();
    for (id, value) in g["properties"].as_object().unwrap() {
        let p = &value["source"];
        let n = nodes[id.as_str()];
        assert_eq!(p["entity"]["name"], n.attribute("name").unwrap_or(""));
        assert_eq!(
            p["owner"],
            n.parent_element().unwrap().attribute((ns, "id")).unwrap()
        );
        for (field, attr, default) in [
            ("is_derived", "isDerived", false),
            ("is_derived_union", "isDerivedUnion", false),
            ("is_ordered", "isOrdered", false),
            ("is_unique", "isUnique", true),
            ("is_read_only", "isReadOnly", false),
            ("is_id", "isID", false),
        ] {
            assert_eq!(
                p[field],
                n.attribute(attr).map_or(default, |v| v == "true"),
                "{id}/{field}"
            );
        }
        assert_eq!(
            p["aggregation"],
            n.attribute("aggregation").unwrap_or("none")
        );
        for (field, tag) in [
            ("redefines", "redefinedProperty"),
            ("subsets", "subsettedProperty"),
        ] {
            let refs: Vec<_> = n
                .children()
                .filter(|c| c.has_tag_name(tag))
                .map(|c| c.attribute((ns, "idref")).unwrap())
                .collect();
            assert_eq!(p[field], serde_json::json!(refs), "{id}/{field}");
        }
        let lower = n
            .children()
            .find(|c| c.has_tag_name("lowerValue"))
            .map_or(1, |c| {
                c.attribute("value").unwrap_or("0").parse::<u64>().unwrap()
            });
        let upper = n
            .children()
            .find(|c| c.has_tag_name("upperValue"))
            .map_or("1", |c| c.attribute("value").unwrap_or("0"));
        assert_eq!(p["lower"], lower);
        assert_eq!(
            p["upper"],
            if upper == "-1" {
                serde_json::json!({"kind":"unlimited"})
            } else {
                serde_json::json!({"kind":"finite", "value": upper.parse::<u64>().unwrap()})
            }
        );
        let target = n.children().find(|c| c.has_tag_name("type")).unwrap();
        assert_eq!(
            p["type_ref"]["target"],
            target
                .attribute((ns, "idref"))
                .or_else(|| target.attribute("href"))
                .unwrap()
        );
        let association = n
            .children()
            .find(|c| c.has_tag_name("association"))
            .map(|c| c.attribute((ns, "idref")).unwrap());
        assert_eq!(p["association"], serde_json::json!(association));
        if let Some(a) = association {
            let ends: Vec<_> = nodes[a]
                .children()
                .filter(|c| c.has_tag_name("memberEnd"))
                .map(|c| c.attribute((ns, "idref")).unwrap())
                .filter(|end| *end != id)
                .collect();
            assert_eq!(p["opposite_ends"], serde_json::json!(ends));
        }
    }
    for (id, value) in g["classifiers"].as_object().unwrap() {
        let n = nodes[id.as_str()];
        assert_eq!(
            value["abstract"],
            n.attribute("isAbstract").is_some_and(|v| v == "true")
        );
        assert_eq!(value["entity"]["name"], n.attribute("name").unwrap());
        assert_eq!(
            value["package"],
            n.parent_element().unwrap().attribute((ns, "id")).unwrap()
        );
        let supers: Vec<_> = n
            .children()
            .filter(|c| c.has_tag_name("generalization"))
            .flat_map(|c| c.children().filter(|c| c.has_tag_name("general")))
            .map(|c| c.attribute((ns, "idref")).unwrap())
            .collect();
        assert_eq!(value["direct_supertypes"], serde_json::json!(supers));
    }
}

#[test]
fn identity_is_source_qualified_and_independent_of_display_names() {
    let mut bundle = pipeline::generate(&root()).unwrap();
    let key = bundle.metamodel.classifiers["Root-Elements-Element"]
        .entity
        .key
        .clone();
    let property = bundle.metamodel.properties["Root-Elements-Element-name"]
        .entity
        .key
        .clone();
    bundle
        .metamodel
        .classifiers
        .get_mut("Root-Elements-Element")
        .unwrap()
        .entity
        .name = "Changed label".into();
    bundle
        .metamodel
        .properties
        .get_mut("Root-Elements-Element-name")
        .unwrap()
        .entity
        .name = "Changed property label".into();
    let selection = closure(&bundle.metamodel, SEEDS).unwrap();
    let set = descriptor_set(&bundle, &selection).unwrap();
    assert!(
        set.classes
            .iter()
            .any(|c| c.id == key.metaclass_id().unwrap() && c.name == "Changed label")
    );
    assert!(
        set.properties
            .iter()
            .any(|p| p.id == property.property_id().unwrap() && p.name == "Changed property label")
    );
    for change in ["artifact", "external_id", "package", "version"] {
        let mut other = key.clone();
        match change {
            "artifact" => other.source.sha256 = "0".repeat(64),
            "external_id" => other.external_id.push_str("-other"),
            "package" => other.package_path.push("other".into()),
            _ => other.source.version = "different".into(),
        }
        assert_ne!(key.uuid(), other.uuid());
    }
}

#[test]
fn generation_is_deterministic_dependency_closed_and_current() {
    let bundle = pipeline::generate(&root()).unwrap();
    let first = artifacts(&bundle).unwrap();
    assert_eq!(
        first,
        artifacts(&pipeline::generate(&root()).unwrap()).unwrap()
    );
    let mut seeds = SEEDS.to_vec();
    seeds.reverse();
    seeds.push(SEEDS[0]);
    assert_eq!(
        closure(&bundle.metamodel, SEEDS).unwrap(),
        closure(&bundle.metamodel, &seeds).unwrap()
    );
    for (path, bytes) in &first {
        assert_eq!(
            *bytes,
            fs::read(root().join(path)).unwrap(),
            "{path} is stale"
        );
    }
    let selection = closure(&bundle.metamodel, SEEDS).unwrap();
    let model = &bundle.metamodel;
    for id in &selection.classifiers {
        let c = &model.classifiers[id];
        assert!(
            c.generalizations
                .iter()
                .all(|id| selection.classifiers.contains(id))
        );
        assert!(
            c.properties
                .iter()
                .chain(&c.member_ends)
                .all(|id| selection.properties.contains(id))
        );
    }
    for id in &selection.properties {
        let p = &model.properties[id];
        assert!(selection.classifiers.contains(&p.owner));
        assert!(
            p.redefines
                .iter()
                .chain(&p.subsets)
                .chain(&p.opposite_ends)
                .all(|id| selection.properties.contains(id))
        );
    }
}

#[test]
fn cli_check_detects_each_stale_generated_file_without_writing() {
    let binary = env!("CARGO_BIN_EXE_metamodel-gen");
    assert!(
        Command::new(binary)
            .arg("--check")
            .output()
            .unwrap()
            .status
            .success()
    );
    let temp = tempfile::tempdir().unwrap();
    let run = |check: bool| {
        let mut c = Command::new(binary);
        c.arg("--output")
            .arg(temp.path().join("ir.json"))
            .arg("--descriptor-output")
            .arg(temp.path());
        if check {
            c.arg("--check");
        }
        c.output().unwrap()
    };
    assert!(run(false).status.success());
    assert!(run(true).status.success());
    for file in [RUST_PATH, GOLDEN_PATH] {
        let path = temp.path().join(file);
        let original = fs::read(&path).unwrap();
        fs::write(&path, b"stale\n").unwrap();
        let result = run(true);
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains("stale"));
        assert_eq!(fs::read(&path).unwrap(), b"stale\n");
        fs::write(&path, original).unwrap();
    }
    assert!(run(true).status.success());
}
