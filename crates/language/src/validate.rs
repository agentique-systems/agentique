//! Model validity for the subset: references lead somewhere, types have the
//! right kind, redefinitions target inherited features, connections fit, parts
//! compose, requirements are satisfied by features of the subject's type, and
//! unsupported or unreadable text is reported. Linked references are followed
//! by identity; unlinked ones are resolved by name on every run.

use crate::library::library;
use crate::resolve::{LookupError, Model};
use crate::tree::*;
use std::collections::HashMap;

/// A problem at an element. `code` is short and stable; `message` is plain language.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub element: ElementId,
    pub location: Option<Location>,
    pub code: &'static str,
    pub message: String,
}

/// Validates the whole tree. Diagnostics come in document order.
pub fn validate(tree: &Tree) -> Vec<Diagnostic> {
    let mut checker = Checker::new(tree);
    checker.check_duplicates();
    let order = tree.walk();
    for &id in &order {
        checker.check(id);
    }
    let position: HashMap<ElementId, usize> =
        order.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    checker.out.sort_by_key(|d| position[&d.element]);
    checker.out
}

pub(crate) struct Checker<'a> {
    pub(crate) model: Model<'a>,
    out: Vec<Diagnostic>,
}

impl<'a> Checker<'a> {
    /// A checker that reports nothing yet, for queries (`Semantics`).
    pub(crate) fn new(tree: &'a Tree) -> Self {
        Checker {
            model: Model::new(tree, library()),
            out: Vec::new(),
        }
    }

    fn report(&mut self, id: ElementId, code: &'static str, message: String) {
        self.out.push(Diagnostic {
            element: id,
            location: self.model.tree[id].location,
            code,
            message,
        });
    }

    fn kind(&self, id: ElementId) -> ElementKind {
        self.model.get(id).kind
    }

    fn name(&self, id: ElementId) -> String {
        self.model.name(id).unwrap_or("?").to_string()
    }

    fn lookup_error(&mut self, id: ElementId, what: &str, written: &str, error: LookupError) {
        let (code, message) = match error {
            LookupError::NotFound(_) if written.is_empty() => {
                ("unresolved", format!("{what} is missing its name"))
            }
            LookupError::NotFound(name) if name.is_empty() || name == written => {
                ("unresolved", format!("cannot find {what} `{written}`"))
            }
            LookupError::NotFound(name) => (
                "unresolved",
                format!("cannot find `{name}` in {what} `{written}`"),
            ),
            LookupError::Ambiguous(_, candidates) => {
                let names: Vec<String> = candidates
                    .iter()
                    .map(|c| format!("`{}`", self.model.describe(*c)))
                    .collect();
                let names = names.join(" or ");
                (
                    "ambiguous",
                    format!("{what} `{written}` is ambiguous: it could be {names}"),
                )
            }
            LookupError::Removed(name) => (
                "removed-target",
                format!("{what} `{written}` referred to `{name}`, which no longer exists"),
            ),
            LookupError::Unsupported(construct) => {
                ("unsupported", format!("{construct} is not supported"))
            }
        };
        self.report(id, code, message);
    }

    /// Follows a reference held by `id` to the element it points at, or
    /// reports why it does not lead to a supported element.
    fn follow(
        &mut self,
        id: ElementId,
        role: Role,
        reference: &Reference,
        what: &str,
    ) -> Option<ElementId> {
        self.follow_steps(id, role, reference, what)?
            .last()
            .copied()
    }

    /// Like [`Checker::follow`], returning the element of every chain step.
    fn follow_steps(
        &mut self,
        id: ElementId,
        role: Role,
        reference: &Reference,
        what: &str,
    ) -> Option<Vec<ElementId>> {
        let written = reference.to_string();
        let steps = match self.model.resolve_reference(id, role, reference) {
            Ok(steps) => steps,
            Err(error) => {
                self.lookup_error(id, what, &written, error);
                return None;
            }
        };
        // Linked chain steps keep their targets; each must still be a
        // feature of the step before it.
        for pair in steps.windows(2) {
            let (outer, inner) = (pair[0], pair[1]);
            let owned_by = self.model.get(inner).owner();
            if !owned_by.is_some_and(|o| self.model.specializes(outer, o)) {
                let message = format!(
                    "in {what} `{written}`, `{}` is not a feature of `{}`",
                    self.name(inner),
                    self.name(outer)
                );
                self.report(id, "unresolved", message);
                return None;
            }
        }
        let target = *steps.last()?;
        if self.kind(target) == ElementKind::Unsupported {
            let note = self.model.get(target).note.clone().unwrap_or_default();
            self.report(
                id,
                "unsupported",
                format!("`{written}` is an unsupported {note}, so this reference is not checked"),
            );
            return None;
        }
        Some(steps)
    }

