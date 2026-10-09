//! Prints a tree as canonical SysML text: four-space indentation, one member
//! per line, a blank line between package members except consecutive imports.
//! `Unsupported` and `SyntaxError` elements are printed exactly as they were read.
//!
//! A linked reference is printed with a name that resolves back to its
//! target from where it is written, so renames and moves carry through. The
//! same holds for the names inside expressions (C-50).
//!
//! Steps of an action body or a scenario after the first are written with
//! `then`, the standard notation for "after the one before".

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
            printer.members(document.members(), 0, None, &mut out);
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
    printer.member(id, 0, false, &mut out);
    Some(out)
}

/// A reference held by `holder` as [`print`] writes it: each step named as
/// printed, targets kept. Use it to keep the written names current, for
/// example before the targets are removed.
pub fn printed_reference(
    tree: &Tree,
    holder: ElementId,
    role: Role,
    reference: &Reference,
) -> Reference {
    Model::new(tree, library())
        .printed(holder, role, reference)
        .0
}

/// An expression held by `holder` as [`print`] writes it, with each name
/// printed so that it leads back to its target.
pub fn print_expression(tree: &Tree, holder: ElementId, expression: &Expression) -> String {
    let printer = Printer {
        model: Model::new(tree, library()),
    };
    printer.expression(holder, expression)
}

struct Printer<'a> {
    model: Model<'a>,
}

/// Whether members of an element of this kind are steps that run in order,
/// written with `then` after the first.
fn sequential(kind: ElementKind) -> bool {
    matches!(kind, ElementKind::Action | ElementKind::VerificationDef)
}

/// Whether a member is a step: an action node or a check.
fn is_step(element: &Element) -> bool {
    (element.kind.is_action_node() || element.kind == ElementKind::AssertConstraint)
        && element.state_action.is_none()
}

