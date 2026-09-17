use crate::{Result, ir::*, profile::*, sha256};
use roxmltree::{Document, Node};
use std::collections::{BTreeMap, BTreeSet};

pub type ExternalTypes = BTreeMap<String, DescriptorKey>;

pub fn external_types(model: &Metamodel) -> ExternalTypes {
    let mut entries = ExternalTypes::new();
    let mut add = |key: DescriptorKey| {
        entries.insert(
            format!("{}#{}", key.source.artifact_uri, key.external_id),
            key,
        );
    };
    for package in model.packages.values() {
        add(package.entity.key.clone());
    }
    for property in model.properties.values() {
        add(property.entity.key.clone());
    }
    for class in model.classifiers.values() {
        add(class.entity.key.clone());
        for literal in &class.literals {
            add(literal.key.clone());
        }
        for operation in class.retained.iter().filter(|n| n.tag == "ownedOperation") {
            let id = operation
                .attributes
                .get(&format!("{{{XMI}}}id"))
                .or_else(|| operation.attributes.get(&format!("{{{XMI_OLD}}}id")));
            if let Some(id) = id {
                let mut key = class.entity.key.clone();
                key.external_id = id.clone();
                key.kind = Kind::Operation;
                add(key);
            }
        }
    }
    entries
}

pub fn import(xml: &str, source: Source, external: &ExternalTypes) -> Result<Metamodel> {
    let root_uri = source.metamodel_uri.clone();
    import_with_root_uri(xml, source, external, &root_uri)
}

/// The serialization root URI is explicit baseline evidence, separate from the
/// canonical publication namespace. No heuristic URL repair is performed.
pub fn import_with_root_uri(
    xml: &str,
    source: Source,
    external: &ExternalTypes,
    root_uri: &str,
) -> Result<Metamodel> {
    if sha256(xml.as_bytes()) != source.sha256 {
        return Err("source provenance SHA-256 does not match input".into());
    }
    // roxmltree rejects DTDs by default and never fetches external entities.
    let doc = Document::parse(xml).map_err(|e| format!("malformed XML: {e}"))?;
    let ids = validate(&doc)?;
    let root = doc.root_element();
    let package = one(root, "Package")?.ok_or("missing root package")?;
    if package.attribute("URI") != Some(root_uri) {
        return Err(error(package, "root metamodel URI differs from provenance"));
    }
    let mut model = Metamodel {
        format: "agentique-metamodel-ir/1".into(),
        source,
        packages: BTreeMap::new(),
        classifiers: BTreeMap::new(),
        properties: BTreeMap::new(),
        retained: children(root, "Tag").map(raw).collect(),
    };
    parse_package(package, None, &[], &mut model)?;
    validate_links(&mut model, external)?;
    // Validate typed reference targets even inside opaque operations/constraints.
    for node in ids
        .values()
        .flat_map(|n| n.children())
        .filter(Node::is_element)
    {
        if node.tag_name().name() == "type" {
            validate_type(&type_ref(node)?, &model, external)?;
        }
        if let Some(uri) = node.attribute("href") {
            let expected = match node.tag_name().name() {
                "type" => continue, // Domain validated above.
                "general" => Kind::Class,
                "importedPackage" => Kind::Package,
                "redefinedProperty" | "subsettedProperty" => Kind::Property,
                "redefinedOperation" => Kind::Operation,
                "instance" => Kind::EnumerationLiteral,
                _ => return Err(error(node, "unsupported external reference role")),
            };
            if !external.get(uri).is_some_and(|key| key.kind == expected) {
                return Err(error(
                    node,
                    &format!("unresolved/wrong-kind external reference {uri}"),
                ));
            }
        }
    }
    Ok(model)
}

fn expanded(namespace: Option<&str>, name: &str) -> String {
    namespace.map_or_else(|| name.to_owned(), |ns| format!("{{{ns}}}{name}"))
}

