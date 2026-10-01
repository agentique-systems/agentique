//! What the printer writes so that the parser reads it back as the same
//! element. Code that builds or changes elements (the System State) checks
//! them with [`writable`]; the tests hold these rules to the printer and the
//! parser for every kind and field.

use crate::parser::{Source, parse_into};
use crate::printer::print_element;
use crate::tree::*;

/// A part of an element's text form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    /// The declared name.
    Name,
    Visibility,
    /// `abstract`
    Abstract,
    /// `end`
    End,
    /// `in`, `out`, `inout`
    Direction,
    /// `: T`
    TypedBy,
    /// `: ~T`
    Conjugated,
    /// `:>`
    Specializes,
    /// `:>>`
    Redefines,
    /// `[1..*]`
    Multiplicity,
    /// `= value`
    Value,
    /// `connect a to b`; a transition's `first S ... then T`; a
    /// dependency's `from A to B`
    Ends,
    /// The imported name, the requirement a `satisfy` names or a `verify`
    /// verifies, the state after `then`, the feature an `assign` sets.
    Target,
    /// `import A::*`
    Wildcard,
    /// `satisfy R by x`
    By,
    /// The text of a doc or comment.
    Text,
    /// Members in a body `{ ... }`, such as a doc comment.
    Members,
    /// `= expression`, the payload of `send`, the value of `assign`, the
    /// condition of `if`, the duration of `accept after`, the constraint of
    /// an `assert` (C-50).
    Expression,
    /// A transition's `if guard`.
    Guard,
    /// `via port` of a `send` or `accept`.
    Via,
    /// `exhibit state`
    Exhibit,
    /// `entry`, `do`, `exit` before a state's action.
    StateAction,
    /// `accept after`
    After,
}

impl Field {
    /// Whether elements of `kind` can have this field in text.
    pub fn fits(self, kind: ElementKind) -> bool {
        use ElementKind::*;
        // Steps and parts of behaviour carry only their own fields below.
        let plain = kind.is_definition() || kind.is_usage();
        match self {
            Field::Name => {
                plain
                    || matches!(
                        kind,
                        Package
                            | Dependency
                            | State
                            | Transition
                            | Action
                            | Accept
                            | Objective
                            | AssertConstraint
                    )
            }
            Field::Visibility => {
                plain
                    || matches!(
                        kind,
                        Package | Import | Satisfy | Dependency | State | Transition
                    )
            }
            Field::Abstract | Field::Specializes => plain,
            // `end x;` without a kind keyword reads back as a port, or not at all.
            Field::End => kind.is_usage() && kind != Reference,
            Field::Direction | Field::Conjugated | Field::Multiplicity | Field::Value => {
                kind.is_usage()
            }
            Field::TypedBy => kind.is_usage() || kind == Accept,
            Field::Redefines => kind.is_usage() || kind == State,
            Field::Ends => matches!(kind, Connection | Interface | Transition | Dependency),
            Field::Target => matches!(kind, Import | Satisfy | Succession | Verify | Assign),
            Field::Wildcard => kind == Import,
            Field::By => kind == Satisfy,
            Field::Text => matches!(kind, Doc | Comment),
            Field::Members => kind.is_namespace() || matches!(kind, Satisfy | Dependency),
            Field::Expression => {
                kind.is_usage() || matches!(kind, Send | Assign | If | Accept | AssertConstraint)
            }
            Field::Guard => kind == Transition,
            Field::Via => matches!(kind, Send | Accept),
            Field::Exhibit => kind == State,
            Field::StateAction => matches!(kind, Action | Send | Assign),
            Field::After => kind == Accept,
        }
    }

    /// The field's name in messages.
    pub fn label(self) -> &'static str {
        match self {
            Field::Name => "name",
            Field::Visibility => "visibility",
            Field::Abstract => "`abstract`",
            Field::End => "`end`",
            Field::Direction => "direction",
            Field::TypedBy => "type (`:`)",
            Field::Conjugated => "conjugated type (`~`)",
            Field::Specializes => "specialisation or subsetting (`:>`)",
            Field::Redefines => "redefinition (`:>>`)",
            Field::Multiplicity => "multiplicity",
            Field::Value => "value (`=`)",
            Field::Ends => "connection end",
            Field::Target => "target",
            Field::Wildcard => "`::*`",
            Field::By => "`by` feature",
            Field::Text => "comment text",
            Field::Members => "members",
            Field::Expression => "expression",
            Field::Guard => "guard (`if`)",
            Field::Via => "port (`via`)",
            Field::Exhibit => "`exhibit`",
            Field::StateAction => "`entry`, `do` or `exit`",
            Field::After => "time trigger (`after`)",
        }
    }
}