impl Printer<'_> {
    /// Prints `members`; `owner` is the kind of their owner (`None` at the
    /// top level of a document).
    fn members(
        &self,
        members: &[ElementId],
        level: usize,
        owner: Option<ElementKind>,
        out: &mut String,
    ) {
        let spaced = owner.is_none_or(|kind| kind == ElementKind::Package);
        let sequence = owner.is_some_and(sequential);
        let mut previous: Option<ElementKind> = None;
        let mut steps = 0;
        for &id in members {
            let element = &self.model.tree[id];
            let kind = element.kind;
            let both_imports = previous == Some(ElementKind::Import) && kind == ElementKind::Import;
            if spaced && previous.is_some() && !both_imports {
                out.push('\n');
            }
            let step = sequence && is_step(element);
            self.member(id, level, step && steps > 0, out);
            if step {
                steps += 1;
            }
            previous = Some(kind);
        }
    }

    /// Prints one member on its own lines; `then` writes it as the step
    /// after the one before.
    fn member(&self, id: ElementId, level: usize, then: bool, out: &mut String) {
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
        if then {
            out.push_str("then ");
        }
        if let Some(which) = e.state_action {
            out.push_str(which.keyword());
            // `entry;`: an empty action.
            if e.kind == ElementKind::Action && e.name.is_none() && e.children().is_empty() {
                out.push_str(";\n");
                return;
            }
            out.push(' ');
        }
        match e.kind {
            ElementKind::Transition => return self.transition(id, level, out),
            ElementKind::If => {
                self.if_node(id, level, out);
                out.push('\n');
                return;
            }
            ElementKind::AssertConstraint
            | ElementKind::AssumeConstraint
            | ElementKind::RequireConstraint => return self.constraint(id, level, out),
            _ => {}
        }
        out.push_str(&self.header(id));
        self.body(id, level, out);
    }

    /// `;` or `{ members }` and a line end.
    fn body(&self, id: ElementId, level: usize, out: &mut String) {
        let e = &self.model.tree[id];
        if e.children().is_empty() {
            out.push_str(";\n");
        } else {
            out.push_str(" {\n");
            self.members(e.children(), level + 1, Some(e.kind), out);
            out.push_str(&"    ".repeat(level));
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

    fn expression(&self, id: ElementId, expression: &Expression) -> String {
        let mut out = String::new();
        expression.write(&mut out, &mut |reference| {
            self.reference(id, Role::Value, reference)
        });
        out
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
        let via = |e: &Element| {
            e.via
                .as_ref()
                .map(|v| format!(" via {}", self.reference(id, Role::Via, v)))
                .unwrap_or_default()
        };
        let expression = |e: &Element| {
            e.expression
                .as_ref()
                .map(|x| self.expression(id, x))
                .unwrap_or_default()
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
            ElementKind::Dependency => {
                let mut h = "dependency ".to_string();
                if let Some(name) = &e.name {
                    let _ = write_name(&mut h, name);
                    h.push(' ');
                }
                if e.ends.len() == 2 {
                    let _ = write!(
                        h,
                        "from {} to {}",
                        self.reference(id, Role::End, &e.ends[0]),
                        self.reference(id, Role::End, &e.ends[1])
                    );
                }
                return h;
            }
            ElementKind::Succession => return format!("then {}", target(e)),
            ElementKind::Verify => return format!("verify {}", target(e)),
            ElementKind::Send => return format!("send {}{}", expression(e), via(e)),
            ElementKind::Assign => {
                return format!("assign {} := {}", target(e), expression(e));
            }
            ElementKind::Accept => {
                if e.after {
                    return format!("accept after {}", expression(e));
                }
                let mut h = "accept ".to_string();
                if let Some(name) = &e.name {
                    let _ = write_name(&mut h, name);
                    h.push_str(" : ");
                }
                h.push_str(&self.list(id, Role::TypedBy, &e.typed_by));
                h.push_str(&via(e));
                return h;
            }
            ElementKind::Action | ElementKind::Objective if e.name.is_none() => {
                return e.kind.keyword().to_string();
            }
            _ => {}
        }
        // The order of SysML's usage prefix: direction, `abstract`, then
        // `ref` right before the kind keyword (8.2.2.6.2).
        let mut parts: Vec<String> = Vec::new();
        if let Some(direction) = e.direction {
            parts.push(direction.keyword().into());
        }
        if e.is_abstract {
            parts.push("abstract".into());
        }
        if e.is_end {
            parts.push("end".into());
        }
        if e.exhibit {
            parts.push("exhibit".into());
        }
        if e.referential {
            parts.push("ref".into());
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
        } else if let Some(value) = &e.expression {
            parts.push(format!("= {}", self.expression(id, value)));
        }
        if e.ends.len() == 2 {
            parts.push(ends(e));
        }
        parts.join(" ")
    }

    /// `transition [name] first S [accept ...] [if g] [do effect] then T;`
    fn transition(&self, id: ElementId, level: usize, out: &mut String) {
        let e = &self.model.tree[id];
        out.push_str("transition ");
        if let Some(name) = &e.name {
            let _ = write_name(out, name);
            out.push(' ');
        }
        let end = |i: usize| {
            e.ends
                .get(i)
                .map(|r| self.reference(id, Role::End, r))
                .unwrap_or_default()
        };
        let _ = write!(out, "first {}", end(0));
        let trigger = e
            .children()
            .iter()
            .copied()
            .find(|c| self.model.tree[*c].kind == ElementKind::Accept);
        if let Some(trigger) = trigger {
            let _ = write!(out, " {}", self.header(trigger));
        }
        if let Some(guard) = &e.guard {
            let _ = write!(out, " if {}", self.expression(id, guard));
        }
        let effect = e.children().iter().copied().find(|c| {
            let kind = self.model.tree[*c].kind;
            kind.is_action_node() && kind != ElementKind::Accept
        });
        if let Some(effect) = effect {
            let element = &self.model.tree[effect];
            out.push_str(" do ");
            out.push_str(&self.header(effect));
            if !element.children().is_empty() {
                out.push_str(" {\n");
                self.members(element.children(), level + 1, Some(element.kind), out);
                out.push_str(&"    ".repeat(level));
                out.push('}');
            }
        }
        let _ = write!(out, " then {}", end(1));
        // Other members (a doc, a comment) go in the transition's body.
        let rest: Vec<ElementId> = e
            .children()
            .iter()
            .copied()
            .filter(|c| Some(*c) != trigger && Some(*c) != effect)
            .collect();
        if rest.is_empty() {
            out.push_str(";\n");
        } else {
            out.push_str(" {\n");
            self.members(&rest, level + 1, Some(ElementKind::Transition), out);
            out.push_str(&"    ".repeat(level));
            out.push_str("}\n");
        }
    }

    /// `if c { ... } else { ... }`, without the line end.
    fn if_node(&self, id: ElementId, level: usize, out: &mut String) {
        let e = &self.model.tree[id];
        let condition = e
            .expression
            .as_ref()
            .map(|x| self.expression(id, x))
            .unwrap_or_default();
        let _ = write!(out, "if {condition} ");
        let branches = e.children();
        let branch = |branch: ElementId, out: &mut String| {
            let element = &self.model.tree[branch];
            out.push_str("{\n");
            self.members(
                element.children(),
                level + 1,
                Some(ElementKind::Action),
                out,
            );
            out.push_str(&"    ".repeat(level));
            out.push('}');
        };
        if let Some(first) = branches.first() {
            branch(*first, out);
        } else {
            out.push_str("{\n");
            out.push_str(&"    ".repeat(level));
            out.push('}');
        }
        if let Some(second) = branches.get(1) {
            out.push_str(" else ");
            if self.model.tree[*second].kind == ElementKind::If {
                self.if_node(*second, level, out);
            } else {
                branch(*second, out);
            }
        }
    }

    /// `assert constraint [name] { [doc] expression }`; also `assume` and
    /// `require`, written `require constraint [name];` with nothing inside.
    fn constraint(&self, id: ElementId, level: usize, out: &mut String) {
        let e = &self.model.tree[id];
        out.push_str(e.kind.keyword());
        if let Some(name) = &e.name {
            out.push(' ');
            let _ = write_name(out, name);
        }
        if e.expression.is_none() && e.children().is_empty() {
            out.push_str(";\n");
            return;
        }
        out.push_str(" {\n");
        self.members(e.children(), level + 1, Some(e.kind), out);
        let inner = "    ".repeat(level + 1);
        if let Some(expression) = &e.expression {
            let _ = writeln!(out, "{inner}{}", self.expression(id, expression));
        }
        out.push_str(&"    ".repeat(level));
        out.push_str("}\n");
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
        && !e.is_end
        && !e.referential
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
