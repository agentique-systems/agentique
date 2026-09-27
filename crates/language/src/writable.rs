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
    /// `connect a to b`
    Ends,
    /// The imported name, or the requirement a `satisfy` names.
    Target,
    /// `import A::*`
    Wildcard,
    /// `satisfy R by x`
    By,
    /// The text of a doc or comment.
    Text,
    /// Members in a body `{ ... }`, such as a doc comment.
    Members,
}

impl Field {
    /// Whether elements of `kind` can have this field in text.
    pub fn fits(self, kind: ElementKind) -> bool {
        use ElementKind::*;
        match self {
            Field::Name => kind.is_namespace(),
            Field::Visibility => kind.is_namespace() || matches!(kind, Import | Satisfy),
            Field::Abstract | Field::Specializes => kind.is_definition() || kind.is_usage(),
            // `end x;` without a kind keyword reads back as a port, or not at all.
            Field::End => kind.is_usage() && kind != Reference,
            Field::Direction
            | Field::TypedBy
            | Field::Conjugated
            | Field::Redefines
            | Field::Multiplicity
            | Field::Value => kind.is_usage(),
            Field::Ends => matches!(kind, Connection | Interface),
            Field::Target => matches!(kind, Import | Satisfy),
            Field::Wildcard => kind == Import,
            Field::By => kind == Satisfy,
            Field::Text => matches!(kind, Doc | Comment),
            Field::Members => kind.is_namespace() || kind == Satisfy,
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
    if let Some(owner) = e.owner()
        && !Field::Members.fits(tree[owner].kind)
    {
        return Err(no(tree[owner].kind, Field::Members));
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
        None if kind == Package || kind.is_definition() => {
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
    if !matches!(e.ends.len(), 0 | 2) {
        return Err(format!(
            "{} has two ends (or none), not {}",
            a(kind),
            e.ends.len()
        ));
    }
    if e.value.is_some() && !e.ends.is_empty() {
        return Err(format!(
            "{} with ends (`connect`) cannot have a value",
            a(kind)
        ));
    }
    for (role, reference) in e.references() {
        check_reference(role, reference)?;
    }
    if let Some(literal) = &e.value {
        check_literal(literal)?;
    }
    if matches!(kind, Doc | Comment) {
        check_text(e)?;
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

/// Only connection ends and `by` features are feature chains (`a.b`); every
/// step names something.
fn check_reference(role: Role, reference: &Reference) -> Result<(), String> {
    let field = match role {
        Role::TypedBy => Field::TypedBy,
        Role::Specializes => Field::Specializes,
        Role::Redefines => Field::Redefines,
        Role::End => Field::Ends,
        Role::Target => Field::Target,
        Role::By => Field::By,
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
    if reference.steps.len() > 1 && !matches!(role, Role::End | Role::By) {
        return Err(format!(
            "the {} names an element, not a feature chain like `{reference}`",
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
