//! Text in and out: unsupported constructs stay explicit and verbatim, syntax
//! errors recover at the member, and printing is canonical.
use agq_language::{Element, ElementKind, Parent, Reference, Source, Tree, parse, print, validate};

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

    calc def Color { }

    part def Car {
        attribute color : Color;
        attribute speed = wheels->size();
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
        messages[2].contains("`Color` is an unsupported `calc def`"),
        "{}",
        messages[2]
    );
    assert!(
        messages[3].starts_with("`->` function calls"),
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
    let package = tree
        .add(
            Parent::Document(doc),
            Element::named(ElementKind::Package, "P"),
        )
        .unwrap();
    let plug = tree
        .add(
            Parent::Element(package),
            Element::named(ElementKind::PortDef, "Plug"),
        )
        .unwrap();
    let device = tree
        .add(
            Parent::Element(package),
            Element::named(ElementKind::PartDef, "Device"),
        )
        .unwrap();
    let mut socket = Element::named(ElementKind::Port, "socket");
    socket.typed_by.push(Reference::to(plug, "Plug"));
    socket.conjugated = true;
    let socket = tree.add(Parent::Element(device), socket).unwrap();
    let mut doc = Element::new(ElementKind::Doc);
    doc.text = Some("Plugs in.\nNever */ ends early.".into());
    tree.insert(Parent::Element(device), 0, doc).unwrap();
    assert_eq!(tree[device].children()[1], socket);
    assert_eq!(codes(&tree), []);
    let text = &print(&tree)[0].text;
    assert_eq!(
        text,
        "package P {
    port def Plug;

    part def Device {
        doc /* Plugs in.
             * Never * / ends early.
             */
        port socket : ~Plug;
    }
}
"
    );
    assert_eq!(codes(&load(text)), []);
}

#[test]
fn comments_are_elements_and_never_swallow_members() {
    let text = "package P {
    /* A note about A. */
    part def A;

    /* Two
     * lines. */
    part def B;
}
";
    let tree = load(text);
    assert_eq!(codes(&tree), []);
    let kinds: Vec<ElementKind> = tree[tree.find("P").unwrap()]
        .children()
        .iter()
        .map(|c| tree[*c].kind)
        .collect();
    use ElementKind::{Comment, PartDef};
    assert_eq!(kinds, [Comment, PartDef, Comment, PartDef]);
    assert_eq!(
        print(&tree)[0].text,
        "package P {
    /* A note about A. */

    part def A;

    /* Two
     * lines.
     */

    part def B;
}
"
    );
    round_trips(&tree);
}

#[test]
fn recovery_stops_at_the_broken_member_despite_open_brackets() {
    let tree = load("package P { part def A { part x : A[1 ; } part def B; }");
    assert_eq!(
        codes(&tree),
        [("P::A::(syntax error)".to_string(), "syntax")]
    );
    assert!(tree.find("P::B").is_some());
    let tree = load("package P { part def A { attribute v = f(1 ; } part def B; }");
    assert_eq!(codes(&tree), [("P::A::v".to_string(), "unsupported")]);
    assert!(tree.find("P::B").is_some());
}

#[test]
fn keyword_less_usages_are_reference_usages() {
    let text = "package P {
    private import ScalarValues::*;

    requirement def R {
        attribute limit : Natural;
    }

    requirement r : R {
        :>> limit = 5;
    }

    part def Q {
        ref helper : R;
        other : R;
    }
}
";
    let tree = load(text);
    assert_eq!(codes(&tree), []);
    let limit = tree.find("P::r::limit").unwrap();
    assert_eq!(tree[limit].kind, ElementKind::Reference);
    let printed = &print(&tree)[0].text;
    assert!(printed.contains("        :>> limit = 5;\n"));
    assert!(printed.contains("        helper : R;\n"));
    round_trips(&tree);
}

#[test]
fn unsupported_forms_that_only_refer_to_names_declare_none() {
    let tree = load(
        "package P { part def A { part x : A[0..1]; part y : A[0..1]; bind x = y; perform x; perform y; } }",
    );
    let codes = codes(&tree);
    assert_eq!(codes.len(), 3, "{codes:?}");
    assert!(
        codes.iter().all(|(_, code)| *code == "unsupported"),
        "{codes:?}"
    );
}

#[test]
fn an_escape_at_a_line_end_keeps_line_numbers() {
    let tree = load("package P {\n    part def 'A\\\n    ;\n    part def B :> Missing;\n}\n");
    let located: Vec<(&str, u32)> = validate(&tree)
        .into_iter()
        .map(|d| (d.code, d.location.unwrap().line))
        .collect();
    assert_eq!(located, [("syntax", 2), ("unresolved", 4)]);
}

#[test]
fn valid_sysml_outside_the_subset_is_unsupported_not_a_syntax_error() {
    let tree = load(
        "package P {
             part def A { part a : A[0..1]; }
             connection def C { end a : A; }
             part x :> a.b;
             part y :>> a.b;
             #M part z;
             @M;
         }",
    );
    let found: Vec<(String, &str)> = codes(&tree);
    assert_eq!(found.len(), 5, "{found:?}");
    assert!(
        found.iter().all(|(_, code)| *code == "unsupported"),
        "{found:?}"
    );
}