/// Checks that the printer can write element `id` (without its members) in
/// its place, and the parser read it back as the same element. The reason is
/// plain language, such as "a part def has no type (`:`)". Unsupported text
/// and syntax errors are kept verbatim and always writable.
pub fn writable(tree: &Tree, id: ElementId) -> Result<(), String> {
    use ElementKind::*;
    let e = tree
        .get(id)
        .ok_or_else(|| format!("element {id} does not exist"))?;
    let kind = e.kind;
    if let Some(owner) = e.owner() {
        if !Field::Members.fits(tree[owner].kind) {
            return Err(no(tree[owner].kind, Field::Members));
        }
        place(tree, id, e, owner)?;
    }
    if matches!(kind, Unsupported | SyntaxError) {
        return Ok(());
    }
    if let Some(field) = fields(e).into_iter().find(|field| !field.fits(kind)) {
        return Err(no(kind, field));
    }
    if e.note.is_some() {
        return Err(format!("{} has no note", a(kind)));
    }
    match &e.name {
        Some(name) => check_name(name)?,
        None if kind == Package || kind.is_definition() || kind == Enum => {
            return Err(format!("{} needs a name", a(kind)));
        }
        None => {}
    }
    // The parser starts a usage without a kind keyword only at a name, `:>`
    // or `:>>`, and the printer writes a type before those.
    if kind == Reference
        && e.name.is_none()
        && (!e.typed_by.is_empty() || e.specializes.is_empty() && e.redefines.is_empty())
    {
        return Err(
            "a usage without a kind keyword needs a name, or a subsetting (`:>`) or redefinition (`:>>`) and no type"
                .into(),
        );
    }
    if e.target.is_none() && kind == Import {
        return Err("an import needs the name it imports".into());
    }
    if e.target.is_none() && kind == Satisfy {
        return Err("a satisfy needs the requirement it satisfies".into());
    }
    if e.conjugated && e.typed_by.len() != 1 {
        return Err("a conjugated usage (`~`) has exactly one type".into());
    }
    let ends = match kind {
        Transition | Dependency => 2..=2,
        _ => 0..=2,
    };
    if !ends.contains(&e.ends.len()) || e.ends.len() == 1 {
        return Err(format!(
            "{} has two ends{}, not {}",
            a(kind),
            if ends.start() == &0 { " (or none)" } else { "" },
            e.ends.len()
        ));
    }
    if (e.value.is_some() || e.expression.is_some()) && !e.ends.is_empty() && kind.is_usage() {
        return Err(format!(
            "{} with ends (`connect`) cannot have a value",
            a(kind)
        ));
    }
    if e.value.is_some() && e.expression.is_some() {
        return Err(format!(
            "{} has one value: a literal or an expression, not both",
            a(kind)
        ));
    }
    behaviour(e)?;
    for (role, reference) in e.references() {
        check_reference(kind, role, reference)?;
    }
    if let Some(literal) = &e.value {
        check_literal(literal)?;
    }
    for expression in e.expression.iter().chain(&e.guard) {
        check_expression(expression)?;
    }
    if matches!(kind, Doc | Comment) {
        check_text(e)?;
    }
    Ok(())
}

/// The fields behaviour elements need, and the combinations they allow.
fn behaviour(e: &Element) -> Result<(), String> {
    use ElementKind::*;
    let kind = e.kind;
    let needs = |present: bool, what: &str| {
        if present {
            Ok(())
        } else {
            Err(format!("{} needs {what}", a(kind)))
        }
    };
    match kind {
        Succession => needs(e.target.is_some(), "the state it leads to"),
        Verify => needs(e.target.is_some(), "the requirement it verifies"),
        Send => needs(e.expression.is_some(), "what it sends"),
        Assign => {
            needs(e.target.is_some(), "the feature it sets")?;
            needs(e.expression.is_some(), "the value it sets")
        }
        If => needs(e.expression.is_some(), "a condition"),
        AssertConstraint => needs(e.expression.is_some(), "the constraint it checks"),
        Accept if e.after => {
            needs(e.expression.is_some(), "a duration after `after`")?;
            if e.name.is_some() || !e.typed_by.is_empty() || e.via.is_some() {
                return Err("`accept after` waits for a time and accepts no payload".into());
            }
            Ok(())
        }
        Accept => {
            if e.expression.is_some() {
                return Err("an accept with a payload has no expression".into());
            }
            if e.typed_by.len() != 1 {
                return Err(
                    "an accept names one type for its payload, as in `accept x : T`".into(),
                );
            }
            Ok(())
        }
        State if !e.typed_by.is_empty() || e.multiplicity.is_some() => {
            Err("a state has no type or multiplicity".into())
        }
        _ => Ok(()),
    }
}

