//! References carry identity: renames and moves keep them bound, removals are
//! reported, unresolved names are retried, and ids are never reused.
use agq_language::{
    Element, ElementId, ElementKind, Parent, Role, Source, Tree, TreeError, link, parse, print,
    validate,
};
use std::collections::HashMap;

fn load(text: &str) -> Tree {
    parse(&[Source::new("test.sysml", text)])
}

fn codes(tree: &Tree) -> Vec<(String, &'static str)> {
    validate(tree)
        .into_iter()
        .map(|d| (tree.qualified_name(d.element), d.code))
        .collect()
}

const PROBE: &str = "package P {
    part def A;

    package Q {
        part def A;

        part x : A;
    }
}
";

#[test]
fn a_rename_never_rebinds_a_reference_by_name() {
    let mut tree = load(PROBE);
    let inner = tree.find("P::Q::A").unwrap();
    let x = tree.find("P::Q::x").unwrap();
    assert_eq!(tree[x].typed_by[0].target(), Some(inner));

    tree.get_mut(inner).unwrap().name = Some("B".into());
    assert_eq!(tree[x].typed_by[0].target(), Some(inner));
    assert_eq!(codes(&tree), []);
    let text = &print(&tree)[0].text;
    assert!(text.contains("part x : B;"), "{text}");
    let reloaded = load(text);
    let x = reloaded.find("P::Q::x").unwrap();
    assert_eq!(reloaded[x].typed_by[0].target(), reloaded.find("P::Q::B"));
}

#[test]
fn a_move_keeps_references_and_prints_a_name_that_still_resolves() {
    let mut tree = load(
        "package P { package Q { part def A; } package R { part x : Q::A; part y : P::Q::A; } }",
    );
    let a = tree.find("P::Q::A").unwrap();
    let p = tree.find("P").unwrap();
    tree.move_to(a, Parent::Element(p), 0).unwrap();
    assert_eq!(tree.qualified_name(a), "P::A");
    assert_eq!(codes(&tree), []);
    let text = &print(&tree)[0].text;
    assert!(text.contains("part x : P::A;"), "{text}");
    assert!(text.contains("part y : P::A;"), "{text}");
    assert_eq!(codes(&load(text)), []);

    let q = tree.find("P::Q").unwrap();
    let r = tree.find("P::R").unwrap();
    assert_eq!(
        tree.move_to(p, Parent::Element(q), 0),
        Err(TreeError::IntoItself)
    );
    assert_eq!(
        tree.move_to(r, Parent::Element(r), 0),
        Err(TreeError::IntoItself)
    );
}

#[test]
fn a_removed_target_is_reported_where_it_was_used() {
    let mut tree = load(PROBE);
    let inner = tree.find("P::Q::A").unwrap();
    let x = tree.find("P::Q::x").unwrap();
    assert_eq!(tree.references_to(inner), [(x, Role::TypedBy)]);
    tree.remove(inner);
    let diagnostics = validate(&tree);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "removed-target");
    assert_eq!(diagnostics[0].element, x);
    assert_eq!(
        diagnostics[0].message,
        "the type `A` referred to `A`, which no longer exists"
    );
    // Linking again does not re-bind it to `P::A` by name.
    link(&mut tree);
    assert_eq!(tree[x].typed_by[0].target(), Some(inner));
}

#[test]
fn unresolved_references_are_linked_once_their_target_exists() {
    let mut tree = load("package P { part x : Later; }");
    assert_eq!(codes(&tree), [("P::x".to_string(), "unresolved")]);
    let x = tree.find("P::x").unwrap();
    assert_eq!(tree[x].typed_by[0].target(), None);
    let p = tree.find("P").unwrap();
    let later = tree
        .add(
            Parent::Element(p),
            Element::named(ElementKind::PartDef, "Later"),
        )
        .unwrap();
    assert_eq!(codes(&tree), []); // validation retries by name
    link(&mut tree);
    assert_eq!(tree[x].typed_by[0].target(), Some(later));
}

#[test]
fn chains_link_every_step() {
    let tree = load(
        "package P {
             port def Pt;
             part def S { port p : Pt; port q : Pt; }
             part def T { part a : S; part b : S; connection c connect a.p to b.q; }
         }",
    );
    let c = tree.find("P::T::c").unwrap();
    let ends = &tree[c].ends;
    assert_eq!(
        ends[0].steps.iter().map(|s| s.target).collect::<Vec<_>>(),
        [tree.find("P::T::a"), tree.find("P::S::p")]
    );
    assert_eq!(ends[1].target(), tree.find("P::S::q"));
    assert_eq!(tree.references().len(), 6); // 4 typings and 2 ends
}