    /// A linked reference must print as a name that leads back to its
    /// target, so that saving and loading keeps it bound to the same element.
    fn check_reachable(&mut self, id: ElementId) {
        for (role, reference) in self.model.tree[id].references() {
            let Ok(target) = self.model.target(id, role, reference) else {
                continue;
            };
            if !self.model.name_for(id, role, reference).1 {
                let message = format!(
                    "no name written here leads to `{}` (it is private, hidden by another element with that name, or inside an unnamed element), so `{reference}` would not survive saving",
                    self.model.describe(target)
                );
                self.report(id, "unreachable-target", message);
            }
        }
    }

    fn check(&mut self, id: ElementId) {
        let element = &self.model.tree[id];
        match element.kind {
            ElementKind::Unsupported => {
                let note = element.note.clone().unwrap_or_default();
                self.report(
                    id,
                    "unsupported",
                    format!("{note} is not supported; it is kept as text and not validated"),
                );
            }
            ElementKind::SyntaxError => {
                let note = element.note.clone().unwrap_or_default();
                self.report(id, "syntax", note);
            }
            ElementKind::Import => self.check_import(id),
            ElementKind::Satisfy => self.check_satisfy(id),
            ElementKind::Package | ElementKind::Doc | ElementKind::Comment => {}
            kind if kind.is_definition() => self.check_definition(id),
            _ => self.check_usage(id),
        }
        self.check_hides_inherited(id);
        self.check_reachable(id);
    }

    fn check_duplicates(&mut self) {
        let groups: Vec<Vec<ElementId>> = self.model.duplicates().map(<[_]>::to_vec).collect();
        for group in groups {
            let in_library = group.iter().any(|id| self.model.is_library(*id));
            let authored: Vec<ElementId> = group
                .into_iter()
                .filter(|id| !self.model.is_library(*id))
                .collect();
            let skip = if in_library { 0 } else { 1 };
            for &id in authored.iter().skip(skip) {
                let name = self.name(id);
                let message = if in_library {
                    format!("`{name}` is also the name of a built-in library element")
                } else {
                    let at = self.model.tree[authored[0]]
                        .location
                        .map(|l| format!(" (at line {})", l.line))
                        .unwrap_or_default();
                    format!("another member of the same namespace is also named `{name}`{at}")
                };
                self.report(id, "duplicate-name", message);
            }
        }
    }

    /// An owned feature named like an inherited one must redefine it
    /// (explicitly, or implicitly as ends and subjects do).
    fn check_hides_inherited(&mut self, id: ElementId) {
        let element = &self.model.tree[id];
        let (Some(name), Some(owner)) = (&element.name, element.owner()) else {
            return;
        };
        if !(element.kind.is_usage() || element.kind.is_definition()) {
            return;
        }
        if let Some(first) = self.model.inherited(owner, name).first() {
            let message = format!(
                "`{name}` has the same name as the inherited `{}`; redefine it with `:>> {name}` or rename it",
                self.model.describe(*first)
            );
            self.report(id, "duplicate-name", message);
        }
    }

    fn check_import(&mut self, id: ElementId) {
        let element = &self.model.tree[id];
        let wildcard = element.wildcard;
        let Some(reference) = element.target.clone() else {
            self.report(
                id,
                "missing-target",
                "an import needs the name it imports".into(),
            );
            return;
        };
        let Some(target) = self.follow(id, Role::Target, &reference, "the imported name") else {
            return;
        };
        if wildcard && !self.kind(target).is_namespace() {
            let kind = self.kind(target).keyword();
            self.report(
                id,
                "wrong-kind",
                format!("`{reference}` is a {kind}, which has no members to import"),
            );
        }
    }

