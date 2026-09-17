//! Explicit supported XMI serialization profile, not a permissive UML loader.
use crate::Result;
use roxmltree::{Document, Node};
use std::collections::BTreeMap;

pub const UML: &str = "http://www.omg.org/spec/UML/20161101";
pub const XMI: &str = "http://www.omg.org/spec/XMI/20161101";
pub const XMI_OLD: &str = "http://www.omg.org/spec/XMI/20131001";
const MOF: &str = "http://www.omg.org/spec/MOF/20161101";
const MOF_OLD: &str = "http://www.omg.org/spec/MOF/20131001";

pub fn xa<'a>(node: Node<'a, '_>, name: &str) -> Option<&'a str> {
    node.attribute((XMI, name))
        .or_else(|| node.attribute((XMI_OLD, name)))
}

pub fn kind<'a>(node: Node<'a, '_>) -> Result<&'a str> {
    let Some(value) = xa(node, "type") else {
        return Ok("");
    };
    let (prefix, local) = value
        .split_once(':')
        .ok_or_else(|| error(node, "xmi:type must be namespace qualified"))?;
    let ns = node.lookup_namespace_uri(Some(prefix));
    if ns == Some(UML) || (local == "Tag" && matches!(ns, Some(MOF | MOF_OLD))) {
        Ok(local)
    } else {
        Err(error(node, "unsupported xmi:type namespace"))
    }
}

pub fn error(node: Node<'_, '_>, message: &str) -> String {
    let pos = node.document().text_pos_at(node.range().start);
    format!(
        "{message} at {}:{} <{}> ({})",
        pos.row,
        pos.col,
        node.tag_name().name(),
        xa(node, "id").unwrap_or("no xmi:id")
    )
}

pub fn required<'a>(node: Node<'a, '_>, name: &str) -> Result<&'a str> {
    node.attribute(name)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| error(node, &format!("missing {name}")))
}

pub fn id<'a>(node: Node<'a, '_>) -> Result<&'a str> {
    xa(node, "id")
        .filter(|s| !s.is_empty())
        .ok_or_else(|| error(node, "missing xmi:id"))
}

pub fn boolean(node: Node<'_, '_>, name: &str, default: bool) -> Result<bool> {
    match node.attribute(name) {
        None => Ok(default),
        Some("true" | "1") => Ok(true),
        Some("false" | "0") => Ok(false),
        Some(_) => Err(error(node, &format!("invalid boolean {name}"))),
    }
}

pub fn children<'a, 'input>(
    node: Node<'a, 'input>,
    tag: &'a str,
) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children()
        .filter(move |n| n.is_element() && n.tag_name().name() == tag)
}

pub fn one<'a, 'input>(node: Node<'a, 'input>, tag: &'a str) -> Result<Option<Node<'a, 'input>>> {
    let mut matches = children(node, tag);
    let first = matches.next();
    if matches.next().is_some() {
        return Err(error(node, &format!("duplicate singleton {tag}")));
    }
    Ok(first)
}

pub fn reference(node: Node<'_, '_>) -> Result<String> {
    if let Some(uri) = node.attribute("href") {
        if xa(node, "idref").is_some() || uri.is_empty() || !uri.contains('#') {
            return Err(error(node, "invalid source-qualified href"));
        }
        return Ok(uri.to_owned());
    }
    xa(node, "idref")
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| error(node, "expected local xmi:idref"))
}

pub fn references(node: Node<'_, '_>, tag: &str) -> Result<Vec<String>> {
    children(node, tag).map(reference).collect()
}

/// Audit the entire tree, including retained operations/constraints. Unknown
/// attributes/elements/types never disappear behind a wildcard or skip handler.
pub fn validate<'a>(doc: &'a Document<'_>) -> Result<BTreeMap<String, Node<'a, 'a>>> {
    let root = doc.root_element();
    let xmi_ns = root.tag_name().namespace();
    if root.tag_name().name() != "XMI" || !matches!(xmi_ns, Some(XMI | XMI_OLD)) {
        return Err(error(root, "expected supported xmi:XMI root"));
    }
    let mut ids = BTreeMap::new();
    for node in doc.descendants().filter(Node::is_element) {
        validate_node(node, xmi_ns)?;
        if let Some(external_id) = xa(node, "id")
            && (external_id.is_empty() || ids.insert(external_id.to_owned(), node).is_some())
        {
            return Err(error(node, "duplicate or empty external identifier"));
        }
    }
    for node in doc.descendants().filter(Node::is_element) {
        for reference in [
            xa(node, "idref"),
            node.attribute("element"),
            node.attribute("annotatedElement"),
        ]
        .into_iter()
        .flatten()
        {
            if !ids.contains_key(reference) {
                return Err(error(node, &format!("unresolved reference {reference}")));
            }
        }
    }
    Ok(ids)
}