#[test]
fn removed_ids_are_never_handed_out_again() {
    let mut tree = load("package P { part def A; }");
    let p = tree.find("P").unwrap();
    let a = tree.find("P::A").unwrap();
    tree.remove(a);
    let b = tree
        .add(
            Parent::Element(p),
            Element::named(ElementKind::PartDef, "B"),
        )
        .unwrap();
    assert!(b.raw() > a.raw());
    assert_eq!(Tree::default(), Tree::new());
}

#[test]
fn rekey_replaces_parsed_ids_with_stored_ones() {
    let mut tree = load(PROBE);
    let (p, inner, x) = (
        tree.find("P").unwrap(),
        tree.find("P::Q::A").unwrap(),
        tree.find("P::Q::x").unwrap(),
    );
    let stored: HashMap<ElementId, ElementId> = [
        (p, ElementId::from_raw(500)),
        (inner, ElementId::from_raw(501)),
        (x, ElementId::from_raw(502)),
        // An element deleted since it was stored: its id stays retired.
        (ElementId::from_raw(9_999), ElementId::from_raw(900)),
    ]
    .into();
    let fresh = tree.rekey(&stored).unwrap();
    assert_eq!(fresh.len(), 2); // P::A and P::Q were not in the map
    assert!(fresh.iter().all(|id| id.raw() > 900));
    let x = tree.find("P::Q::x").unwrap();
    assert_eq!(x, ElementId::from_raw(502));
    assert_eq!(tree[x].typed_by[0].target(), Some(ElementId::from_raw(501)));
    assert_eq!(tree[x].owner(), tree.find("P::Q"));
    assert_eq!(codes(&tree), []);
    let next = tree
        .add(Parent::Element(x), Element::new(ElementKind::Comment))
        .unwrap();
    assert!(next.raw() > fresh.iter().map(|id| id.raw()).max().unwrap());

    let p = tree.find("P").unwrap();
    let clash: HashMap<ElementId, ElementId> =
        [(p, ElementId::from_raw(1)), (x, ElementId::from_raw(1))].into();
    assert_eq!(
        tree.rekey(&clash),
        Err(TreeError::BadId(ElementId::from_raw(1)))
    );
}

/// Every reference as (holder's position in document order, role,
/// qualified names of its step targets).
fn bindings(tree: &Tree) -> Vec<(usize, Role, Vec<String>)> {
    let order = tree.walk();
    tree.references()
        .into_iter()
        .map(|(holder, role, reference)| {
            let targets = reference
                .steps
                .iter()
                .map(|s| s.target.map_or("-".into(), |t| tree.qualified_name(t)))
                .collect();
            let position = order.iter().position(|id| *id == holder).unwrap();
            (position, role, targets)
        })
        .collect()
}

/// A tree that validates clean prints text that loads with the same bindings.
fn survives_saving(tree: &Tree) {
    assert_eq!(codes(tree), []);
    let reloaded = parse(&print(tree));
    assert_eq!(bindings(&reloaded), bindings(tree));
}

#[test]
fn valid_trees_reload_with_the_same_bindings() {
    survives_saving(&load(include_str!(
        "../../../models/url-shortener/UrlShortener.sysml"
    )));

    let mut renamed = load(PROBE);
    let inner = renamed.find("P::Q::A").unwrap();
    renamed.get_mut(inner).unwrap().name = Some("B".into());
    survives_saving(&renamed);

    let mut moved = load("package P { package Q { part def A; } package R { part x : Q::A; } }");
    let (a, p) = (moved.find("P::Q::A").unwrap(), moved.find("P").unwrap());
    moved.move_to(a, Parent::Element(p), 0).unwrap();
    survives_saving(&moved);

    // `import Q::A` becomes `import Q::B`, even where a `P` hides the package.
    let mut imported = load(
        "package P { package Q { part def A; } package R { part def P; import Q::A; part x : A; } }",
    );
    let a = imported.find("P::Q::A").unwrap();
    imported.get_mut(a).unwrap().name = Some("B".into());
    let text = &print(&imported)[0].text;
    assert!(text.contains("import Q::B;"), "{text}");
    survives_saving(&imported);
}