    fn check_definition(&mut self, id: ElementId) {
        let kind = self.kind(id);
        for general in self.model.tree[id].specializes.clone() {
            let Some(target) = self.follow(
                id,
                Role::Specializes,
                &general,
                "the specialised definition",
            ) else {
                continue;
            };
            if target == id {
                self.report(
                    id,
                    "specialization-cycle",
                    "this definition specialises itself".into(),
                );
            } else if !definition_fits(kind, self.kind(target)) {
                let message = format!(
                    "`{general}` is a {}; a {} can only specialise a {}",
                    self.kind(target).keyword(),
                    kind.keyword(),
                    allowed_generals(kind)
                );
                self.report(id, "wrong-kind", message);
            }
        }
        if self.model.in_cycle(id) {
            self.report(
                id,
                "specialization-cycle",
                "this definition specialises itself through its generals".into(),
            );
        }
        if kind == ElementKind::InterfaceDef {
            self.check_interface_def_ends(id);
        }
        if kind == ElementKind::PartDef {
            self.check_composition(id);
        }
        self.check_subjects(id);
    }

    fn check_usage(&mut self, id: ElementId) {
        let element = &self.model.tree[id];
        let kind = element.kind;
        let owner_kind = element.owner().map(|o| self.kind(o));
        if element.is_end
            && !matches!(
                owner_kind,
                Some(ElementKind::ConnectionDef | ElementKind::InterfaceDef)
            )
        {
            self.report(
                id,
                "misplaced-end",
                "`end` features belong in a connection def or interface def".into(),
            );
        } else if element.is_end
            && owner_kind == Some(ElementKind::InterfaceDef)
            && kind != ElementKind::Port
        {
            self.report(
                id,
                "wrong-kind",
                "the ends of an interface def are ports".into(),
            );
        }
        if kind == ElementKind::Subject
            && !matches!(
                owner_kind,
                Some(ElementKind::RequirementDef | ElementKind::Requirement)
            )
        {
            self.report(
                id,
                "misplaced-subject",
                "a subject belongs in a requirement".into(),
            );
        }
        self.check_typing(id);
        self.check_subsetting(id);
        self.check_redefinition(id);
        let element = &self.model.tree[id];
        if let Some(m) = element.multiplicity
            && m.upper.is_some_and(|upper| upper < m.lower)
        {
            self.report(
                id,
                "bad-multiplicity",
                format!("the lower bound of {m} is greater than its upper bound"),
            );
        }
        if let Some(value) = element.value.clone() {
            self.check_value(id, &value);
        }
        if matches!(kind, ElementKind::Connection | ElementKind::Interface) {
            match element.ends.len() {
                0 => {}
                2 => self.check_connection(id),
                n => {
                    let message = format!(
                        "a {} connects two ends (`connect a to b`) or is written without them; this one has {n}",
                        kind.keyword()
                    );
                    self.report(id, "wrong-end-count", message);
                }
            }
        }
        self.check_subjects(id);
    }

    fn check_typing(&mut self, id: ElementId) {
        let element = &self.model.tree[id];
        let (kind, conjugated) = (element.kind, element.conjugated);
        for reference in element.typed_by.clone() {
            let Some(target) = self.follow(id, Role::TypedBy, &reference, "the type") else {
                continue;
            };
            let target_kind = self.kind(target);
            if !allowed_types(kind).contains(&target_kind) {
                let expected = allowed_types(kind)
                    .iter()
                    .map(|k| k.keyword())
                    .collect::<Vec<_>>()
                    .join(" or ");
                let message = format!(
                    "`{reference}` is a {}; a {} must be typed by a {expected}",
                    target_kind.keyword(),
                    kind.keyword()
                );
                self.report(id, "wrong-type", message);
            }
        }
        if conjugated && kind != ElementKind::Port {
            self.report(
                id,
                "wrong-type",
                "only ports can have a conjugated type `~T`".into(),
            );
        }
    }