/// Members that only certain owners can print: the branches of an `if`,
/// the trigger and effect of a transition, the doc of a constraint.
fn place(tree: &Tree, id: ElementId, e: &Element, owner: ElementId) -> Result<(), String> {
    use ElementKind::*;
    let owner_element = &tree[owner];
    let position = owner_element.children().iter().position(|c| *c == id);
    let siblings = |kind: fn(&Element) -> bool| {
        owner_element
            .children()
            .iter()
            .filter(|c| **c != id && kind(&tree[**c]))
            .count()
    };
    match owner_element.kind {
        If => {
            let fits = match position {
                Some(0) => e.kind == Action && e.name.is_none() && e.state_action.is_none(),
                Some(1) => {
                    (e.kind == Action && e.name.is_none() && e.state_action.is_none())
                        || e.kind == If
                }
                _ => false,
            };
            if !fits {
                return Err(
                    "an `if` has a `then` branch and an optional `else` branch or `else if`, each an unnamed action"
                        .into(),
                );
            }
        }
        Transition => match e.kind {
            Accept if siblings(|s| s.kind == Accept) == 0 => {}
            Send | Assign | Action
                if e.state_action.is_none()
                    && siblings(|s| s.kind.is_action_node() && s.kind != Accept) == 0 => {}
            Doc | Comment => {}
            _ => {
                return Err(
                    "a transition has one trigger (`accept`), one effect (`do`) and comments"
                        .into(),
                );
            }
        },
        Send | Assign => {
            if owner_element
                .owner()
                .is_some_and(|o| tree[o].kind == Transition)
            {
                return Err(
                    "a transition's effect is written in one line and has no members".into(),
                );
            }
        }
        AssertConstraint if !matches!(e.kind, Doc | Comment) => {
            return Err("a constraint's members are its doc and comments".into());
        }
        _ => {}
    }
    Ok(())
}

/// The fields an element has that are not empty or default.
fn fields(e: &Element) -> Vec<Field> {
    [
        (e.name.is_some(), Field::Name),
        (e.visibility != Visibility::Public, Field::Visibility),
        (e.is_abstract, Field::Abstract),
        (e.is_end, Field::End),
        (e.direction.is_some(), Field::Direction),
        (!e.typed_by.is_empty(), Field::TypedBy),
        (e.conjugated, Field::Conjugated),
        (!e.specializes.is_empty(), Field::Specializes),
        (!e.redefines.is_empty(), Field::Redefines),
        (e.multiplicity.is_some(), Field::Multiplicity),
        (e.value.is_some(), Field::Value),
        (!e.ends.is_empty(), Field::Ends),
        (e.target.is_some(), Field::Target),
        (e.wildcard, Field::Wildcard),
        (e.by.is_some(), Field::By),
        (e.text.is_some(), Field::Text),
        (!e.children().is_empty(), Field::Members),
        (e.expression.is_some(), Field::Expression),
        (e.guard.is_some(), Field::Guard),
        (e.via.is_some(), Field::Via),
        (e.exhibit, Field::Exhibit),
        (e.state_action.is_some(), Field::StateAction),
        (e.after, Field::After),
    ]
    .into_iter()
    .filter_map(|(present, field)| present.then_some(field))
    .collect()
}

/// A name is not empty and reads back the same (it is quoted when needed).
fn check_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("a name cannot be empty".into());
    }
    let text = format!("part {};", QualifiedName::new([name]));
    match read_back(&text) {
        Some(e) if e.kind == ElementKind::Part && e.name.as_deref() == Some(name) => Ok(()),
        _ => Err(format!(
            "the name {name:?} cannot be written: names cannot contain line breaks"
        )),
    }
}

