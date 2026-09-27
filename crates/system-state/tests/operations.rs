use agq_language::{Element, ElementId, ElementKind, Parent, Reference, Source, parse, print};
use agq_system_state::{
    Actor, Change, Comparison, EventKind, Operation, Property, Rejection, SystemState, compare,
};
use std::collections::BTreeSet;

const MODEL: &str = "package Shop {
    port def OrderPort { in item order : Order; }
    item def Order;
    part def Store { port orders : OrderPort; }
    part def Web { port orders : ~OrderPort; }
    part def System {
        part store : Store;
        part web : Web;
    }
}";

fn state() -> SystemState {
    let tree = parse(&[Source::new("shop.sysml", MODEL)]);
    let state = SystemState::new(tree, BTreeSet::new());
    assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
    state
}

fn id(state: &SystemState, name: &str) -> ElementId {
    state
        .tree()
        .find(name)
        .unwrap_or_else(|| panic!("no {name}"))
}

fn operator(description: &str, operations: Vec<Operation>) -> Change {
    Change::new(Actor::Operator, description, operations)
}

#[test]
fn create_and_connect_parts_by_hand() {
    let mut state = state();
    let system = id(&state, "Shop::System");
    let store = id(&state, "Shop::Store");
    let mut part = Element::named(ElementKind::Part, "backup");
    part.typed_by = vec![Reference::to(store, "Store")];
    let event = state
        .apply(operator(
            "Add a backup store and connect it",
            vec![
                Operation::Create {
                    parent: Parent::Element(system),
                    element: Box::new(part),
                },
                Operation::Connect {
                    parent: system,
                    kind: ElementKind::Connection,
                    name: Some("replication".into()),
                    definition: None,
                    from: Reference::new("web.orders"),
                    to: Reference::new("backup.orders"),
                },
            ],
        ))
        .unwrap();
    assert_eq!(event.kind, EventKind::Applied);
    assert_eq!(event.created.len(), 2);
    assert_eq!(event.updated, vec![system]);
    assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
    let text = &print(state.tree())[0].text;
    assert!(text.contains("part backup : Store;"), "{text}");
    assert!(
        text.contains("connection replication connect web.orders to backup.orders;"),
        "{text}"
    );
}

#[test]
fn rename_keeps_references_bound() {
    let mut state = state();
    let store = id(&state, "Shop::Store");
    state
        .apply(operator(
            "Rename Store",
            vec![Operation::Rename {
                element: store,
                name: "Warehouse".into(),
            }],
        ))
        .unwrap();
    assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
    let text = &print(state.tree())[0].text;
    assert!(text.contains("part store : Warehouse;"), "{text}");
}

#[test]
fn an_invalid_operation_rejects_the_whole_change() {
    let mut state = state();
    let system = id(&state, "Shop::System");
    let before = state.tree().clone();
    let result = state.apply(operator(
        "Two edits, the second impossible",
        vec![
            Operation::Rename {
                element: system,
                name: "Platform".into(),
            },
            Operation::Delete {
                element: ElementId::from_raw(99_999),
            },
        ],
    ));
    assert!(matches!(
        result,
        Err(Rejection::Invalid { operation: 1, .. })
    ));
    assert_eq!(state.tree(), &before);
    assert_eq!(state.revision(), 0);
}

#[test]
fn a_well_formed_edit_that_breaks_the_model_is_applied_and_reported() {
    let mut state = state();
    let store = id(&state, "Shop::System::store");
    state
        .apply(operator(
            "Type the store by something missing",
            vec![Operation::Set {
                element: store,
                property: Property::TypedBy(vec![Reference::new("Missing")]),
            }],
        ))
        .unwrap();
    assert_eq!(state.diagnostics_for(store).count(), 1);
    state.undo().unwrap();
    assert!(state.diagnostics().is_empty());
}

#[test]
fn a_stale_change_is_rejected() {
    let mut state = state();
    let system = id(&state, "Shop::System");
    let mut change = operator(
        "Rename",
        vec![Operation::Rename {
            element: system,
            name: "Platform".into(),
        }],
    );
    change.base = Some(3);
    assert_eq!(
        state.apply(change),
        Err(Rejection::Stale {
            base: 3,
            current: 0
        })
    );
}

