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
            ElementKind::Dependency => self.check_dependency(id),
            kind if kind.is_behavior() => self.check_behavior(id),
            kind if kind.is_definition() => self.check_definition(id),
            _ => self.check_usage(id),
        }
        self.check_expressions(id);
        self.check_hides_inherited(id);
        self.check_reachable(id);
    }

    // ---- dependencies, expressions, behaviour, scenarios and agents (C-50) ----

    fn check_dependency(&mut self, id: ElementId) {
        let ends = self.model.tree[id].ends.clone();
        for (reference, what) in ends.iter().zip(["the client", "the supplier"]) {
            self.follow(id, Role::End, reference, what);
        }
    }

    /// Every name in the element's expressions leads to a feature, a value
    /// of an enum def, or (after `new`) a definition with those features.
    fn check_expressions(&mut self, id: ElementId) {
        let element = &self.model.tree[id];
        let expressions: Vec<Expression> = element
            .expression
            .iter()
            .chain(&element.guard)
            .cloned()
            .collect();
        for expression in &expressions {
            let mut names = Vec::new();
            let mut news = Vec::new();
            expression.walk(&mut |e| match e {
                Expression::Name(reference) => names.push(reference.clone()),
                Expression::New { ty, arguments } => news.push((ty.clone(), arguments.clone())),
                _ => {}
            });
            for reference in names {
                let Some(target) = self.follow(id, Role::Value, &reference, "the name") else {
                    continue;
                };
                let kind = self.kind(target);
                if !(kind.is_usage() || kind == ElementKind::Accept) {
                    let message = format!(
                        "`{reference}` is a {}; a name in an expression stands for a feature or a value",
                        kind.keyword()
                    );
                    self.report(id, "wrong-kind", message);
                }
            }
            for (ty, arguments) in news {
                let Some(target) = self.follow(id, Role::Value, &ty, "the type after `new`") else {
                    continue;
                };
                if !matches!(
                    self.kind(target),
                    ElementKind::ItemDef | ElementKind::PartDef | ElementKind::AttributeDef
                ) {
                    let message = format!(
                        "`{ty}` is a {}; `new` makes an item, part or attribute value",
                        self.kind(target).keyword()
                    );
                    self.report(id, "wrong-type", message);
                    continue;
                }
                for argument in arguments {
                    self.follow(id, Role::Value, &argument.feature, "the argument");
                }
            }
        }
        // A value of an enum def is written `E::v`; a literal is no such value.
        if element.kind.is_usage() && (element.value.is_some() || element.expression.is_some()) {
            self.check_enum_value(id);
        }
    }

    fn check_enum_value(&mut self, id: ElementId) {
        let Some(&(declared, _)) = self.model.types_of(id).first() else {
            return;
        };
        if self.kind(declared) != ElementKind::EnumDef {
            return;
        }
        let element = &self.model.tree[id];
        let value = match (&element.value, &element.expression) {
            (Some(literal), _) => {
                let message = format!(
                    "`{literal}` is not a value of `{}`; write one of its values, such as `{}::…`",
                    self.name(declared),
                    self.name(declared)
                );
                self.report(id, "wrong-value", message);
                return;
            }
            (None, Some(Expression::Name(reference))) => reference.clone(),
            _ => return,
        };
        let Ok(target) = self.model.target(id, Role::Value, &value) else {
            return;
        };
        let fits = self.kind(target) == ElementKind::Enum
            && self
                .model
                .get(target)
                .owner()
                .is_some_and(|owner| self.model.specializes(owner, declared));
        if !fits {
            let message = format!(
                "`{value}` is not a value of `{}`",
                self.model.describe(declared)
            );
            self.report(id, "wrong-value", message);
        }
    }

    fn check_behavior(&mut self, id: ElementId) {
        let element = &self.model.tree[id];
        let kind = element.kind;
        let owner = element.owner();
        let owner_kind = owner.map(|o| self.kind(o));
        let misplaced = |this: &mut Self, message: &str| {
            this.report(id, "misplaced-behaviour", message.to_string());
        };
        match kind {
            ElementKind::State => {
                if element.exhibit {
                    if !matches!(owner_kind, Some(ElementKind::PartDef | ElementKind::Part)) {
                        misplaced(self, "`exhibit state` belongs in a part def or a part");
                    }
                } else if owner_kind != Some(ElementKind::State) {
                    misplaced(
                        self,
                        "a state belongs in the exhibit state of a part (or in another state)",
                    );
                }
                self.check_redefinition(id);
                self.check_initial_state(id);
            }
            ElementKind::Transition => {
                if owner_kind != Some(ElementKind::State) {
                    misplaced(self, "a transition belongs in a state machine");
                }
                let ends = element.ends.clone();
                for (reference, what) in ends.iter().zip(["the source state", "the target state"]) {
                    self.check_sibling_state(id, reference, what);
                }
            }
            ElementKind::Succession => {
                if owner_kind != Some(ElementKind::State) {
                    misplaced(
                        self,
                        "`then` a state belongs in a state machine, after `entry`",
                    );
                    return;
                }
                let siblings = self.model.tree[owner.expect("owner")].children().to_vec();
                let position = siblings.iter().position(|c| *c == id).unwrap_or(0);
                let after_entry = position > 0
                    && self.model.tree[siblings[position - 1]].state_action
                        == Some(StateAction::Entry);
                if !after_entry {
                    self.report(
                        id,
                        "unsupported",
                        "`then` a state after something other than `entry` is a transition written in short; write it as `transition first … then …`".into(),
                    );
                }
                if let Some(target) = element.target.clone() {
                    self.check_sibling_state(id, &target, "the first state");
                }
            }
            ElementKind::Objective => {
                if owner_kind != Some(ElementKind::VerificationDef) {
                    misplaced(
                        self,
                        "an objective belongs in a verification def (a scenario)",
                    );
                }
            }
            ElementKind::Verify => {
                if owner_kind != Some(ElementKind::Objective) {
                    misplaced(self, "`verify` belongs in the objective of a scenario");
                }
                if let Some(target) = element.target.clone()
                    && let Some(requirement) =
                        self.follow(id, Role::Target, &target, "the verified requirement")
                    && self.kind(requirement) != ElementKind::Requirement
                {
                    let hint = if self.kind(requirement) == ElementKind::RequirementDef {
                        format!(" (declare `requirement x : {target};` and verify that)")
                    } else {
                        String::new()
                    };
                    let message = format!(
                        "`{target}` is a {}; `verify` names a requirement{hint}",
                        self.kind(requirement).keyword()
                    );
                    self.report(id, "wrong-kind", message);
                }
            }
            ElementKind::AssertConstraint => {
                if self.scenario_of(id).is_none() {
                    misplaced(self, "a check (`assert constraint`) belongs in a scenario");
                }
            }
            ElementKind::Assign => {
                self.check_step_place(id);
                if let Some(target) = element.target.clone()
                    && let Some(feature) =
                        self.follow(id, Role::Target, &target, "the assigned feature")
                    && !matches!(
                        self.kind(feature),
                        ElementKind::Attribute | ElementKind::Reference | ElementKind::Item
                    )
                {
                    let message = format!(
                        "`{target}` is a {}; `assign` sets an attribute",
                        self.kind(feature).keyword()
                    );
                    self.report(id, "wrong-kind", message);
                }
            }
            ElementKind::Send => {
                self.check_step_place(id);
                self.check_port_flow(id, true);
            }
            ElementKind::Accept => {
                if owner_kind != Some(ElementKind::Transition) {
                    self.check_step_place(id);
                }
                if !element.after {
                    self.check_typing(id);
                    self.check_port_flow(id, false);
                }
            }
            ElementKind::Action | ElementKind::If => self.check_step_place(id),
            _ => {}
        }
    }

    /// The scenario (verification def) an element is a step of, if any.
    fn scenario_of(&self, id: ElementId) -> Option<ElementId> {
        let mut current = self.model.get(id).owner();
        while let Some(owner) = current {
            match self.kind(owner) {
                ElementKind::VerificationDef => return Some(owner),
                ElementKind::Action | ElementKind::If => current = self.model.get(owner).owner(),
                _ => return None,
            }
        }
        None
    }

    /// An action node belongs in a state (as its entry, do or exit action),
    /// a transition (as its effect), an action or `if` branch, or a scenario.
    fn check_step_place(&mut self, id: ElementId) {
        let element = &self.model.tree[id];
        let owner_kind = element.owner().map(|o| self.kind(o));
        let fits = match (element.state_action, owner_kind) {
            (Some(_), Some(ElementKind::State)) => true,
            (Some(_), _) => false,
            (None, Some(kind)) => matches!(
                kind,
                ElementKind::Transition
                    | ElementKind::Action
                    | ElementKind::If
                    | ElementKind::VerificationDef
            ),
            (None, None) => false,
        };
        if !fits {
            let message = if element.state_action.is_some() {
                "`entry`, `do` and `exit` actions belong in a state"
            } else {
                "an action step belongs in a state's action, a transition, an action or a scenario"
            };
            self.report(id, "misplaced-behaviour", message.into());
        }
    }

    /// A transition's source or target, or the first state, is a state of
    /// the same state machine.
    fn check_sibling_state(&mut self, id: ElementId, reference: &Reference, what: &str) {
        let Some(state) = self.follow(id, Role::End, reference, what) else {
            return;
        };
        let machine = self.model.get(id).owner();
        if self.kind(state) != ElementKind::State || self.model.get(state).owner() != machine {
            let message = format!(
                "{what} `{reference}` is not a state of `{}`",
                machine.map(|m| self.model.describe(m)).unwrap_or_default()
            );
            self.report(id, "wrong-kind", message);
        }
    }

    /// A state with states inside says which comes first: `entry; then S;`.
    fn check_initial_state(&mut self, id: ElementId) {
        let children = self.model.tree[id].children().to_vec();
        let has_states = children
            .iter()
            .any(|c| self.model.tree[*c].kind == ElementKind::State);
        let initials = children
            .iter()
            .filter(|c| self.model.tree[**c].kind == ElementKind::Succession)
            .count();
        if has_states && initials == 0 {
            self.report(
                id,
                "initial-state",
                "it has states but does not say which comes first; add `entry; then` the first state".into(),
            );
        } else if initials > 1 {
            self.report(
                id,
                "initial-state",
                format!("it says {initials} states come first; keep one `entry; then …`"),
            );
        }
    }

    /// Items cross a port the way its directed features allow. `sending`: a
    /// `send` (else an `accept`). Inside a part the part sends through `out`
    /// features and receives through `in` features; a scenario stands outside
    /// its subject, so there it is the other way round.
    fn check_port_flow(&mut self, id: ElementId, sending: bool) {
        let element = &self.model.tree[id];
        let Some(via) = element.via.clone() else {
            if sending {
                self.report(
                    id,
                    "missing-port",
                    "`send` needs the port it sends through: `via port`".into(),
                );
            } else if self.kind(element.owner().unwrap_or(id)) != ElementKind::Transition
                || element.typed_by.is_empty()
            {
                self.report(
                    id,
                    "missing-port",
                    "`accept` needs the port it accepts from: `via port`".into(),
                );
            }
            return;
        };
        let Some(port) = self.follow(id, Role::Via, &via, "the port") else {
            return;
        };
        if self.kind(port) != ElementKind::Port {
            let message = format!(
                "`{via}` is a {}; `via` names a port",
                self.kind(port).keyword()
            );
            self.report(id, "wrong-kind", message);
            return;
        }
        // The item's type: the payload's declared type, or what `new` makes,
        // or the type of a sent feature.
        let item = if sending {
            match &element.expression {
                Some(Expression::New { ty, .. }) => self.model.target(id, Role::Value, ty).ok(),
                Some(Expression::Name(reference)) => self
                    .model
                    .target(id, Role::Value, reference)
                    .ok()
                    .and_then(|f| self.model.types_of(f).first().map(|(t, _)| *t)),
                _ => None,
            }
        } else {
            element
                .typed_by
                .first()
                .and_then(|r| self.model.target(id, Role::TypedBy, r).ok())
        };
        let Some(item) = item else {
            return;
        };
        let outside = self.scenario_of(id).is_some();
        let wanted = if sending != outside {
            Direction::Out
        } else {
            Direction::In
        };
        let carried = self
            .directed_features(port)
            .into_iter()
            .any(|(_, direction, ty)| {
                (direction == wanted || direction == Direction::InOut)
                    && ty.is_none_or(|ty| {
                        self.model.specializes(item, ty) || self.model.specializes(ty, item)
                    })
            });
        if !carried {
            let (code, verb) = if sending {
                ("incompatible-send", "sends")
            } else {
                ("incompatible-trigger", "receives")
            };
            let message = format!(
                "`{via}` ({}) has no `{}` item for `{}`, so nothing {verb} it through this port",
                self.type_text(port),
                wanted.keyword(),
                self.name(item)
            );
            self.report(id, code, message);
        }
    }

    /// An agent's fallback shares its contract and is not itself an agent
    /// (`wrong-fallback`, `agent-fallback`; ROADMAP §4.11).
    fn check_agent(&mut self, id: ElementId) {
        let agent_name = QualifiedName::new(["Agents", "Agent"]);
        let Ok(agent) = self.model.resolve_global(&agent_name) else {
            return;
        };
        if id == agent || !self.model.specializes(id, agent) {
            return;
        }
        let fallback_name = QualifiedName::new(["Agents", "Agent", "fallback"]);
        let Ok(fallback) = self.model.resolve_global(&fallback_name) else {
            return;
        };
        let Some(feature) = self
            .model
            .features(id)
            .into_iter()
            .find(|f| *f == fallback || self.redefines_feature(*f, fallback))
        else {
            return;
        };
        let at = if self.model.tree.contains(feature) && self.model.get(feature).owner() == Some(id)
        {
            feature
        } else {
            id
        };
        for (ty, _) in self.model.types_of(feature) {
            if self.model.specializes(ty, agent) {
                let message = format!(
                    "the fallback `{}` is itself an agent; a fallback is a deterministic part that takes over when the agent fails",
                    self.name(ty)
                );
                self.report(at, "agent-fallback", message);
                continue;
            }
            let contracts: Vec<ElementId> = self
                .model
                .generals(id)
                .iter()
                .copied()
                .filter(|g| *g != agent && !self.model.specializes(*g, agent))
                .filter(|g| self.kind(*g) == ElementKind::PartDef)
                .collect();
            for contract in contracts {
                if !self.model.specializes(ty, contract) {
                    let message = format!(
                        "the fallback `{}` does not share the agent's contract `{}`; it must specialise it too",
                        self.name(ty),
                        self.model.describe(contract)
                    );
                    self.report(at, "wrong-fallback", message);
                }
            }
        }
    }

    /// Does `feature` redefine `target`, directly or through other redefinitions?
    fn redefines_feature(&self, feature: ElementId, target: ElementId) -> bool {
        let mut stack = vec![feature];
        let mut seen = Vec::new();
        while let Some(next) = stack.pop() {
            if seen.contains(&next) {
                continue;
            }
            seen.push(next);
            for redefined in self.model.redefined(next).iter() {
                if *redefined == target {
                    return true;
                }
                stack.push(*redefined);
            }
        }
        false
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
            self.check_agent(id);
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
                Some(
                    ElementKind::RequirementDef
                        | ElementKind::Requirement
                        | ElementKind::VerificationDef
                )
            )
        {
            self.report(
                id,
                "misplaced-subject",
                "a subject belongs in a requirement or a scenario".into(),
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
        self.check_binding(id);
        if kind == ElementKind::Enum
            && self.model.tree[id].owner().map(|o| self.kind(o)) == Some(ElementKind::EnumDef)
            && (!self.model.tree[id].typed_by.is_empty()
                || self.model.tree[id].value.is_some()
                || self.model.tree[id].expression.is_some())
        {
            self.report(
                id,
                "wrong-value",
                "a value of an enum def has no type or value of its own".into(),
            );
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

    /// The features `id` redefines, directly or indirectly, nearest first.
    fn redefined_closure(&self, id: ElementId) -> Vec<ElementId> {
        let mut out: Vec<ElementId> = Vec::new();
        let mut i = 0;
        let mut next = vec![id];
        while i < next.len() {
            for redefined in self.model.redefined(next[i]).iter() {
                if *redefined != id && !out.contains(redefined) {
                    out.push(*redefined);
                    next.push(*redefined);
                }
            }
            i += 1;
        }
        out
    }

    /// `Part` or `Item` when `id` is a part or item usage, or a usage
    /// without a kind keyword that redefines one (directly or indirectly);
    /// `None` for anything else.
    pub(crate) fn part_kind(&self, id: ElementId) -> Option<ElementKind> {
        match self.kind(id) {
            kind @ (ElementKind::Part | ElementKind::Item) => Some(kind),
            ElementKind::Reference => self
                .redefined_closure(id)
                .into_iter()
                .map(|r| self.kind(r))
                .find(|k| matches!(k, ElementKind::Part | ElementKind::Item)),
            _ => None,
        }
    }

    /// Whether a usage is referential as written (SysML 7.6.3,
    /// `validateUsageIsReferential`): `ref`, no kind keyword, directed, an
    /// `end`, or without a featuring type (owned by a package or at the top
    /// of a document).
    fn referential_itself(&self, id: ElementId) -> bool {
        let element = self.model.get(id);
        element.referential
            || element.kind == ElementKind::Reference
            || element.direction.is_some()
            || element.is_end
            || element
                .owner()
                .is_none_or(|owner| self.kind(owner) == ElementKind::Package)
    }

    /// The nearest part or item usage `id` redefines that is composite as
    /// written: it makes `id` composite too, since a redefinition has the
    /// values of what it redefines (KerML 8.3.3.3.8).
    fn composite_redefined(&self, id: ElementId) -> Option<ElementId> {
        self.redefined_closure(id).into_iter().find(|r| {
            matches!(self.kind(*r), ElementKind::Part | ElementKind::Item)
                && !self.referential_itself(*r)
        })
    }

    /// Whether a part or item usage refers to its value or contains it:
    /// `Some((true, kind))` when it is referential itself (`ref`, no kind
    /// keyword, directed, an `end`, or without a featuring type) and so is
    /// every part or item usage it redefines; `Some((false, kind))` for a
    /// composite one. `kind` is `Part` or `Item` ([`Checker::part_kind`]).
    /// `None` for anything else.
    pub(crate) fn part_binding(&self, id: ElementId) -> Option<(bool, ElementKind)> {
        let kind = self.part_kind(id)?;
        let referential = self.referential_itself(id) && self.composite_redefined(id).is_none();
        Some((referential, kind))
    }

    /// The usage whose value `id` has: itself when it has a value, else the
    /// nearest feature it redefines that has one (a redefinition has the
    /// values of what it redefines).
    pub(crate) fn value_holder(&self, id: ElementId) -> Option<ElementId> {
        let has_value = |e: &Element| e.value.is_some() || e.expression.is_some();
        if has_value(self.model.get(id)) {
            return Some(id);
        }
        self.redefined_closure(id)
            .into_iter()
            .find(|r| has_value(self.model.get(*r)))
    }

    /// The usage a value written as a feature chain names, if it does.
    fn named_target(&self, holder: ElementId) -> Option<(Reference, ElementId)> {
        let element = self.model.get(holder);
        let (None, Some(Expression::Name(reference))) = (&element.value, &element.expression)
        else {
            return None;
        };
        let target = self.model.target(holder, Role::Value, reference).ok()?;
        Some((reference.clone(), target))
    }

    /// The value of a part or item usage binds it (SysML 7.13.4). A
    /// referential usage refers to a part (or item) usage that exists
    /// elsewhere, so its value names one whose types fit; a binding is
    /// never changed in a redefinition. A composite usage is not bound to
    /// another part (SysML 7.6.3; deviation 19 for one of the same owner).
    fn check_binding(&mut self, id: ElementId) {
        let Some((referential, kind)) = self.part_binding(id) else {
            return;
        };
        let noun = kind.keyword();
        let own = {
            let element = &self.model.tree[id];
            element.value.is_some() || element.expression.is_some()
        };
        if own
            && let Some(bound) = self
                .redefined_closure(id)
                .into_iter()
                .find(|r| self.value_holder(*r) == Some(*r))
        {
            let message = format!(
                "`{}` is already bound where it is declared (in `{}`); a redefinition cannot bind it again (KerML `validateFeatureValueOverriding`)",
                self.name(bound),
                self.model
                    .get(bound)
                    .owner()
                    .map(|o| self.model.describe(o))
                    .unwrap_or_default()
            );
            self.report(id, "wrong-value", message);
            return;
        }
        let Some(holder) = self.value_holder(id) else {
            return;
        };
        let named = self.named_target(holder);
        if !referential {
            let Some((reference, target)) = named else {
                return;
            };
            let Some(target_kind) = self.part_kind(target) else {
                return;
            };
            let target_noun = target_kind.keyword();
            let message = if let Some(composite) = self.composite_redefined(id)
                && self.referential_itself(id)
            {
                // Written as a reference, but what it redefines is composite.
                format!(
                    "`{}` is composite in `{}`; declare it `ref {noun}` there",
                    self.name(composite),
                    self.model
                        .get(composite)
                        .owner()
                        .map(|o| self.name(o))
                        .unwrap_or_default()
                )
            } else if holder != id {
                format!(
                    "it redefines `{}`, which is bound to `{reference}`: as a composite {noun} it would hold that {target_noun} as its own; declare it `ref {noun}`",
                    self.name(holder)
                )
            } else if self.same_owner(id, &reference, target) {
                format!(
                    "`{reference}` is a {target_noun} of the same owner: bound to it, this composite {noun} would be that {target_noun} under a second name; declare it `ref {noun}` (deviation 19)"
                )
            } else {
                format!(
                    "a composite {noun}'s value cannot be a {target_noun} of another owner (SysML 7.6.3); declare it `ref {noun}` to refer to `{reference}`"
                )
            };
            self.report(id, "wrong-value", message);
            return;
        }
        if holder != id {
            return; // checked where it is written
        }
        let element = &self.model.tree[id];
        match (&element.value, &element.expression) {
            (Some(literal), _) => {
                let message = format!(
                    "`{literal}` is a data value; a referential {noun} refers to a {noun} usage, such as `= bus` or `= power.bus`"
                );
                self.report(id, "wrong-value", message);
            }
            (None, Some(Expression::Name(_))) => self.check_referent(id, kind, named),
            (None, Some(Expression::New { ty, .. })) if kind == ElementKind::Item => {
                let ty = ty.clone();
                if let Ok(made) = self.model.target(id, Role::Value, &ty)
                    && !self
                        .model
                        .types_of(id)
                        .iter()
                        .all(|(expected, _)| self.model.specializes(made, *expected))
                {
                    let message = format!(
                        "`new {ty}` makes a {}, which is not a {}",
                        self.name(made),
                        self.type_text(id)
                    );
                    self.report(id, "wrong-value", message);
                }
            }
            // A ref item's value may be any expression: it is a value.
            (None, Some(_)) if kind == ElementKind::Item => {}
            (None, Some(_)) => {
                let message = "a referential part bound to an expression other than a feature chain (such as `new …`) is not supported; name the part usage it refers to, such as `= bus`".to_string();
                self.report(id, "unsupported", message);
            }
            (None, None) => {}
        }
    }

    /// The usage a referential usage's value names: a part (for `ref item`,
    /// a part or item) usage whose types specialise every type of the
    /// reference, and not the reference itself, directly or through other
    /// references.
    fn check_referent(
        &mut self,
        id: ElementId,
        kind: ElementKind,
        named: Option<(Reference, ElementId)>,
    ) {
        let noun = kind.keyword();
        // A name that does not resolve is reported where names are checked.
        let Some((reference, target)) = named else {
            return;
        };
        let target_kind = self.part_kind(target);
        let fits_kind = match kind {
            ElementKind::Item => target_kind.is_some(),
            _ => target_kind == Some(ElementKind::Part),
        };
        if !fits_kind {
            let message = format!(
                "`{reference}` is a {}; a referential {noun} refers to a {noun} usage",
                target_kind.unwrap_or(self.kind(target)).keyword()
            );
            self.report(id, "wrong-value", message);
            return;
        }
        // Through other references, the binding must reach a part, not
        // come back to this one.
        let mut seen = vec![id];
        let mut current = target;
        loop {
            if current == id {
                let message = if target == id {
                    format!(
                        "`{}` is bound to itself, so it refers to no part",
                        self.name(id)
                    )
                } else {
                    format!(
                        "`{}` and `{reference}` are bound to each other, so neither refers to a part",
                        self.name(id)
                    )
                };
                self.report(id, "wrong-value", message);
                return;
            }
            if seen.contains(&current)
                || !self
                    .part_binding(current)
                    .is_some_and(|(referential, _)| referential)
            {
                break; // a part, or a cycle reported where it is
            }
            let Some((_, next)) = self
                .value_holder(current)
                .and_then(|h| self.named_target(h))
            else {
                break;
            };
            seen.push(current);
            current = next;
        }
        let actual = self.model.types_of(target);
        let fits = self.model.types_of(id).iter().all(|(expected, _)| {
            actual
                .iter()
                .any(|(a, _)| self.model.specializes(*a, *expected))
        });
        if !fits {
            let message = format!(
                "`{reference}` ({}) cannot be what this {noun} refers to: it is not a {}",
                self.type_text(target),
                self.type_text(id)
            );
            self.report(id, "wrong-value", message);
        }
    }

    /// Whether `target`, named by a one-step value of `id`, is a feature of
    /// `id`'s own owner (the same featuring instance).
    fn same_owner(&self, id: ElementId, reference: &Reference, target: ElementId) -> bool {
        let Some(owner) = self.model.get(id).owner() else {
            return false;
        };
        reference.steps.len() == 1
            && (self.model.get(target).owner() == Some(owner)
                || self.model.is_inherited(owner, target))
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
        // What is inside is decided by what each reference is bound to; a
        // part reached through an unbound one is outside.
        let part = |steps: &[ElementId]| self.bound_chain(owner, &steps[..steps.len() - 1]);
        let (Some(part_a), Some(part_b)) = (part(a), part(b)) else {
            return None;
        };
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

    /// A chain of part features from `owner` with each referential step
    /// replaced by the chain its value leads through, from the level of the
    /// chain where the value's first feature is found (as a name is looked
    /// up outward); `None` when a step is a reference that is not bound, or
    /// is bound to something outside the chain's levels.
    fn bound_chain(&self, owner: Option<ElementId>, steps: &[ElementId]) -> Option<Vec<ElementId>> {
        // A usage without a featuring type (owned by a package) is where a
        // chain written there starts, not a reference to replace.
        let featured = |s: ElementId| {
            self.model
                .get(s)
                .owner()
                .is_some_and(|o| self.kind(o) != ElementKind::Package)
        };
        let mut chain = steps.to_vec();
        for _ in 0..16 {
            let Some(i) = chain.iter().position(|s| {
                featured(*s)
                    && self
                        .part_binding(*s)
                        .is_some_and(|(referential, _)| referential)
            }) else {
                return Some(chain);
            };
            let holder = self.value_holder(chain[i])?;
            let reference = match &self.model.get(holder).expression {
                Some(Expression::Name(reference)) => reference,
                _ => return None,
            };
            let value = self
                .model
                .resolve_reference(holder, Role::Value, reference)
                .ok()?;
            let first = *value.first()?;
            let level = (0..=i).rev().find(|k| {
                let namespace = if *k == 0 { owner } else { Some(chain[k - 1]) };
                namespace.is_some_and(|n| self.model.features(n).contains(&first))
            })?;
            let mut replaced = chain[..level].to_vec();
            replaced.extend(value);
            replaced.extend_from_slice(&chain[i + 1..]);
            chain = replaced;
        }
        None
    }

    /// A part def must not contain itself through required composite parts
    /// (lower bound of at least 1; a part without a multiplicity is
    /// required). A referential part contains nothing: it may refer to a
    /// part of its own owner's kind.
    fn check_composition(&mut self, id: ElementId) {
        let mut seen = vec![id];
        let mut i = 0;
        while i < seen.len() {
            for feature in self.model.features(seen[i]) {
                let element = self.model.get(feature);
                let required = element.multiplicity.is_none_or(|m| m.lower > 0);
                if self.part_binding(feature) != Some((false, ElementKind::Part)) || !required {
                    continue;
                }
                for (ty, _) in self.model.types_of(feature) {
                    if ty == id {
                        // Written `ref`, it is composite through what it redefines.
                        let why = match self.composite_redefined(feature) {
                            Some(composite) if self.referential_itself(feature) => format!(
                                " (it redefines the composite `{}`, so it is composite too)",
                                self.model.describe(composite)
                            ),
                            _ => String::new(),
                        };
                        let message = format!(
                            "it contains itself through the required part `{}`{why}; give that part a lower bound of 0",
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
    pub(crate) fn directed_features(
        &self,
        port: ElementId,
    ) -> Vec<(String, Direction, Option<ElementId>)> {
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
        EnumDef,
    ];
    match kind {
        Part => &[PartDef],
        Item => &[ItemDef, PartDef],
        Port => &[PortDef],
        Attribute => &[AttributeDef, EnumDef],
        Enum => &[EnumDef],
        // What an `accept` takes in: an item, a part or a value.
        Accept => &[ItemDef, PartDef, AttributeDef, EnumDef],
        Connection => &[ConnectionDef, InterfaceDef],
        Interface => &[InterfaceDef],
        Requirement => &[RequirementDef],
        Subject | Reference => ANY,
        _ => &[],
    }
}
