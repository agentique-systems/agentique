//! Prints a tree as canonical SysML text: four-space indentation, one member
//! per line, a blank line between package members except consecutive imports.
//! `Unsupported` and `SyntaxError` elements are printed exactly as they were read.
//!
//! A linked reference is printed with a name that resolves back to its
//! target from where it is written, so renames and moves carry through.

use crate::library::library;
use crate::parser::Source;
use crate::resolve::Model;
use crate::tree::*;
use std::fmt::Write;

/// Prints every document of the tree.
pub fn print(tree: &Tree) -> Vec<Source> {
    let printer = Printer {
        model: Model::new(tree, library()),
    };
    tree.documents()
        .iter()
        .map(|document| {
            let mut out = String::new();
            printer.members(document.members(), 0, true, &mut out);
            Source::new(document.path.clone(), out)
        })
        .collect()
}

/// Prints one element (and its children) at indentation level 0, or `None`
/// if the tree has no such element.
pub fn print_element(tree: &Tree, id: ElementId) -> Option<String> {
    tree.get(id)?;
    let printer = Printer {
        model: Model::new(tree, library()),
    };
    let mut out = String::new();
    printer.member(id, 0, &mut out);
    Some(out)
}

struct Printer<'a> {
    model: Model<'a>,
}

impl Printer<'_> {
    fn members(&self, members: &[ElementId], level: usize, spaced: bool, out: &mut String) {
        let mut previous: Option<ElementKind> = None;
        for &id in members {
            let kind = self.model.tree[id].kind;
            let both_imports = previous == Some(ElementKind::Import) && kind == ElementKind::Import;
            if spaced && previous.is_some() && !both_imports {
                out.push('\n');
            }
            self.member(id, level, out);
            previous = Some(kind);
        }
    }

    fn member(&self, id: ElementId, level: usize, out: &mut String) {
        let e = &self.model.tree[id];
        let indent = "    ".repeat(level);
        out.push_str(&indent);
        match e.kind {
            ElementKind::Unsupported | ElementKind::SyntaxError => {
                out.push_str(e.text.as_deref().unwrap_or(""));
                out.push('\n');
                return;
            }
            ElementKind::Doc => return print_comment("doc ", e.text.as_deref(), &indent, out),
            ElementKind::Comment => return print_comment("", e.text.as_deref(), &indent, out),
            _ => {}
        }
        match e.visibility {
            Visibility::Public => {}
            Visibility::Private => out.push_str("private "),
            Visibility::Protected => out.push_str("protected "),
        }
        out.push_str(&self.header(id));
        if e.children().is_empty() {
            out.push_str(";\n");
        } else {
            out.push_str(" {\n");
            self.members(e.children(), level + 1, e.kind == ElementKind::Package, out);
            out.push_str(&indent);
            out.push_str("}\n");
        }
    }

    fn reference(&self, id: ElementId, role: Role, reference: &Reference) -> String {
        self.model.name_for(id, role, reference).0
    }

    fn list(&self, id: ElementId, role: Role, references: &[Reference]) -> String {
        references
            .iter()
            .map(|r| self.reference(id, role, r))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The declaration text before the body, e.g. `part store : SqlLinkStore[1]`.
    fn header(&self, id: ElementId) -> String {
        let e = &self.model.tree[id];
        let target = |e: &Element| {
            e.target
                .as_ref()
                .map(|t| self.reference(id, Role::Target, t))
                .unwrap_or_default()
        };
        let ends = |e: &Element| {
            format!(
                "connect {} to {}",
                self.reference(id, Role::End, &e.ends[0]),
                self.reference(id, Role::End, &e.ends[1])
            )
        };
        match e.kind {
            ElementKind::Import => {
                return format!(
                    "import {}{}",
                    target(e),
                    if e.wildcard { "::*" } else { "" }
                );
            }
            ElementKind::Satisfy => {
                let mut h = format!("satisfy {}", target(e));
                if let Some(by) = &e.by {
                    let _ = write!(h, " by {}", self.reference(id, Role::By, by));
                }
                return h;
            }
            ElementKind::Connection if is_bare_connect(e) => return ends(e),
            _ => {}
        }
        let mut parts: Vec<String> = Vec::new();
        if e.is_abstract {
            parts.push("abstract".into());
        }
        if let Some(direction) = e.direction {
            parts.push(direction.keyword().into());
        }
        if e.is_end {
            parts.push("end".into());
        }
        if e.kind != ElementKind::Reference {
            parts.push(e.kind.keyword().into());
        }
        let mut declared = false;
        if let Some(name) = &e.name {
            let mut text = String::new();
            let _ = write_name(&mut text, name);
            parts.push(text);
            declared = true;
        }
        if !e.typed_by.is_empty() {
            let conjugated = if e.conjugated { "~" } else { "" };
            parts.push(format!(
                ": {conjugated}{}",
                self.list(id, Role::TypedBy, &e.typed_by)
            ));
            declared = true;
        }
        if !e.specializes.is_empty() {
            parts.push(format!(
                ":> {}",
                self.list(id, Role::Specializes, &e.specializes)
            ));
            declared = true;
        }
        if !e.redefines.is_empty() {
            parts.push(format!(
                ":>> {}",
                self.list(id, Role::Redefines, &e.redefines)
            ));
            declared = true;
        }
        if let Some(multiplicity) = e.multiplicity {
            match parts.last_mut() {
                Some(last) if declared => last.push_str(&multiplicity.to_string()),
                _ => parts.push(multiplicity.to_string()),
            }
        }
        if let Some(value) = &e.value {
            parts.push(format!("= {value}"));
        }
        if e.ends.len() == 2 {
            parts.push(ends(e));
        }
        parts.join(" ")
    }
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

/// `doc /* text */` or `/* text */`; continuation lines align their `*`.
/// A `*/` inside the text would end the comment early, so it prints as `* /`.
fn print_comment(keyword: &str, text: Option<&str>, indent: &str, out: &mut String) {
    let text = text.unwrap_or("").replace("*/", "* /");
    let mut lines = text.lines();
    let first = lines.next().unwrap_or("");
    let rest: Vec<&str> = lines.collect();
    if rest.is_empty() {
        let _ = writeln!(out, "{keyword}/* {first} */");
        return;
    }
    let _ = writeln!(out, "{keyword}/* {first}");
    let align = " ".repeat(keyword.len() + 1);
    for line in rest {
        let line = format!("{indent}{align}* {line}");
        out.push_str(line.trim_end());
        out.push('\n');
    }
    let _ = writeln!(out, "{indent}{align}*/");
}