#[test]
fn a_target_no_name_leads_back_to_is_reported() {
    // Hidden: a new `Q::A` hides `P::A`, and `P` means the inner package.
    let mut hidden =
        load("package P { part def A; package Q { package P { part def A; } part x : A; } }");
    let q = hidden.find("P::Q").unwrap();
    hidden
        .add(
            Parent::Element(q),
            Element::named(ElementKind::PartDef, "A"),
        )
        .unwrap();
    assert_eq!(
        codes(&hidden),
        [("P::Q::x".to_string(), "unreachable-target")]
    );

    // Private.
    let mut private = load("package P { package Q { part def A; } part x : Q::A; }");
    let a = private.find("P::Q::A").unwrap();
    private.get_mut(a).unwrap().visibility = agq_language::Visibility::Private;
    assert_eq!(
        codes(&private),
        [("P::x".to_string(), "unreachable-target")]
    );

    // Inside an unnamed element.
    let mut unnamed = load(
        "package P { part def A; part def S { part a : A[0..1]; part b : A[0..1]; connect a to b; } part x : A; }",
    );
    let a = unnamed.find("P::A").unwrap();
    let s = unnamed.find("P::S").unwrap();
    let connect = unnamed[s].children()[2];
    unnamed.move_to(a, Parent::Element(connect), 0).unwrap();
    let problems = codes(&unnamed);
    assert!(
        problems.contains(&("P::x".to_string(), "unreachable-target")),
        "{problems:?}"
    );
    assert!(
        problems
            .iter()
            .all(|(_, code)| *code == "unreachable-target"),
        "{problems:?}"
    );
}

#[test]
fn rekey_keeps_references_to_removed_elements_removed() {
    let mut tree = load("package P { part def A; part x : A; }");
    let (p, a) = (tree.find("P").unwrap(), tree.find("P::A").unwrap());
    tree.remove(a);
    tree.add(
        Parent::Element(p),
        Element::named(ElementKind::PartDef, "A"),
    )
    .unwrap();
    assert_eq!(codes(&tree), [("P::x".to_string(), "removed-target")]);
    tree.rekey(&HashMap::new()).unwrap();
    link(&mut tree);
    assert_eq!(codes(&tree), [("P::x".to_string(), "removed-target")]);
}

#[test]
fn the_public_api_does_not_panic_on_odd_input() {
    let mut tree = load("package P { part def A; part x : A; }");
    let (p, a) = (tree.find("P").unwrap(), tree.find("P::A").unwrap());
    tree.remove(a);
    assert_eq!(tree.qualified_name(a), a.to_string());
    assert_eq!(agq_language::print_element(&tree, a), None);

    let x = tree.find("P::x").unwrap();
    tree.get_mut(x).unwrap().typed_by = vec![agq_language::Reference { steps: vec![] }];
    let mut one_end = Element::new(ElementKind::Connection);
    one_end.ends.push(agq_language::Reference::new("x"));
    tree.add(Parent::Element(p), one_end).unwrap();
    tree.add(Parent::Element(p), Element::new(ElementKind::Satisfy))
        .unwrap();
    tree.add(Parent::Element(p), Element::new(ElementKind::Import))
        .unwrap();
    let found: Vec<&str> = validate(&tree).iter().map(|d| d.code).collect();
    assert_eq!(
        found,
        [
            "unresolved",
            "wrong-end-count",
            "missing-target",
            "missing-target"
        ]
    );
    print(&tree);
}

#[test]
fn reserved_ids_are_never_handed_out() {
    let mut tree = load(PROBE);
    // Ids up to 699 were used before, by elements that are gone.
    tree.reserve_ids(ElementId::from_raw(700));
    let fresh = tree.rekey(&HashMap::new()).unwrap();
    assert!(fresh.iter().all(|id| id.raw() >= 700));
    let next = tree.next_id();
    let p = tree.find("P").unwrap();
    let added = tree
        .add(Parent::Element(p), Element::new(ElementKind::Comment))
        .unwrap();
    assert_eq!(added, next);
    assert!(tree.next_id() > next);
    // Reserving never lowers the next id.
    tree.reserve_ids(ElementId::from_raw(1));
    assert!(tree.next_id() > added);
}