#[test]
fn locked_parts_change_only_with_confirmation() {
    let mut state = state();
    let store = id(&state, "Shop::Store");
    let port = id(&state, "Shop::Store::orders");
    state
        .apply(operator(
            "Lock the store",
            vec![Operation::Lock { element: store }],
        ))
        .unwrap();
    assert!(state.is_locked(port), "a lock covers what the part owns");

    let rename = |confirmed: Vec<ElementId>| {
        let mut change = Change::new(
            Actor::Assistant,
            "Rename the store's port",
            vec![Operation::Rename {
                element: port,
                name: "incoming".into(),
            }],
        );
        change.confirmed = confirmed;
        change
    };
    assert_eq!(
        state.apply(rename(Vec::new())),
        Err(Rejection::Locked {
            elements: vec![store]
        })
    );
    state.apply(rename(vec![store])).unwrap();
    assert_eq!(state.tree()[port].name.as_deref(), Some("incoming"));

    // The Assistant cannot remove a lock on its own; the Operator can.
    let unlock = |actor| Change::new(actor, "Unlock", vec![Operation::Unlock { element: store }]);
    assert!(matches!(
        state.apply(unlock(Actor::Assistant)),
        Err(Rejection::Locked { .. })
    ));
    state.apply(unlock(Actor::Operator)).unwrap();
    assert!(!state.is_locked(port));
}

#[test]
fn adding_beside_a_locked_part_needs_no_confirmation() {
    let mut state = state();
    let system = id(&state, "Shop::System");
    let store = id(&state, "Shop::System::store");
    state
        .apply(operator("Lock", vec![Operation::Lock { element: store }]))
        .unwrap();
    let result = state.apply(Change::new(
        Actor::Assistant,
        "Add a part beside the locked one",
        vec![Operation::Create {
            parent: Parent::Element(system),
            element: Box::new(Element::named(ElementKind::Part, "cache")),
        }],
    ));
    assert!(result.is_ok(), "{result:?}");
    let result = state.apply(Change::new(
        Actor::Assistant,
        "Delete the owner of the locked part",
        vec![Operation::Delete { element: system }],
    ));
    assert_eq!(
        result,
        Err(Rejection::Locked {
            elements: vec![store]
        })
    );
}

#[test]
fn undo_and_redo_restore_the_model_and_report_what_changed() {
    let mut state = state();
    let original = print(state.tree());
    let web = id(&state, "Shop::System::web");
    state
        .apply(operator(
            "Delete web",
            vec![Operation::Delete { element: web }],
        ))
        .unwrap();
    assert_eq!(state.undo_description(), Some("Delete web"));

    let undone = state.undo().unwrap();
    assert_eq!(undone.kind, EventKind::Undone);
    assert_eq!(undone.created, vec![web]);
    assert_eq!(print(state.tree()), original);

    let redone = state.redo().unwrap();
    assert_eq!(redone.deleted, vec![web]);
    assert!(state.redo().is_none());
    assert_eq!(state.revision(), 3);
}

#[test]
fn move_keeps_identity_and_references() {
    let mut state = state();
    let order = id(&state, "Shop::Order");
    let store = id(&state, "Shop::Store");
    state
        .apply(operator(
            "Move Order into Store",
            vec![Operation::Move {
                element: order,
                parent: Parent::Element(store),
            }],
        ))
        .unwrap();
    assert_eq!(state.tree().qualified_name(order), "Shop::Store::Order");
    assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
}

#[test]
fn doc_comments_are_a_property() {
    let mut state = state();
    let store = id(&state, "Shop::Store");
    let set = |text: Option<&str>| {
        operator(
            "Document the store",
            vec![Operation::Set {
                element: store,
                property: Property::Doc(text.map(str::to_string)),
            }],
        )
    };
    state.apply(set(Some("Keeps the orders."))).unwrap();
    assert!(print(state.tree())[0].text.contains("Keeps the orders."));
    state.apply(set(None)).unwrap();
    assert!(!print(state.tree())[0].text.contains("doc"));
}

#[test]
fn a_printed_and_reread_model_compares_equal() {
    let mut state = state();
    let store = id(&state, "Shop::Store");
    state
        .apply(operator(
            "Rename Store",
            vec![Operation::Rename {
                element: store,
                name: "Warehouse".into(),
            }],
        ))
        .unwrap();
    // Reread from text, every element has a new location, and `store : Store`
    // is now written `store : Warehouse`; neither is a change. (References to
    // removed elements: see tests/project.rs.)
    let text = format!("\n\n{}", print(state.tree())[0].text);
    let reread = parse(&[Source::new("shop.sysml", text)]);
    assert_eq!(compare(state.tree(), &reread), Comparison::default());
}
