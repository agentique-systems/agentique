//! Prints a tree as canonical SysML text: four-space indentation, one member
//! per line, a blank line between package members except consecutive imports.
//! `Unsupported` and `SyntaxError` elements are printed exactly as they were read.

use crate::parser::Source;
use crate::tree::*;
use std::fmt::Write;

/// Prints every document of the tree.
pub fn print(tree: &Tree) -> Vec<Source> {
    tree.documents
        .iter()
        .map(|document| {
            let mut out = String::new();
            print_members(tree, &document.members, 0, true, &mut out);
            Source::new(document.path.clone(), out)
        })
        .collect()
}

/// Prints one element (and its children) at indentation level 0.
pub fn print_element(tree: &Tree, id: ElementId) -> String {
    let mut out = String::new();
    print_member(tree, id, 0, &mut out);
    out
}

fn print_members(tree: &Tree, members: &[ElementId], level: usize, spaced: bool, out: &mut String) {
    let mut previous: Option<ElementKind> = None;
    for &id in members {
        let kind = tree[id].kind;
        let both_imports = previous == Some(ElementKind::Import) && kind == ElementKind::Import;
        if spaced && previous.is_some() && !both_imports {
            out.push('\n');
        }
        print_member(tree, id, level, out);
        previous = Some(kind);
    }
}

fn print_member(tree: &Tree, id: ElementId, level: usize, out: &mut String) {
    let e = &tree[id];
    let indent = "    ".repeat(level);
    out.push_str(&indent);
    if let ElementKind::Unsupported | ElementKind::SyntaxError = e.kind {
        out.push_str(e.text.as_deref().unwrap_or(""));
        out.push('\n');
        return;
    }
    if e.kind == ElementKind::Doc {
        print_doc(e.text.as_deref().unwrap_or(""), &indent, out);
        return;
    }
    match e.visibility {
        Visibility::Public => {}
        Visibility::Private => out.push_str("private "),
        Visibility::Protected => out.push_str("protected "),
    }
    out.push_str(&header(e));
    if e.children.is_empty() {
        out.push_str(";\n");
    } else {
        out.push_str(" {\n");
        print_members(
            tree,
            &e.children,
            level + 1,
            e.kind == ElementKind::Package,
            out,
        );
        out.push_str(&indent);
        out.push_str("}\n");
    }
}

/// The declaration text before the body, e.g. `part store : SqlLinkStore[1]`.
fn header(e: &Element) -> String {
    let mut h = String::new();
    match e.kind {
        ElementKind::Import => {
            let target = e.target.clone().unwrap_or_default();
            let _ = write!(h, "import {target}{}", if e.wildcard { "::*" } else { "" });
            return h;
        }
        ElementKind::Satisfy => {
            let _ = write!(h, "satisfy {}", e.target.clone().unwrap_or_default());
            if let Some(by) = &e.by {
                let _ = write!(h, " by {by}");
            }
            return h;
        }
        ElementKind::Connection if is_bare_connect(e) => {
            let _ = write!(h, "connect {} to {}", e.ends[0], e.ends[1]);
            return h;
        }
        _ => {}
    }
    if e.is_abstract {
        h.push_str("abstract ");
    }
    if let Some(direction) = e.direction {
        h.push_str(direction.keyword());
        h.push(' ');
    }
    if e.is_end {
        h.push_str("end ");
    }
    h.push_str(e.kind.keyword());
    if let Some(name) = &e.name {
        h.push(' ');
        let _ = write_name(&mut h, name);
    }
    let list = |names: &[QualifiedName]| {
        names
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    };
    if !e.typed_by.is_empty() {
        let types: Vec<String> = e.typed_by.iter().map(ToString::to_string).collect();
        let _ = write!(h, " : {}", types.join(", "));
    }
    if !e.specializes.is_empty() {
        let _ = write!(h, " :> {}", list(&e.specializes));
    }
    if !e.redefines.is_empty() {
        let _ = write!(h, " :>> {}", list(&e.redefines));
    }
    if let Some(multiplicity) = e.multiplicity {
        if e.name.is_none()
            && e.typed_by.is_empty()
            && e.specializes.is_empty()
            && e.redefines.is_empty()
        {
            h.push(' ');
        }
        let _ = write!(h, "{multiplicity}");
    }
    if let Some(value) = &e.value {
        let _ = write!(h, " = {value}");
    }
    if e.ends.len() == 2 {
        let _ = write!(h, " connect {} to {}", e.ends[0], e.ends[1]);
    }
    h
}

/// `connect a to b;` with nothing else to say.
fn is_bare_connect(e: &Element) -> bool {
    e.ends.len() == 2
        && e.name.is_none()
        && e.typed_by.is_empty()
        && e.specializes.is_empty()
        && e.redefines.is_empty()
        && e.multiplicity.is_none()
        && !e.is_abstract
        && e.direction.is_none()
}

fn print_doc(text: &str, indent: &str, out: &mut String) {
    let mut lines = text.lines();
    let first = lines.next().unwrap_or("");
    let rest: Vec<&str> = lines.collect();
    if rest.is_empty() {
        let _ = writeln!(out, "doc /* {first} */");
        return;
    }
    let _ = writeln!(out, "doc /* {first}");
    for line in rest {
        let line = format!("{indent}     * {line}");
        out.push_str(line.trim_end());
        out.push('\n');
    }
    let _ = writeln!(out, "{indent}     */");
}