fn attributes(node: Node<'_, '_>) -> BTreeMap<String, String> {
    node.attributes()
        .map(|a| (expanded(a.namespace(), a.name()), a.value().to_owned()))
        .collect()
}

fn raw(node: Node<'_, '_>) -> SourceNode {
    SourceNode {
        tag: expanded(node.tag_name().namespace(), node.tag_name().name()),
        attributes: attributes(node),
        text: (node.tag_name().name() == "body").then(|| {
            node.children()
                .filter(Node::is_text)
                .filter_map(|n| n.text())
                .collect()
        }),
        children: node.children().filter(Node::is_element).map(raw).collect(),
    }
}

fn entity(
    node: Node<'_, '_>,
    kind: Kind,
    package_path: &[String],
    parent_path: &[String],
    source: &Source,
) -> Result<Entity> {
    let name = if kind == Kind::Property {
        node.attribute("name").unwrap_or("")
    } else {
        required(node, "name")?
    }
    .to_owned();
    let mut qualified_path = parent_path.to_vec();
    qualified_path.push(name.clone());
    Ok(Entity {
        key: DescriptorKey {
            source: source.clone(),
            package_path: package_path.to_vec(),
            external_id: id(node)?.into(),
            kind,
        },
        name,
        qualified_path,
        source_range: [node.range().start, node.range().end],
        source_attributes: attributes(node),
    })
}

fn parse_package(
    node: Node<'_, '_>,
    parent: Option<String>,
    parent_path: &[String],
    model: &mut Metamodel,
) -> Result<()> {
    let mut e = entity(node, Kind::Package, parent_path, parent_path, &model.source)?;
    e.key.package_path = e.qualified_path.clone();
    let package_id = e.key.external_id.clone();
    let path = e.qualified_path.clone();
    let mut imports = Vec::new();
    for import in children(node, "packageImport") {
        let target = one(import, "importedPackage")?
            .ok_or_else(|| error(import, "missing importedPackage"))?;
        imports.push(reference(target)?);
    }
    model.packages.insert(
        package_id.clone(),
        Package {
            entity: e,
            parent,
            uri: node.attribute("URI").map(str::to_owned),
            imports,
            retained: node
                .children()
                .filter(|n| n.is_element() && n.tag_name().name() != "packagedElement")
                .map(raw)
                .collect(),
        },
    );
    for child in children(node, "packagedElement") {
        if kind(child)? == "Package" {
            parse_package(child, Some(package_id.clone()), &path, model)?;
        } else {
            parse_classifier(child, &package_id, &path, model)?;
        }
    }
    Ok(())
}

fn parse_classifier(
    node: Node<'_, '_>,
    package: &str,
    path: &[String],
    model: &mut Metamodel,
) -> Result<()> {
    let classifier_kind = match kind(node)? {
        "Class" => Kind::Class,
        "Association" => Kind::Association,
        "Enumeration" => Kind::Enumeration,
        "PrimitiveType" => Kind::PrimitiveType,
        _ => return Err(error(node, "unsupported classifier")),
    };
    let e = entity(node, classifier_kind, path, path, &model.source)?;
    let owner = e.key.external_id.clone();
    let mut generalizations = Vec::new();
    for generalization in children(node, "generalization") {
        let general = one(generalization, "general")?
            .ok_or_else(|| error(generalization, "missing general"))?;
        generalizations.push(reference(general)?);
    }
    let mut properties = Vec::new();
    for property in node
        .children()
        .filter(|n| n.is_element() && matches!(n.tag_name().name(), "ownedAttribute" | "ownedEnd"))
    {
        let p = parse_property(property, &owner, path, &e.qualified_path, &model.source)?;
        properties.push(p.entity.key.external_id.clone());
        model.properties.insert(p.entity.key.external_id.clone(), p);
    }
    let literals = children(node, "ownedLiteral")
        .map(|n| {
            entity(
                n,
                Kind::EnumerationLiteral,
                path,
                &e.qualified_path,
                &model.source,
            )
        })
        .collect::<Result<_>>()?;
    model.classifiers.insert(
        owner,
        Classifier {
            entity: e,
            package: package.into(),
            is_abstract: boolean(node, "isAbstract", false)?,
            generalizations,
            properties,
            literals,
            member_ends: references(node, "memberEnd")?,
            navigable_owned_ends: references(node, "navigableOwnedEnd")?,
            retained: node
                .children()
                .filter(|n| {
                    n.is_element() && !matches!(n.tag_name().name(), "ownedAttribute" | "ownedEnd")
                })
                .map(raw)
                .collect(),
        },
    );
    Ok(())
}