fn validate_node(node: Node<'_, '_>, xmi_ns: Option<&str>) -> Result<()> {
    let tag = node.tag_name();
    let ty = kind(node)?;
    let is_ref = xa(node, "idref").is_some();
    let (attrs, child_tags): (&[&str], &[&str]) = match (tag.name(), ty, is_ref) {
        ("XMI", "", false) => (&[], &["Package", "Tag"]),
        ("Package", "" | "Package", false) | ("packagedElement", "Package", false) => (
            &["name", "URI"],
            &["packagedElement", "packageImport", "ownedComment"],
        ),
        ("packagedElement", "Class", false) => (
            &["name", "isAbstract"],
            &[
                "generalization",
                "ownedAttribute",
                "ownedComment",
                "ownedOperation",
                "ownedRule",
            ],
        ),
        ("packagedElement", "Association", false) => (
            &["name", "isAbstract"],
            &[
                "generalization",
                "memberEnd",
                "navigableOwnedEnd",
                "ownedEnd",
                "ownedComment",
            ],
        ),
        ("packagedElement", "Enumeration", false) => (&["name"], &["ownedLiteral", "ownedComment"]),
        ("packagedElement", "PrimitiveType", false) => (&["name"], &["ownedComment"]),
        ("ownedLiteral", "EnumerationLiteral", false) => (&["name"], &["ownedComment"]),
        ("ownedAttribute" | "ownedEnd", "Property", false) => (
            &[
                "name",
                "isDerived",
                "isDerivedUnion",
                "isOrdered",
                "isUnique",
                "isID",
                "isReadOnly",
                "aggregation",
            ],
            &[
                "association",
                "defaultValue",
                "lowerValue",
                "upperValue",
                "ownedComment",
                "redefinedProperty",
                "subsettedProperty",
                "type",
            ],
        ),
        ("packageImport", "PackageImport", false) => (&[], &["importedPackage"]),
        ("generalization", "Generalization", false) => (&[], &["general"]),
        ("lowerValue", "LiteralInteger", false)
        | ("upperValue", "LiteralUnlimitedNatural", false) => (&["name", "value"], &[]),
        ("defaultValue", "LiteralString" | "LiteralBoolean", false) => (&["name", "value"], &[]),
        ("defaultValue", "InstanceValue", false) => (&["name"], &["instance"]),
        ("ownedComment", "Comment", false) => {
            (&["body", "annotatedElement"], &["annotatedElement", "body"])
        }
        ("body", "", false) => (&[], &[]),
        ("ownedRule" | "bodyCondition" | "precondition", "Constraint", false) => (
            &["name"],
            &["constrainedElement", "ownedComment", "specification"],
        ),
        ("specification", "OpaqueExpression", false) => (&["name", "body", "language"], &[]),
        ("ownedOperation", "Operation", false) => (
            &["name", "isAbstract", "isLeaf"],
            &[
                "bodyCondition",
                "ownedComment",
                "ownedParameter",
                "ownedRule",
                "precondition",
                "redefinedOperation",
            ],
        ),
        ("ownedParameter", "Parameter", false) => (
            &["name", "isOrdered", "isUnique", "isStream"],
            &["lowerValue", "upperValue", "type"],
        ),
        ("Tag", "Tag", false) => (&["name", "value", "element"], &[]),
        (
            "type" | "general" | "importedPackage" | "redefinedProperty" | "subsettedProperty"
            | "redefinedOperation" | "instance",
            "",
            false,
        ) if node.attribute("href").is_some() => (&["href"], &[]),
        (
            "importedPackage" | "annotatedElement" | "general" | "association"
            | "subsettedProperty" | "type" | "redefinedProperty" | "ownedRule"
            | "constrainedElement" | "memberEnd" | "navigableOwnedEnd" | "redefinedOperation"
            | "instance",
            "",
            true,
        ) => (&[], &[]),
        _ => {
            return Err(error(
                node,
                &format!("unsupported structural input/type {ty}"),
            ));
        }
    };
    let expected_ns = match tag.name() {
        "XMI" => xmi_ns,
        "Package" => Some(UML),
        "Tag" if xmi_ns == Some(XMI_OLD) => Some(MOF_OLD),
        "Tag" => Some(MOF),
        _ => None,
    };
    if tag.namespace() != expected_ns {
        return Err(error(node, "unsupported element namespace"));
    }
    for attr in node.attributes() {
        let allowed = if attr.namespace() == xmi_ns {
            if is_ref {
                attr.name() == "idref"
            } else {
                matches!(attr.name(), "id" | "type")
            }
        } else {
            attr.namespace().is_none() && attrs.contains(&attr.name())
        };
        if !allowed {
            return Err(error(
                node,
                &format!("unsupported attribute {}", attr.name()),
            ));
        }
        if attr.name().starts_with("is") && attr.namespace().is_none() {
            boolean(node, attr.name(), false)?;
        }
    }
    if !is_ref && node.attribute("href").is_none() && !matches!(tag.name(), "XMI" | "type" | "body")
    {
        id(node)?;
    }
    for child in node.children() {
        if child.is_element() && !child_tags.contains(&child.tag_name().name()) {
            return Err(error(child, "unsupported child in this structural context"));
        }
        if child.is_text() && tag.name() != "body" && !child.text().unwrap_or("").trim().is_empty()
        {
            return Err(error(node, "unexpected text content"));
        }
        if child.is_pi() {
            return Err(error(node, "unsupported processing instruction"));
        }
    }
    Ok(())
}
