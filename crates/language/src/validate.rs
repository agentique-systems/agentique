//! Model validity for the subset: names resolve, types have the right kind,
//! redefinitions target inherited features, connections fit, requirements are
//! satisfied by features of the subject's type, and unsupported or unreadable
//! text is reported. Each run re-resolves the whole tree.

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
    let mut checker = Checker {
        model: Model::new(tree, library()),
        out: Vec::new(),
    };
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

struct Checker<'a> {
    model: Model<'a>,
    out: Vec<Diagnostic>,
}

impl Checker<'_> {
    fn report(&mut self, id: ElementId, code: &'static str, message: String) {
        self.out.push(Diagnostic {
            element: id,
            location: self.model.tree[id].location,
            code,
            message,
        });
    }

    fn lookup_error(&mut self, id: ElementId, what: &str, written: &str, error: LookupError) {
        match error {
            LookupError::NotFound(name) if name.is_empty() || name == written => {
                self.report(id, "unresolved", format!("cannot find {what} `{written}`"))
            }
            LookupError::NotFound(name) => self.report(
                id,
                "unresolved",
                format!("cannot find `{name}` in {what} `{written}`"),
            ),
            LookupError::Ambiguous(_, candidates) => {
                let names: Vec<String> = candidates
                    .iter()
                    .map(|c| format!("`{}`", self.model.describe(*c)))
                    .collect();
                self.report(
                    id,
                    "ambiguous",
                    format!(
                        "{what} `{written}` is ambiguous: it could be {}",
                        names.join(" or ")
                    ),
                );
            }
            LookupError::Unsupported(construct) => {
                self.report(id, "unsupported", format!("{construct} is not supported"))
            }
        }
    }

    fn kind(&self, id: ElementId) -> ElementKind {
        self.model.get(id).kind
    }

    /// Reports a reference to an unsupported element; returns true if it was one.
    fn unsupported_target(&mut self, id: ElementId, target: ElementId, written: &str) -> bool {
        if self.kind(target) != ElementKind::Unsupported {
            return false;
        }
        let note = self.model.get(target).note.clone().unwrap_or_default();
        self.report(
            id,
            "unsupported",
            format!("`{written}` is an unsupported {note}, so this reference is not checked"),
        );
        true
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
            ElementKind::Package | ElementKind::Doc => {}
            kind if kind.is_definition() => self.check_definition(id),
            _ => self.check_usage(id),
        }
        self.check_hides_inherited(id);
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
                let name = self.model.tree[id]
                    .effective_name()
                    .unwrap_or("")
                    .to_string();
                let message = if in_library {
                    format!("`{name}` is also the name of a built-in library element")
                } else {
                    let first = authored[0];
                    let at = self.model.tree[first]
                        .location
                        .map(|l| format!(" (at line {})", l.line))
                        .unwrap_or_default();
                    format!("another member of the same namespace is also named `{name}`{at}")
                };
                self.report(id, "duplicate-name", message);
            }
        }
    }

    /// An owned feature named like an inherited one must redefine it.
    fn check_hides_inherited(&mut self, id: ElementId) {
        let element = &self.model.tree[id];
        let (Some(name), Some(owner)) = (&element.name, element.owner) else {
            return;
        };
        if !element.redefines.is_empty()
            || !(element.kind.is_usage() || element.kind.is_definition())
        {
            return;
        }
        let inherited = self.model.inherited_named(owner, name);
        if let Some(first) = inherited.first() {
            let message = format!(
                "`{name}` has the same name as the inherited `{}`; redefine it with `:>> {name}` or rename it",
                self.model.describe(*first)
            );
            self.report(id, "duplicate-name", message);
        }
    }

    fn check_import(&mut self, id: ElementId) {
        let element = &self.model.tree[id];
        let written = element.target.clone().unwrap_or_default().to_string();
        let wildcard = element.wildcard;
        match self.model.resolve_import(id) {
            Err(error) => self.lookup_error(id, "the imported name", &written, error),
            Ok(target) if wildcard && !self.kind(target).is_namespace() => {
                let kind = self.kind(target).keyword();
                self.report(
                    id,
                    "wrong-kind",
                    format!("`{written}` is a {kind}, which has no members to import"),
                );
            }
            Ok(_) => {}
        }
    }

    fn check_definition(&mut self, id: ElementId) {
        let element = &self.model.tree[id];
        let kind = element.kind;
        for general in element.specializes.clone() {
            let written = general.to_string();
            match self.model.resolve(element.owner, &general) {
                Err(error) => self.lookup_error(id, "the specialised definition", &written, error),
                Ok(target) if self.unsupported_target(id, target, &written) => {}
                Ok(target) if !definition_fits(kind, self.kind(target)) => {
                    let message = format!(
                        "`{written}` is a {}; a {} can only specialise a {}",
                        self.kind(target).keyword(),
                        kind.keyword(),
                        allowed_generals(kind)
                    );
                    self.report(id, "wrong-kind", message);
                }
                Ok(_) => {}
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
        let owner_kind = element.owner.map(|o| self.kind(o));
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
        if matches!(kind, ElementKind::Connection | ElementKind::Interface)
            && !element.ends.is_empty()
        {
            self.check_connection(id);
        }
        self.check_subjects(id);
    }

    fn check_typing(&mut self, id: ElementId) {
        let element = &self.model.tree[id];
        let kind = element.kind;
        for type_ref in element.typed_by.clone() {
            let written = type_ref.to_string();
            let target = match self.model.resolve(element.owner, &type_ref.name) {
                Err(error) => {
                    self.lookup_error(id, "the type", &written, error);
                    continue;
                }
                Ok(target) => target,
            };
            if self.unsupported_target(id, target, &written) {
                continue;
            }
            let target_kind = self.kind(target);
            if !allowed_types(kind).contains(&target_kind) {
                let expected = allowed_types(kind)
                    .iter()
                    .map(|k| k.keyword())
                    .collect::<Vec<_>>()
                    .join(" or ");
                let message = format!(
                    "`{}` is a {}; a {} must be typed by a {expected}",
                    type_ref.name,
                    target_kind.keyword(),
                    kind.keyword()
                );
                self.report(id, "wrong-type", message);
            }
            if type_ref.conjugated && kind != ElementKind::Port {
                self.report(
                    id,
                    "wrong-type",
                    format!("only ports can have a conjugated type like `{written}`"),
                );
            }
        }
    }

    fn check_subsetting(&mut self, id: ElementId) {
        let element = &self.model.tree[id];
        let kind = element.kind;
        for name in element.specializes.clone() {
            let written = name.to_string();
            match self.model.resolve(element.owner, &name) {
                Err(error) => self.lookup_error(id, "the subsetted feature", &written, error),
                Ok(target) if self.unsupported_target(id, target, &written) => {}
                Ok(target) if !self.kind(target).is_usage() => {
                    let message = format!(
                        "`{written}` is a {}; `:>` on a {} names a feature it subsets (use `:` for its type)",
                        self.kind(target).keyword(),
                        kind.keyword()
                    );
                    self.report(id, "wrong-kind", message);
                }
                Ok(target) if !usage_fits(kind, self.kind(target)) => {
                    let message = format!(
                        "a {} cannot subset `{written}`, which is a {}",
                        kind.keyword(),
                        self.kind(target).keyword()
                    );
                    self.report(id, "wrong-kind", message);
                }
                Ok(_) => {}
            }
        }
    }

    fn check_redefinition(&mut self, id: ElementId) {
        let element = &self.model.tree[id];
        let kind = element.kind;
        for name in element.redefines.clone() {
            let written = name.to_string();
            match self.model.resolve_redefined(id, &name) {
                Err(LookupError::NotFound(_)) => {
                    let owner = element
                        .owner
                        .map(|o| self.model.describe(o))
                        .unwrap_or_default();
                    let message = format!(
                        "`{written}` is not a feature that `{owner}` inherits; `:>>` can only redefine an inherited feature"
                    );
                    self.report(id, "redefines-unknown", message);
                }
                Err(error) => self.lookup_error(id, "the redefined feature", &written, error),
                Ok(target) if self.unsupported_target(id, target, &written) => {}
                Ok(target) if !usage_fits(kind, self.kind(target)) => {
                    let message = format!(
                        "a {} cannot redefine `{}`, which is a {}",
                        kind.keyword(),
                        self.model.describe(target),
                        self.kind(target).keyword()
                    );
                    self.report(id, "wrong-kind", message);
                }
                Ok(_) => {}
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
        let Ok(literal_def) = self.model.resolve(None, &name) else {
            return;
        };
        if !self.model.specializes(literal_def, declared) {
            let declared_name = self.model.get(declared).name.clone().unwrap_or_default();
            self.report(
                id,
                "wrong-value",
                format!("`{value}` is not a valid {declared_name}"),
            );
        }
    }

    fn check_connection(&mut self, id: ElementId) {
        let element = &self.model.tree[id];
        let (kind, scope) = (element.kind, element.owner);
        let mut ends = Vec::new();
        for chain in element.ends.clone() {
            let written = chain.to_string();
            let feature = match self.model.resolve_chain(scope, &chain) {
                Err(error) => {
                    self.lookup_error(id, "the connection end", &written, error);
                    return;
                }
                Ok(steps) => *steps.last().expect("a chain has steps"),
            };
            let feature_kind = self.kind(feature);
            if !feature_kind.is_usage() {
                let message = format!(
                    "the connection end `{written}` is a {}, not a feature",
                    feature_kind.keyword()
                );
                self.report(id, "wrong-kind", message);
                return;
            }
            if kind == ElementKind::Interface && feature_kind != ElementKind::Port {
                let message = format!(
                    "an interface connects ports, but `{written}` is a {}",
                    feature_kind.keyword()
                );
                self.report(id, "wrong-kind", message);
                return;
            }
            ends.push((written, feature));
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
            let def_ends: Vec<ElementId> = self
                .model
                .features(definition)
                .into_iter()
                .filter(|f| self.model.get(*f).is_end)
                .collect();
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
            for ((written, feature), end) in ends.iter().zip(def_ends) {
                if !self.conforms(*feature, end) {
                    let end_name = self
                        .model
                        .get(end)
                        .effective_name()
                        .unwrap_or("?")
                        .to_string();
                    let message = format!(
                        "`{written}` ({}) does not fit end `{end_name}` ({}) of `{def_name}`",
                        self.type_text(*feature),
                        self.type_text(end)
                    );
                    self.report(id, "incompatible-ends", message);
                }
            }
        } else if ends.iter().all(|(_, f)| self.kind(*f) == ElementKind::Port) {
            let ((a_text, a), (b_text, b)) = (&ends[0], &ends[1]);
            if let Some(problem) = self.port_mismatch(*a, *b) {
                let message = format!(
                    "`{a_text}` ({}) and `{b_text}` ({}) do not fit: {problem}",
                    self.type_text(*a),
                    self.type_text(*b)
                );
                self.report(id, "incompatible-ends", message);
            }
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

    /// The two port ends of an interface def must fit each other.
    fn check_interface_def_ends(&mut self, id: ElementId) {
        let ends: Vec<ElementId> = self.model.tree[id]
            .children
            .iter()
            .copied()
            .filter(|c| self.model.tree[*c].is_end && self.kind(*c) == ElementKind::Port)
            .collect();
        if let [a, b] = ends[..]
            && let Some(problem) = self.port_mismatch(a, b)
        {
            let message = format!(
                "its ends `{}` ({}) and `{}` ({}) do not fit: {problem}",
                self.model.get(a).name.clone().unwrap_or_default(),
                self.type_text(a),
                self.model.get(b).name.clone().unwrap_or_default(),
                self.type_text(b)
            );
            self.report(id, "incompatible-ends", message);
        }
    }

    fn check_subjects(&mut self, id: ElementId) {
        let subjects = self.model.tree[id]
            .children
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
        let scope = element.owner;
        let requirement = element.target.clone().and_then(|target| {
            let written = target.to_string();
            match self.model.resolve(scope, &target) {
                Err(error) => {
                    self.lookup_error(id, "the requirement", &written, error);
                    None
                }
                Ok(r) if self.unsupported_target(id, r, &written) => None,
                Ok(r) if self.kind(r) != ElementKind::Requirement => {
                    let hint = if self.kind(r) == ElementKind::RequirementDef {
                        format!(" (declare `requirement x : {written};` and satisfy that)")
                    } else {
                        String::new()
                    };
                    let message = format!(
                        "`{written}` is a {}; `satisfy` names a requirement{hint}",
                        self.kind(r).keyword()
                    );
                    self.report(id, "wrong-kind", message);
                    None
                }
                Ok(r) => Some(r),
            }
        });
        let element = &self.model.tree[id];
        let Some(by) = element.by.clone() else {
            return;
        };
        let written = by.to_string();
        let feature = match self.model.resolve_chain(scope, &by) {
            Err(error) => {
                self.lookup_error(id, "the satisfying feature", &written, error);
                return;
            }
            Ok(steps) => *steps.last().expect("a chain has steps"),
        };
        if !self.kind(feature).is_usage() {
            let message = format!(
                "`{written}` is a {}, not a feature",
                self.kind(feature).keyword()
            );
            self.report(id, "wrong-kind", message);
            return;
        }
        let Some(requirement) = requirement else {
            return;
        };
        let subject = self
            .model
            .features(requirement)
            .into_iter()
            .find(|f| self.kind(*f) == ElementKind::Subject);
        if let Some(subject) = subject {
            let expected = self.model.types_of(subject);
            let actual = self.model.types_of(feature);
            let fits = expected
                .iter()
                .all(|(t, _)| actual.iter().any(|(a, _)| self.model.specializes(*a, *t)));
            if !fits {
                let message = format!(
                    "`{written}` ({}) cannot be the subject of `{}`, whose subject `{}` is a {}",
                    self.type_text(feature),
                    self.model.describe(requirement),
                    self.model.get(subject).name.clone().unwrap_or_default(),
                    self.type_text(subject)
                );
                self.report(id, "wrong-subject", message);
            }
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

    /// Why two ports cannot be connected, if they cannot: every directed
    /// feature must meet a same-named feature of the opposite direction.
    fn port_mismatch(&self, a: ElementId, b: ElementId) -> Option<String> {
        let (items_a, items_b) = (self.directed_features(a), self.directed_features(b));
        for (name, direction, ty) in &items_a {
            let Some((_, other_direction, other_ty)) = items_b.iter().find(|(n, _, _)| n == name)
            else {
                return Some(format!("`{name}` has no counterpart on the other side"));
            };
            if *other_direction != direction.flipped() {
                return Some(format!(
                    "`{name}` is `{}` on one side and `{}` on the other; one side must send what the other receives",
                    direction.keyword(),
                    other_direction.keyword()
                ));
            }
            if ty != other_ty {
                return Some(format!("`{name}` carries different types on the two sides"));
            }
        }
        items_b
            .iter()
            .find(|(name, _, _)| !items_a.iter().any(|(n, _, _)| n == name))
            .map(|(name, _, _)| format!("`{name}` has no counterpart on the other side"))
    }

    /// (name, direction as seen from outside the port, type) of each directed
    /// feature of a port, with conjugation applied.
    fn directed_features(&self, port: ElementId) -> Vec<(String, Direction, Option<ElementId>)> {
        let conjugated = self.model.types_of(port).first().is_some_and(|(_, c)| *c);
        self.model
            .features(port)
            .into_iter()
            .filter_map(|f| {
                let element = self.model.get(f);
                let direction = element.direction?;
                let direction = if conjugated {
                    direction.flipped()
                } else {
                    direction
                };
                let ty = self.model.types_of(f).first().map(|(t, _)| *t);
                Some((element.effective_name()?.to_string(), direction, ty))
            })
            .collect()
    }

    fn type_text(&self, feature: ElementId) -> String {
        let types: Vec<String> = self
            .model
            .types_of(feature)
            .iter()
            .map(|(t, conjugated)| {
                let name = self.model.get(*t).name.clone().unwrap_or_default();
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

fn usage_fits(specific: ElementKind, general: ElementKind) -> bool {
    use ElementKind::*;
    specific == general
        || (specific, general) == (Part, Item)
        || (specific, general) == (Interface, Connection)
}

/// The definition kinds a usage may be typed by.
fn allowed_types(kind: ElementKind) -> &'static [ElementKind] {
    use ElementKind::*;
    match kind {
        Part => &[PartDef],
        Item => &[ItemDef, PartDef],
        Port => &[PortDef],
        Attribute => &[AttributeDef],
        Connection => &[ConnectionDef, InterfaceDef],
        Interface => &[InterfaceDef],
        Requirement => &[RequirementDef],
        Subject => &[
            PartDef,
            ItemDef,
            PortDef,
            AttributeDef,
            ConnectionDef,
            InterfaceDef,
            RequirementDef,
        ],
        _ => &[],
    }
}