    fn check_subsetting(&mut self, id: ElementId) {
        let kind = self.kind(id);
        for reference in self.model.tree[id].specializes.clone() {
            let Some(target) =
                self.follow(id, Role::Specializes, &reference, "the subsetted feature")
            else {
                continue;
            };
            let target_kind = self.kind(target);
            if target == id {
                self.report(
                    id,
                    "specialization-cycle",
                    "this feature subsets itself".into(),
                );
            } else if !target_kind.is_usage() {
                let message = format!(
                    "`{reference}` is a {}; `:>` on a {} names a feature it subsets (use `:` for its type)",
                    target_kind.keyword(),
                    kind.keyword()
                );
                self.report(id, "wrong-kind", message);
            } else if !usage_fits(kind, target_kind) {
                let message = format!(
                    "a {} cannot subset `{reference}`, which is a {}",
                    kind.keyword(),
                    target_kind.keyword()
                );
                self.report(id, "wrong-kind", message);
            }
        }
    }

    fn check_redefinition(&mut self, id: ElementId) {
        let element = &self.model.tree[id];
        let (kind, owner) = (element.kind, element.owner());
        for reference in element.redefines.clone() {
            let owner_name = owner.map(|o| self.model.describe(o)).unwrap_or_default();
            let not_inherited = |written: &str| {
                format!(
                    "`{written}` is not a feature that `{owner_name}` inherits; `:>>` can only redefine an inherited feature"
                )
            };
            let target = match self.model.target(id, Role::Redefines, &reference) {
                Err(LookupError::NotFound(_)) => {
                    self.report(
                        id,
                        "redefines-unknown",
                        not_inherited(&reference.to_string()),
                    );
                    continue;
                }
                Err(_) => {
                    self.follow(id, Role::Redefines, &reference, "the redefined feature");
                    continue;
                }
                Ok(target) => target,
            };
            if !owner.is_some_and(|o| self.model.is_inherited(o, target)) {
                let written = self.model.describe(target);
                self.report(id, "redefines-unknown", not_inherited(&written));
            } else if self.kind(target) == ElementKind::Unsupported {
                self.follow(id, Role::Redefines, &reference, "the redefined feature");
            } else if !usage_fits(kind, self.kind(target)) {
                let message = format!(
                    "a {} cannot redefine `{}`, which is a {}",
                    kind.keyword(),
                    self.model.describe(target),
                    self.kind(target).keyword()
                );
                self.report(id, "wrong-kind", message);
            }
        }
    }

    /// Literal values are checked against the built-in scalar types.
    fn check_value(&mut self, id: ElementId, value: &Literal) {
        let Some(&(declared, _)) = self.model.types_of(id).first() else {
            return;
        };
        if !self.model.is_library(declared) {
            return;
        }
        let literal_type = match value {
            Literal::Boolean(_) => "Boolean",
            Literal::String(_) => "String",
            Literal::Real(_) => "Real",
            Literal::Integer(text) if text.starts_with('-') => "Integer",
            Literal::Integer(text) if text.chars().all(|c| c == '0') => "Natural",
            Literal::Integer(_) => "Positive",
        };
        let name = QualifiedName::new(["ScalarValues", literal_type]);
        let Ok(literal_def) = self.model.resolve_global(&name) else {
            return;
        };
        if !self.model.specializes(literal_def, declared) {
            let message = format!("`{value}` is not a valid {}", self.name(declared));
            self.report(id, "wrong-value", message);
        }
    }

