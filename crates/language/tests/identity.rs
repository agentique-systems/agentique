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
