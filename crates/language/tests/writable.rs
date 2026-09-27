//! `writable` agrees with the printer and the parser: an element it accepts
//! prints and reads back as the same element, and one it refuses does not.
use agq_language::{
    Direction, Element, ElementId, ElementKind, Field, Literal, Multiplicity, Parent, Reference,
    Tree, Visibility, parse, print, writable,
};

const KINDS: [ElementKind; 21] = {
    use ElementKind::*;
    [
        Package,
        PartDef,
        Part,
        PortDef,
        Port,
        ItemDef,
        Item,
        AttributeDef,
        Attribute,
        ConnectionDef,
        Connection,
        InterfaceDef,
        Interface,
        RequirementDef,
        Requirement,
        Subject,
        Reference,
        Satisfy,
        Import,
        Doc,
        Comment,
    ]
};

const FIELDS: [Field; 17] = [
    Field::Name,
    Field::Visibility,
    Field::Abstract,
    Field::End,
    Field::Direction,
    Field::TypedBy,
    Field::Conjugated,
    Field::Specializes,
    Field::Redefines,
    Field::Multiplicity,
    Field::Value,
    Field::Ends,
    Field::Target,
    Field::Wildcard,
    Field::By,
    Field::Text,
    Field::Members,
];

fn named(text: &str) -> Reference {
    Reference::new(text)
}

/// The smallest writable element of a kind.
fn minimal(kind: ElementKind) -> Element {
    use ElementKind::*;
    let mut e = Element::new(kind);
    match kind {
        Package | Reference => e.name = Some("n".into()),
        _ if kind.is_definition() => e.name = Some("N".into()),
        Import | Satisfy => e.target = Some(named("T")),
        Doc | Comment => e.text = Some("text".into()),
        _ => {}
    }
    e
}

fn with(mut e: Element, field: Field) -> Element {
    match field {
        Field::Name => e.name = Some("m".into()),
        Field::Visibility => e.visibility = Visibility::Private,
        Field::Abstract => e.is_abstract = true,
        Field::End => e.is_end = true,
        Field::Direction => e.direction = Some(Direction::In),
        Field::TypedBy => e.typed_by = vec![named("T")],
        Field::Conjugated => {
            e.conjugated = true;
            e.typed_by = vec![named("T")];
        }
        Field::Specializes => e.specializes = vec![named("S")],
        Field::Redefines => e.redefines = vec![named("R")],
        Field::Multiplicity => {
            e.multiplicity = Some(Multiplicity {
                lower: 1,
                upper: Some(2),
            })
        }
        Field::Value => e.value = Some(Literal::Integer("1".into())),
        Field::Ends => e.ends = vec![named("a.b"), named("c")],
        Field::Target => e.target = Some(named("T")),
        Field::Wildcard => e.wildcard = true,
        Field::By => e.by = Some(named("x")),
        Field::Text => e.text = Some("text".into()),
        Field::Members => {}
    }
    e
}

/// A package holding `element` (and, for `Members`, a comment inside it).
fn tree_with(element: Element, member: bool) -> (Tree, ElementId, Option<ElementId>) {
    let mut tree = Tree::new();
    let document = tree.add_document("test.sysml");
    let package = tree
        .add(Parent::Document(document), minimal(ElementKind::Package))
        .unwrap();
    let id = tree.add(Parent::Element(package), element).unwrap();
    let child = member.then(|| {
        tree.add(Parent::Element(id), minimal(ElementKind::Comment))
            .unwrap()
    });
    (tree, id, child)
}

/// Prints and reads back; compares elements without locations and links.
fn reads_back(tree: &Tree) -> bool {
    let normal = |tree: &Tree| {
        tree.walk()
            .into_iter()
            .map(|id| {
                let mut e = tree[id].clone();
                e.location = None;
                for reference in e
                    .typed_by
                    .iter_mut()
                    .chain(&mut e.specializes)
                    .chain(&mut e.redefines)
                    .chain(&mut e.ends)
                    .chain(&mut e.target)
                    .chain(&mut e.by)
                {
                    reference.steps.iter_mut().for_each(|s| s.target = None);
                }
                (id, e)
            })
            .collect::<Vec<_>>()
    };
    normal(tree) == normal(&parse(&print(tree)))
}

