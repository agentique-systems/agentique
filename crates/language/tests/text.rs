//! Text in and out: unsupported constructs stay explicit and verbatim, syntax
//! errors recover at the member, and printing is canonical.
use agq_language::{
    Element, ElementKind, Parent, QualifiedName, Source, Tree, TypeRef, parse, print, validate,
};

fn load(text: &str) -> Tree {
    parse(&[Source::new("test.sysml", text)])
}

fn codes(tree: &Tree) -> Vec<(String, &'static str)> {
    validate(tree)
        .into_iter()
        .map(|d| (tree.qualified_name(d.element), d.code))
        .collect()
}

fn round_trips(tree: &Tree) {
    let printed = print(tree);
    let again = parse(&printed);
    assert_eq!(
        tree.clone().without_locations(),
        again.clone().without_locations()
    );
    assert_eq!(print(&again), printed);
}

#[test]
fn unsupported_constructs_are_reported_and_kept_verbatim() {
    let text = "package P {
    action def Brake { action a; }

    enum def Color { enum red; }

    part def Car {
        attribute color : Color;
        attribute speed = 1 + 2;
        perform action brake;
    }
}
";
    let tree = load(text);
    let brake = tree
        .find("P::Brake")
        .expect("the unsupported element keeps its name");
    assert_eq!(tree[brake].kind, ElementKind::Unsupported);
    assert_eq!(
        tree[brake].text.as_deref(),
        Some("action def Brake { action a; }")
    );
    assert_eq!(
        codes(&tree),
        [
            ("P::Brake".to_string(), "unsupported"),
            ("P::Color".to_string(), "unsupported"),
            ("P::Car::color".to_string(), "unsupported"),
            ("P::Car::speed".to_string(), "unsupported"),
            ("P::Car::brake".to_string(), "unsupported"),
        ]
    );
    let messages: Vec<String> = validate(&tree).into_iter().map(|d| d.message).collect();
    assert_eq!(
        messages[0],
        "`action def` is not supported; it is kept as text and not validated"
    );
    assert!(
        messages[2].contains("`Color` is an unsupported `enum def`"),
        "{}",
        messages[2]
    );
    assert!(
        messages[3].starts_with("feature value expression"),
        "{}",
        messages[3]
    );
    assert_eq!(print(&tree)[0].text, text);
    round_trips(&tree);
}

#[test]
fn a_syntax_error_is_contained_to_its_member() {
    let tree = load("package P { part def A { part x : ; part y : B; } part def B; }");
    assert_eq!(
        codes(&tree),
        [("P::A::(syntax error)".to_string(), "syntax")]
    );
    assert!(tree.find("P::A::y").is_some());
    assert!(tree.find("P::B").is_some());
    let d = &validate(&tree)[0];
    assert_eq!(d.message, "expected a name, found `;`");
    assert_eq!(d.location.map(|l| l.column), Some(35));
    round_trips(&tree);
}

#[test]
fn an_unclosed_body_keeps_its_text() {
    let text = "package P {\n    part def A {\n        part x : A;\n";
    let tree = load(text);
    assert_eq!(codes(&tree), [("(syntax error)".to_string(), "syntax")]);
    assert_eq!(print(&tree)[0].text, text);
}

#[test]
fn keyword_forms_mean_the_same_as_symbols() {
    let words = load(
        "package P { part def A; part def B specializes A; part a typed by A; part b : B subsets a; part def C :> A { part x redefines a; } }",
    );
    let symbols = load(
        "package P { part def A; part def B :> A; part a : A; part b : B :> a; part def C :> A { part x :>> a; } }",
    );
    assert_eq!(words.without_locations(), symbols.without_locations());
}

#[test]
fn names_docs_and_values_print_canonically() {
    let tree = load(
        "package 'My Shop' {
            doc /* First line.
                 * Second line. */
            import ScalarValues::*;
            abstract part def 'Order Line' ;
            part def Order { in attribute total : Real = 12.50; part lines : 'Order Line' [0..*]; attribute ok : Boolean = true; }
            // gone
        }",
    );
    assert_eq!(codes(&tree), []);
    let expected = "package 'My Shop' {
    doc /* First line.
         * Second line.
         */

    import ScalarValues::*;

    abstract part def 'Order Line';

    part def Order {
        in attribute total : Real = 12.50;
        part lines : 'Order Line'[*];
        attribute ok : Boolean = true;
    }
}
";
    assert_eq!(print(&tree)[0].text, expected);
    round_trips(&tree);
}

#[test]
fn a_tree_built_in_code_prints_and_validates() {
    let mut tree = Tree::new();
    let doc = tree.add_document("built.sysml");
    let package = tree.add(
        Parent::Document(doc),
        Element::named(ElementKind::Package, "P"),
    );
    tree.add(
        Parent::Element(package),
        Element::named(ElementKind::PortDef, "Plug"),
    );
    let mut socket = Element::named(ElementKind::Port, "socket");
    socket.typed_by.push(TypeRef {
        name: QualifiedName::new(["Plug"]),
        conjugated: true,
    });
    let device = tree.add(
        Parent::Element(package),
        Element::named(ElementKind::PartDef, "Device"),
    );
    tree.add(Parent::Element(device), socket);
    assert_eq!(codes(&tree), []);
    assert_eq!(
        print(&tree)[0].text,
        "package P {\n    port def Plug;\n\n    part def Device {\n        port socket : ~Plug;\n    }\n}\n"
    );
}