    fn check_connection(&mut self, id: ElementId) {
        let element = &self.model.tree[id];
        let (kind, owner) = (element.kind, element.owner());
        let mut ends = Vec::new();
        for reference in element.ends.clone() {
            let Some(steps) = self.follow_steps(id, Role::End, &reference, "the connection end")
            else {
                return;
            };
            let feature = *steps.last().expect("follow_steps returns steps");
            let feature_kind = self.kind(feature);
            if !feature_kind.is_usage() {
                let message = format!(
                    "the connection end `{reference}` is a {}, not a feature",
                    feature_kind.keyword()
                );
                self.report(id, "wrong-kind", message);
                return;
            }
            if kind == ElementKind::Interface && feature_kind != ElementKind::Port {
                let message = format!(
                    "an interface connects ports, but `{reference}` is a {}",
                    feature_kind.keyword()
                );
                self.report(id, "wrong-kind", message);
                return;
            }
            ends.push((reference.to_string(), feature, steps));
        }
        let definition = self
            .model
            .types_of(id)
            .into_iter()
            .map(|(def, _)| def)
            .find(|def| {
                matches!(
                    self.kind(*def),
                    ElementKind::ConnectionDef | ElementKind::InterfaceDef
                )
            });
        if let Some(definition) = definition {
            let def_ends = self.model.ends(definition);
            let def_name = self.model.describe(definition);
            if def_ends.len() != ends.len() {
                let message = format!(
                    "`{def_name}` has {} ends, but `connect` gives {}",
                    def_ends.len(),
                    ends.len()
                );
                self.report(id, "incompatible-ends", message);
                return;
            }
            for ((written, feature, _), end) in ends.iter().zip(def_ends) {
                if !self.conforms(*feature, end) {
                    let message = format!(
                        "`{written}` ({}) does not fit end `{}` ({}) of `{def_name}`",
                        self.type_text(*feature),
                        self.name(end),
                        self.type_text(end)
                    );
                    self.report(id, "incompatible-ends", message);
                }
            }
            return;
        }
        if !ends
            .iter()
            .all(|(_, f, _)| self.kind(*f) == ElementKind::Port)
        {
            return;
        }
        // A port connected to a port of a part inside the first port's part
        // passes items on (`p` to `inner.p`, `sub.i` to `sub.inner.i`): same
        // directions. Otherwise the two ports face each other: mirrored.
        let (a, b) = (&ends[0], &ends[1]);
        let problem = match self.delegation(owner, &a.2, &b.2) {
            Some(true) => self.port_mismatch(a.1, b.1, true),
            Some(false) => self.port_mismatch(b.1, a.1, true),
            None => self.port_mismatch(a.1, b.1, false),
        };
        if let Some(problem) = problem {
            let message = format!(
                "`{}` ({}) and `{}` ({}) do not fit: {problem}",
                a.0,
                self.type_text(a.1),
                b.0,
                self.type_text(b.1)
            );
            self.report(id, "incompatible-ends", message);
        }
    }

    /// Is one end a port of the part that contains the other end's part?
    /// `Some(true)` when `a` is that outer port, `Some(false)` when `b` is.
    /// The part of an end is its chain minus the last step; an empty one is
    /// the connection's owner, so the port must be one of the owner's.
    fn delegation(
        &self,
        owner: Option<ElementId>,
        a: &[ElementId],
        b: &[ElementId],
    ) -> Option<bool> {
        let part = |steps: &[ElementId]| steps[..steps.len() - 1].to_vec();
        let (part_a, part_b) = (part(a), part(b));
        let outer_of = |outer: &[ElementId], port: ElementId, inner: &[ElementId]| {
            outer.len() < inner.len()
                && inner.starts_with(outer)
                && (!outer.is_empty()
                    || self
                        .model
                        .get(port)
                        .owner()
                        .is_some_and(|o| owner.is_some_and(|c| self.model.specializes(c, o))))
        };
        if outer_of(&part_a, a[a.len() - 1], &part_b) {
            Some(true)
        } else if outer_of(&part_b, b[b.len() - 1], &part_a) {
            Some(false)
        } else {
            None
        }
    }

    /// A part def must not contain itself through required parts (lower bound
    /// of at least 1; a part without a multiplicity is required).
    fn check_composition(&mut self, id: ElementId) {
        let mut seen = vec![id];
        let mut i = 0;
        while i < seen.len() {
            for feature in self.model.features(seen[i]) {
                let element = self.model.get(feature);
                let required = element.multiplicity.is_none_or(|m| m.lower > 0);
                if element.kind != ElementKind::Part || !required {
                    continue;
                }
                for (ty, _) in self.model.types_of(feature) {
                    if ty == id {
                        let message = format!(
                            "it contains itself through the required part `{}`; give that part a lower bound of 0",
                            self.model.describe(feature)
                        );
                        self.report(id, "composition-cycle", message);
                        return;
                    }
                    if !seen.contains(&ty) {
                        seen.push(ty);
                    }
                }
            }
            i += 1;
        }
    }