#[test]
fn writable_agrees_with_printing_and_reading_back_for_every_kind_and_field() {
    let mut disagreements = Vec::new();
    for kind in KINDS {
        let (tree, id, _) = tree_with(minimal(kind), false);
        assert!(writable(&tree, id).is_ok(), "minimal {kind:?}");
        assert!(reads_back(&tree), "minimal {kind:?}");
        for field in FIELDS {
            let (tree, id, child) = tree_with(with(minimal(kind), field), field == Field::Members);
            let accepted = writable(&tree, id).is_ok()
                && child.is_none_or(|child| writable(&tree, child).is_ok());
            if accepted != reads_back(&tree) || accepted != field.fits(kind) {
                disagreements.push((kind, field, accepted));
            }
        }
    }
    assert!(disagreements.is_empty(), "{disagreements:?}");
}

#[test]
fn combinations_that_do_not_read_back_are_refused() {
    use ElementKind::*;
    let refused = |element: Element, expected: &str| {
        let (tree, id, _) = tree_with(element, false);
        assert!(!reads_back(&tree), "{expected}");
        assert_eq!(writable(&tree, id), Err(expected.to_string()));
    };
    // `: T;` reads back as a syntax error.
    refused(
        with(Element::new(Reference), Field::TypedBy),
        "a usage without a kind keyword needs a name, or a subsetting (`:>`) or redefinition (`:>>`) and no type",
    );
    refused(
        with(Element::new(PartDef), Field::Specializes),
        "a part def needs a name",
    );
    refused(Element::new(Import), "an import needs the name it imports");
    let mut port = with(minimal(Port), Field::Conjugated);
    port.typed_by.push(named("U"));
    refused(port, "a conjugated usage (`~`) has exactly one type");
    let mut three = with(minimal(Connection), Field::Ends);
    three.ends.push(named("d"));
    refused(three, "a connection has two ends (or none), not 3");
    refused(
        with(with(minimal(Connection), Field::Ends), Field::Value),
        "a connection with ends (`connect`) cannot have a value",
    );
    let mut name = minimal(Part);
    name.name = Some("two\nlines".into());
    refused(
        name,
        "the name \"two\\nlines\" cannot be written: names cannot contain line breaks",
    );
    refused(
        with(minimal(Part), Field::Specializes).tap(|e| e.specializes = vec![named("a.b")]),
        "the specialisation or subsetting (`:>`) names an element, not a feature chain like `a.b`",
    );
    for (literal, expected) in [
        (Literal::Integer("1a".into()), "`1a` is not a whole number"),
        (
            Literal::Real("2".into()),
            "`2` is not a real number such as `1.5` or `2e3`",
        ),
        (
            Literal::String("a \"b\"".into()),
            "a string value is written with `\\\"` for a quote, `\\\\` for a backslash and `\\n` for a line break",
        ),
    ] {
        refused(
            minimal(Attribute).tap(|e| e.value = Some(literal)),
            expected,
        );
    }
    refused(
        minimal(Doc).tap(|e| e.text = Some("a */ b".into())),
        "comment text cannot contain `*/`",
    );
    refused(
        minimal(Comment).tap(|e| e.text = Some("trailing \n".into())),
        "comment text cannot begin or end with blank lines or spaces, or have spaces at the end of a line",
    );
    refused(
        minimal(Comment).tap(|e| e.text = None),
        "a comment needs text (it may be empty)",
    );
}

#[test]
fn an_end_connection_written_as_connect_keeps_its_end() {
    let connection = with(
        with(Element::new(ElementKind::Connection), Field::Ends),
        Field::End,
    );
    let (tree, id, _) = tree_with(connection, false);
    assert_eq!(writable(&tree, id), Ok(()));
    assert!(reads_back(&tree), "{}", print(&tree)[0].text);
}

trait Tap: Sized {
    fn tap(mut self, change: impl FnOnce(&mut Self)) -> Self {
        change(&mut self);
        self
    }
}

impl Tap for Element {}