fn type_ref(node: Node<'_, '_>) -> Result<TypeRef> {
    if let Some(href) = node.attribute("href") {
        Ok(TypeRef::External(href.into()))
    } else {
        Ok(TypeRef::Local(reference(node)?))
    }
}

fn parse_property(
    node: Node<'_, '_>,
    owner: &str,
    package_path: &[String],
    parent_path: &[String],
    source: &Source,
) -> Result<Property> {
    let type_node = one(node, "type")?.ok_or_else(|| error(node, "missing property type"))?;
    // MultiplicityElement defaults to 1..1; a present LiteralInteger with no
    // value is 0, NOT 1. UnlimitedNatural uses -1 in the published XMI.
    let lower = match one(node, "lowerValue")? {
        None => 1,
        Some(n) => n
            .attribute("value")
            .unwrap_or("0")
            .parse::<u64>()
            .map_err(|_| error(n, "invalid lower bound"))?,
    };
    let upper = match one(node, "upperValue")? {
        None => Upper::Finite(1),
        Some(n) => match n.attribute("value").unwrap_or("0") {
            "-1" | "*" => Upper::Unlimited,
            value => Upper::Finite(value.parse().map_err(|_| error(n, "invalid upper bound"))?),
        },
    };
    if matches!(upper, Upper::Finite(u) if u < lower) {
        return Err(error(node, "lower bound exceeds upper bound"));
    }
    let aggregation = match node.attribute("aggregation").unwrap_or("none") {
        "none" => Aggregation::None,
        "shared" => Aggregation::Shared,
        "composite" => Aggregation::Composite,
        _ => return Err(error(node, "unsupported aggregation")),
    };
    Ok(Property {
        entity: entity(node, Kind::Property, package_path, parent_path, source)?,
        owner: owner.into(),
        type_ref: type_ref(type_node)?,
        lower,
        upper,
        is_ordered: boolean(node, "isOrdered", false)?,
        is_unique: boolean(node, "isUnique", true)?,
        is_derived: boolean(node, "isDerived", false)?,
        is_derived_union: boolean(node, "isDerivedUnion", false)?,
        is_read_only: boolean(node, "isReadOnly", false)?,
        is_id: boolean(node, "isID", false)?,
        aggregation,
        redefines: references(node, "redefinedProperty")?,
        subsets: references(node, "subsettedProperty")?,
        association: one(node, "association")?.map(reference).transpose()?,
        opposite_ends: Vec::new(),
        default_value: one(node, "defaultValue")?.map(raw),
        retained: node
            .children()
            .filter(|n| n.is_element() && n.tag_name().name() != "defaultValue")
            .map(raw)
            .collect(),
    })
}

fn validate_type(target: &TypeRef, model: &Metamodel, external: &ExternalTypes) -> Result<()> {
    match target {
        TypeRef::Local(id)
            if model
                .classifiers
                .get(id)
                .is_some_and(|c| c.entity.key.kind != Kind::Association) =>
        {
            Ok(())
        }
        TypeRef::External(uri)
            if external.get(uri).is_some_and(|key| {
                matches!(
                    key.kind,
                    Kind::Class | Kind::Enumeration | Kind::PrimitiveType
                )
            }) =>
        {
            Ok(())
        }
        _ => Err(format!(
            "unresolved or unsupported property/parameter type {target:?}"
        )),
    }
}