/// Feature chains (`a.b`) are written only where the text allows them:
/// connection ends, `by`, `via`, names in expressions and the feature an
/// `assign` sets. Every step names something.
fn check_reference(kind: ElementKind, role: Role, reference: &Reference) -> Result<(), String> {
    let field = match role {
        Role::TypedBy => Field::TypedBy,
        Role::Specializes => Field::Specializes,
        Role::Redefines => Field::Redefines,
        Role::End => Field::Ends,
        Role::Target => Field::Target,
        Role::By => Field::By,
        Role::Via => Field::Via,
        Role::Value => Field::Expression,
    };
    if reference.steps.is_empty() {
        return Err(format!("the {} needs a name", field.label()));
    }
    for step in &reference.steps {
        if step.name.segments.is_empty() {
            return Err(format!("the {} needs a name", field.label()));
        }
        for segment in &step.name.segments {
            check_name(segment).map_err(|reason| format!("in the {}, {reason}", field.label()))?;
        }
    }
    let chains = match role {
        Role::End => !matches!(kind, ElementKind::Dependency),
        Role::By | Role::Via | Role::Value => true,
        Role::Target => kind == ElementKind::Assign,
        _ => false,
    };
    if reference.steps.len() > 1 && !chains {
        return Err(format!(
            "the {} names an element, not a feature chain like `{reference}`",
            field.label()
        ));
    }
    if reference.steps.len() > 1
        && reference.steps[1..]
            .iter()
            .any(|step| step.name.segments.len() > 1)
    {
        return Err(format!(
            "in the {}, only the first step of a feature chain can be a qualified name",
            field.label()
        ));
    }
    Ok(())
}

/// A literal reads back as the same literal.
fn check_literal(literal: &Literal) -> Result<(), String> {
    let text = format!("attribute x = {literal};");
    if read_back(&text).is_some_and(|e| e.value.as_ref() == Some(literal)) {
        return Ok(());
    }
    Err(match literal {
        Literal::Integer(text) => format!("`{text}` is not a whole number"),
        Literal::Real(text) => format!("`{text}` is not a real number such as `1.5` or `2e3`"),
        Literal::String(_) => {
            "a string value is written with `\\\"` for a quote, `\\\\` for a backslash and `\\n` for a line break"
                .into()
        }
        Literal::Boolean(_) => "a boolean is `true` or `false`".into(),
    })
}

/// An expression reads back as the same expression (names as written).
fn check_expression(expression: &Expression) -> Result<(), String> {
    let mut literals = Vec::new();
    expression.walk(&mut |e| {
        if let Expression::Literal(literal) = e {
            literals.push(literal.clone());
        }
    });
    for literal in &literals {
        check_literal(literal)?;
    }
    if let Expression::New { arguments, .. } = expression
        && arguments.iter().any(|a| a.feature.steps.len() != 2)
    {
        return Err("an argument of `new` names one feature of the type".into());
    }
    let text = format!("send {expression};");
    let read = read_back(&text).and_then(|e| e.expression);
    if read.is_some_and(|read| without_links(&read) == without_links(expression)) {
        Ok(())
    } else {
        Err(format!(
            "the expression `{expression}` does not read back as written"
        ))
    }
}

fn without_links(expression: &Expression) -> Expression {
    let mut copy = expression.clone();
    for reference in copy.references_mut() {
        for step in &mut reference.steps {
            step.target = None;
        }
    }
    copy
}

/// A doc or comment has text that reads back the same.
fn check_text(e: &Element) -> Result<(), String> {
    let Some(text) = &e.text else {
        return Err(format!("{} needs text (it may be empty)", a(e.kind)));
    };
    let mut scratch = Tree::new();
    let document = scratch.add_document("");
    let mut copy = Element::new(e.kind);
    copy.text = Some(text.clone());
    let id = scratch
        .add(Parent::Document(document), copy)
        .expect("the document exists");
    let printed = print_element(&scratch, id).expect("the element exists");
    if read_back(&printed).is_some_and(|back| back.text.as_ref() == Some(text)) {
        Ok(())
    } else if text.contains("*/") {
        Err("comment text cannot contain `*/`".into())
    } else {
        Err("comment text cannot begin or end with blank lines or spaces, or have spaces at the end of a line".into())
    }
}

/// The one element the parser reads from `text`, if it reads exactly one.
fn read_back(text: &str) -> Option<Element> {
    let mut tree = Tree::new();
    parse_into(&mut tree, &[Source::new("", text)]);
    let roots: Vec<ElementId> = tree.roots().collect();
    match roots[..] {
        [root] => tree.get(root).cloned(),
        _ => None,
    }
}

fn no(kind: ElementKind, field: Field) -> String {
    format!("{} has no {}", a(kind), field.label())
}

/// `a part`, `an item def`.
fn a(kind: ElementKind) -> String {
    let keyword = kind.keyword();
    let article = if keyword.starts_with(['a', 'e', 'i', 'o', 'u']) {
        "an"
    } else {
        "a"
    };
    format!("{article} {keyword}")
}