    /// The two port ends of an interface def must face each other.
    fn check_interface_def_ends(&mut self, id: ElementId) {
        let ends: Vec<ElementId> = self.model.tree[id]
            .children()
            .iter()
            .copied()
            .filter(|c| self.model.tree[*c].is_end && self.kind(*c) == ElementKind::Port)
            .collect();
        if let [a, b] = ends[..]
            && let Some(problem) = self.port_mismatch(a, b, false)
        {
            let message = format!(
                "its ends `{}` ({}) and `{}` ({}) do not fit: {problem}",
                self.name(a),
                self.type_text(a),
                self.name(b),
                self.type_text(b)
            );
            self.report(id, "incompatible-ends", message);
        }
    }

    fn check_subjects(&mut self, id: ElementId) {
        let subjects = self.model.tree[id]
            .children()
            .iter()
            .filter(|c| self.model.tree[**c].kind == ElementKind::Subject)
            .count();
        if subjects > 1 {
            self.report(
                id,
                "duplicate-subject",
                "a requirement has at most one subject".into(),
            );
        }
    }

    fn check_satisfy(&mut self, id: ElementId) {
        let element = &self.model.tree[id];
        let (target, by) = (element.target.clone(), element.by.clone());
        if target.is_none() {
            self.report(
                id,
                "missing-target",
                "`satisfy` needs the requirement it satisfies".into(),
            );
        }
        let requirement = target.and_then(|reference| {
            let r = self.follow(id, Role::Target, &reference, "the requirement")?;
            if self.kind(r) == ElementKind::Requirement {
                return Some(r);
            }
            let hint = if self.kind(r) == ElementKind::RequirementDef {
                format!(" (declare `requirement x : {reference};` and satisfy that)")
            } else {
                String::new()
            };
            let message = format!(
                "`{reference}` is a {}; `satisfy` names a requirement{hint}",
                self.kind(r).keyword()
            );
            self.report(id, "wrong-kind", message);
            None
        });
        let Some(by) = by else {
            return;
        };
        let Some(feature) = self.follow(id, Role::By, &by, "the satisfying feature") else {
            return;
        };
        if !self.kind(feature).is_usage() {
            let message = format!(
                "`{by}` is a {}, not a feature",
                self.kind(feature).keyword()
            );
            self.report(id, "wrong-kind", message);
            return;
        }
        let Some(subject) = requirement.and_then(|r| self.model.subject(r)) else {
            return;
        };
        let expected = self.model.types_of(subject);
        let actual = self.model.types_of(feature);
        let fits = expected
            .iter()
            .all(|(t, _)| actual.iter().any(|(a, _)| self.model.specializes(*a, *t)));
        if !fits {
            let message = format!(
                "`{by}` ({}) cannot be the subject of `{}`, whose subject `{}` is a {}",
                self.type_text(feature),
                self.model.describe(requirement.expect("subject found")),
                self.name(subject),
                self.type_text(subject)
            );
            self.report(id, "wrong-subject", message);
        }
    }

    /// Does `feature` have (a specialisation of) every type of `end`, with
    /// the same conjugation?
    fn conforms(&self, feature: ElementId, end: ElementId) -> bool {
        let actual = self.model.types_of(feature);
        self.model.types_of(end).iter().all(|(t, conjugated)| {
            actual
                .iter()
                .any(|(a, c)| c == conjugated && self.model.specializes(*a, *t))
        })
    }