fn validate_links(model: &mut Metamodel, external: &ExternalTypes) -> Result<()> {
    for package in model.packages.values() {
        for target in &package.imports {
            if !model.packages.contains_key(target)
                && !external
                    .get(target)
                    .is_some_and(|k| k.kind == Kind::Package)
            {
                return Err(format!("unresolved imported package {target}"));
            }
        }
    }
    for (id, classifier) in &model.classifiers {
        let mut unique = BTreeSet::new();
        for target in &classifier.generalizations {
            let kind = model
                .classifiers
                .get(target)
                .map(|c| &c.entity.key.kind)
                .or_else(|| external.get(target).map(|k| &k.kind))
                .ok_or_else(|| format!("unresolved referenced metaclass {target} on {id}"))?;
            if *kind != classifier.entity.key.kind || !unique.insert(target) {
                return Err(format!("invalid/duplicate generalization {id} -> {target}"));
            }
        }
        let mut names = BTreeSet::new();
        for property in &classifier.properties {
            let name = &model.properties[property].entity.name;
            if !name.is_empty() && !names.insert(name) {
                return Err(format!("duplicate owned property name on {id}"));
            }
        }
        if classifier.entity.key.kind == Kind::Association {
            let ends: BTreeSet<_> = classifier.member_ends.iter().collect();
            if ends.len() < 2 || ends.len() != classifier.member_ends.len() {
                return Err(format!("invalid association ends on {id}"));
            }
            for target in &classifier.member_ends {
                let p = model
                    .properties
                    .get(target)
                    .ok_or_else(|| format!("unresolved association end {target}"))?;
                if p.association.as_deref() != Some(id) {
                    return Err(format!("association/end mismatch {id} / {target}"));
                }
            }
            for target in &classifier.navigable_owned_ends {
                if !ends.contains(target) || !classifier.properties.contains(target) {
                    return Err(format!("invalid navigable owned end {target}"));
                }
            }
        }
    }
    for (id, property) in &model.properties {
        validate_type(&property.type_ref, model, external)?;
        for target in property.redefines.iter().chain(&property.subsets) {
            if !model.properties.contains_key(target)
                && !external
                    .get(target)
                    .is_some_and(|k| k.kind == Kind::Property)
            {
                return Err(format!("unresolved property reference {id} -> {target}"));
            }
        }
        if let Some(association) = &property.association {
            let a = model
                .classifiers
                .get(association)
                .ok_or_else(|| format!("unresolved association {association}"))?;
            if a.entity.key.kind != Kind::Association || !a.member_ends.contains(id) {
                return Err(format!("invalid association membership {id}"));
            }
        }
    }
    // A cycle is structural invalidity; inherited property/redefinition semantics
    // are deliberately not computed here.
    fn visit(
        id: &str,
        model: &Metamodel,
        active: &mut BTreeSet<String>,
        done: &mut BTreeSet<String>,
    ) -> Result<()> {
        if done.contains(id) {
            return Ok(());
        }
        if !model.classifiers.contains_key(id) {
            return Ok(());
        } // Previously validated pinned dependency.
        if !active.insert(id.into()) {
            return Err(format!("generalization cycle at {id}"));
        }
        for target in &model.classifiers[id].generalizations {
            visit(target, model, active, done)?;
        }
        active.remove(id);
        done.insert(id.into());
        Ok(())
    }
    let mut done = BTreeSet::new();
    for id in model.classifiers.keys() {
        visit(id, model, &mut BTreeSet::new(), &mut done)?;
    }
    for (id, p) in &mut model.properties {
        if let Some(a) = &p.association {
            p.opposite_ends = model.classifiers[a]
                .member_ends
                .iter()
                .filter(|end| *end != id)
                .cloned()
                .collect();
        }
    }
    Ok(())
}