    /// Why two ports cannot be connected, if they cannot. Every directed
    /// feature must meet a same-named feature: of the opposite direction
    /// when the ports face each other, of the same direction when `a` passes
    /// items on to an inner part's port `b` (`delegation`). What is sent must
    /// be (a specialisation of) what is received.
    pub(crate) fn port_mismatch(
        &self,
        a: ElementId,
        b: ElementId,
        delegation: bool,
    ) -> Option<String> {
        let (items_a, items_b) = (self.directed_features(a), self.directed_features(b));
        for (name, direction, ty) in &items_a {
            let Some((_, other_direction, other_ty)) = items_b.iter().find(|(n, _, _)| n == name)
            else {
                return Some(format!("`{name}` has no counterpart on the other side"));
            };
            let expected = if delegation {
                *direction
            } else {
                direction.flipped()
            };
            if *other_direction != expected {
                let rule = if delegation {
                    "a port passed on to an inner part keeps its directions"
                } else {
                    "one side must send what the other receives"
                };
                return Some(format!(
                    "`{name}` is `{}` on one side and `{}` on the other; {rule}",
                    direction.keyword(),
                    other_direction.keyword()
                ));
            }
            // Items flow out of `a` when it sends (or, passed on, receives
            // from outside); otherwise they flow into `a`.
            let from_a = (*direction == Direction::Out) != delegation;
            let fits = match direction {
                Direction::InOut => self.carries(*ty, *other_ty) && self.carries(*other_ty, *ty),
                _ if from_a => self.carries(*ty, *other_ty),
                _ => self.carries(*other_ty, *ty),
            };
            if !fits {
                return Some(format!(
                    "`{name}` sends {} where {} is received",
                    self.type_name(if from_a { *ty } else { *other_ty }),
                    self.type_name(if from_a { *other_ty } else { *ty })
                ));
            }
        }
        items_b
            .iter()
            .find(|(name, _, _)| !items_a.iter().any(|(n, _, _)| n == name))
            .map(|(name, _, _)| format!("`{name}` has no counterpart on the other side"))
    }

    /// Can items of type `sent` go where `received` is expected?
    fn carries(&self, sent: Option<ElementId>, received: Option<ElementId>) -> bool {
        match (sent, received) {
            (Some(sent), Some(received)) => self.model.specializes(sent, received),
            _ => true,
        }
    }

    fn type_name(&self, ty: Option<ElementId>) -> String {
        ty.map_or("an untyped item".into(), |t| format!("`{}`", self.name(t)))
    }

    /// (name, direction as seen from outside the port, type) of each directed
    /// feature of a port, with conjugation applied.
    fn directed_features(&self, port: ElementId) -> Vec<(String, Direction, Option<ElementId>)> {
        let conjugated = self.model.types_of(port).first().is_some_and(|(_, c)| *c);
        self.model
            .features(port)
            .into_iter()
            .filter_map(|f| {
                let direction = self.model.get(f).direction?;
                let direction = if conjugated {
                    direction.flipped()
                } else {
                    direction
                };
                let ty = self.model.types_of(f).first().map(|(t, _)| *t);
                Some((self.model.name(f)?.to_string(), direction, ty))
            })
            .collect()
    }

    fn type_text(&self, feature: ElementId) -> String {
        let types: Vec<String> = self
            .model
            .types_of(feature)
            .iter()
            .map(|(t, conjugated)| {
                let name = self.name(*t);
                if *conjugated {
                    format!("~{name}")
                } else {
                    name
                }
            })
            .collect();
        if types.is_empty() {
            "untyped".into()
        } else {
            types.join(", ")
        }
    }
}

fn definition_fits(specific: ElementKind, general: ElementKind) -> bool {
    use ElementKind::*;
    specific == general
        || (specific, general) == (PartDef, ItemDef)
        || (specific, general) == (InterfaceDef, ConnectionDef)
}

fn allowed_generals(kind: ElementKind) -> &'static str {
    match kind {
        ElementKind::PartDef => "part def or item def",
        ElementKind::InterfaceDef => "interface def or connection def",
        other => other.keyword(),
    }
}

/// Can a usage of kind `specific` subset or redefine one of kind `general`?
/// A keyword-less reference usage fits any.
fn usage_fits(specific: ElementKind, general: ElementKind) -> bool {
    use ElementKind::*;
    specific == general
        || specific == Reference
        || general == Reference
        || (specific, general) == (Part, Item)
        || (specific, general) == (Interface, Connection)
}

/// The definition kinds a usage may be typed by.
fn allowed_types(kind: ElementKind) -> &'static [ElementKind] {
    use ElementKind::*;
    const ANY: &[ElementKind] = &[
        PartDef,
        ItemDef,
        PortDef,
        AttributeDef,
        ConnectionDef,
        InterfaceDef,
        RequirementDef,
    ];
    match kind {
        Part => &[PartDef],
        Item => &[ItemDef, PartDef],
        Port => &[PortDef],
        Attribute => &[AttributeDef],
        Connection => &[ConnectionDef, InterfaceDef],
        Interface => &[InterfaceDef],
        Requirement => &[RequirementDef],
        Subject | Reference => ANY,
        _ => &[],
    }
}
