use agq_language::{
    Diagnostic, Direction, Element, ElementId, ElementKind, Literal, Multiplicity, Parent,
    QualifiedName, Reference, Source, Visibility, parse, print, validate,
};
use agq_system_state::{
    Actor, Change, ChangeEvent, Comparison, EventKind, Operation, Property, Rejection, SystemState,
    compare,
};
use std::collections::{BTreeSet, HashMap};

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

// ---- Scenario A3, A4 and A9 through the public API ----

fn rejected_reason(result: Result<ChangeEvent, Rejection>) -> String {
    match result {
        Err(Rejection::Invalid { reason, .. }) => reason,
        other => panic!("expected an invalid operation, got {other:?}"),
    }
}

fn set(element: ElementId, property: Property) -> Operation {
    Operation::Set { element, property }
}

/// Saving and reopening gives the same elements (kinds, names, properties,
/// owners and what references point at) and the same problems (A9).
fn assert_survives_saving(state: &SystemState) {
    let tree = state.tree();
    let mut reopened = parse(&print(tree));
    // Reopening keeps identities: the reopened elements get the stored ids,
    // matched in document order as History does.
    let (stored, parsed) = (tree.walk(), reopened.walk());
    assert_eq!(stored.len(), parsed.len(), "as many elements");
    let ids: HashMap<ElementId, ElementId> = parsed.into_iter().zip(stored.clone()).collect();
    reopened.rekey(&ids).unwrap();
    // A linked reference is compared by its targets; the printed name may
    // differ from the name first written (after a rename, for example).
    let normal = |element: &Element| {
        let mut element = element.clone();
        element.location = None;
        for reference in element
            .typed_by
            .iter_mut()
            .chain(&mut element.specializes)
            .chain(&mut element.redefines)
            .chain(&mut element.ends)
            .chain(&mut element.target)
            .chain(&mut element.by)
        {
            for step in &mut reference.steps {
                if step.target.is_some() {
                    step.name = QualifiedName::default();
                }
            }
        }
        element
    };
    for id in stored {
        assert_eq!(
            normal(&reopened[id]),
            normal(&tree[id]),
            "{}",
            tree.qualified_name(id)
        );
    }
    let codes = |diagnostics: &[Diagnostic]| diagnostics.iter().map(|d| d.code).collect::<Vec<_>>();
    assert_eq!(
        codes(&validate(&reopened)),
        codes(state.diagnostics()),
        "{:?}",
        state.diagnostics()
    );
}

#[test]
fn only_elements_that_can_own_members_are_parents() {
    let mut state = state();
    let store = id(&state, "Shop::Store");
    let port = id(&state, "Shop::Store::orders");
    state
        .apply(operator(
            "Document",
            vec![set(store, Property::Doc(Some("Keeps orders.".into())))],
        ))
        .unwrap();
    let doc = state.tree()[store].children()[0];
    let reason = rejected_reason(state.apply(operator(
        "Add into a doc comment",
        vec![Operation::Create {
            parent: Parent::Element(doc),
            element: Box::new(Element::named(ElementKind::Part, "p")),
        }],
    )));
    assert_eq!(
        reason,
        "`Shop::Store::(doc)` is a doc and cannot own elements"
    );

    // A part inside a port can be written, so it is applied and left to
    // validation.
    state
        .apply(operator(
            "Add a part inside a port",
            vec![Operation::Create {
                parent: Parent::Element(port),
                element: Box::new(Element::named(ElementKind::Part, "inner")),
            }],
        ))
        .unwrap();
    assert_survives_saving(&state);
}

#[test]
fn new_elements_must_be_writable() {
    let mut state = state();
    let shop = id(&state, "Shop");
    let create = |element: Element| {
        operator(
            "Create",
            vec![Operation::Create {
                parent: Parent::Element(shop),
                element: Box::new(element),
            }],
        )
    };
    let reason = rejected_reason(state.apply(create(Element::new(ElementKind::Unsupported))));
    assert!(reason.contains("cannot be created"), "{reason}");
    assert_eq!(
        rejected_reason(state.apply(create(Element::new(ElementKind::PartDef)))),
        "`Shop::(part def)`: a part def needs a name"
    );
    let mut typed_def = Element::named(ElementKind::PartDef, "Cache");
    typed_def.typed_by = vec![Reference::new("Store")];
    assert_eq!(
        rejected_reason(state.apply(create(typed_def))),
        "`Shop::Cache`: a part def has no type (`:`)"
    );
    let mut import = Element::new(ElementKind::Import);
    import.wildcard = true;
    assert_eq!(
        rejected_reason(state.apply(create(import))),
        "`Shop::(import)`: an import needs the name it imports"
    );
    assert_eq!(state.revision(), 0);

    // A new element loses any source location it was copied with.
    let mut copied = Element::named(ElementKind::PartDef, "Cache");
    copied.location = state.tree()[shop].location;
    let event = state.apply(create(copied)).unwrap();
    assert_eq!(state.tree()[event.created[0]].location, None);
}

#[test]
fn names_that_need_quoting_survive_saving_and_line_breaks_are_rejected() {
    let mut state = state();
    let store = id(&state, "Shop::Store");
    let web = id(&state, "Shop::Web");
    state
        .apply(operator(
            "Rename",
            vec![
                Operation::Rename {
                    element: store,
                    name: "Order store".into(),
                },
                Operation::Rename {
                    element: web,
                    name: "part".into(),
                },
            ],
        ))
        .unwrap();
    let text = &print(state.tree())[0].text;
    assert!(text.contains("part store : 'Order store';"), "{text}");
    assert!(text.contains("part def 'part'"), "{text}");
    assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
    assert_survives_saving(&state);

    for bad in ["", "  ", "Two\nlines"] {
        let result = state.apply(operator(
            "Rename",
            vec![Operation::Rename {
                element: store,
                name: bad.into(),
            }],
        ));
        assert!(matches!(result, Err(Rejection::Invalid { .. })), "{bad:?}");
    }
    let reason = rejected_reason(state.apply(operator(
        "Rename something that never existed",
        vec![Operation::Rename {
            element: ElementId::from_raw(10_000),
            name: "x".into(),
        }],
    )));
    assert_eq!(reason, "element #10000 does not exist");
}

#[test]
fn a_duplicate_name_is_applied_and_reported_at_the_element() {
    let mut state = state();
    let web = id(&state, "Shop::Web");
    state
        .apply(operator(
            "Rename Web to Store",
            vec![Operation::Rename {
                element: web,
                name: "Store".into(),
            }],
        ))
        .unwrap();
    let codes: Vec<_> = state.diagnostics_for(web).map(|d| d.code).collect();
    assert_eq!(codes, vec!["duplicate-name"]);
}

#[test]
fn move_across_owners_and_documents_reports_the_moved_element() {
    let tree = parse(&[
        Source::new("shop.sysml", MODEL),
        Source::new("more.sysml", "package More { part def Extra; }"),
    ]);
    let mut state = SystemState::new(tree, BTreeSet::new());
    let more = id(&state, "More");
    let order = id(&state, "Shop::Order");
    let store = id(&state, "Shop::Store");
    let event = state
        .apply(operator(
            "Move More into the first document",
            vec![Operation::Move {
                element: more,
                parent: Parent::Document(0),
            }],
        ))
        .unwrap();
    assert_eq!(event.updated, vec![more]);
    assert_eq!(state.tree().document_of(more), Some(0));

    let event = state
        .apply(operator(
            "Move Order into More",
            vec![Operation::Move {
                element: order,
                parent: Parent::Element(more),
            }],
        ))
        .unwrap();
    let shop = id(&state, "Shop");
    for changed in [order, shop, more] {
        assert!(event.updated.contains(&changed), "{:?}", event.updated);
    }
    assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
    let text = print(state.tree())[0].text.clone();
    assert!(text.contains("in item order : More::Order;"), "{text}");
    assert_survives_saving(&state);

    let reason = rejected_reason(state.apply(operator(
        "Move Store into its own port",
        vec![Operation::Move {
            element: store,
            parent: Parent::Element(id(&state, "Shop::Store::orders")),
        }],
    )));
    assert_eq!(
        reason,
        "`Shop::Store` cannot be moved into itself or into `Shop::Store::orders`, which it owns"
    );
}

#[test]
fn delete_removes_the_subtree_and_reports_references_to_it() {
    let mut state = state();
    let store = id(&state, "Shop::Store");
    let store_port = id(&state, "Shop::Store::orders");
    let usage = id(&state, "Shop::System::store");
    state
        .apply(operator(
            "Lock",
            vec![Operation::Lock {
                element: store_port,
            }],
        ))
        .unwrap();
    let mut delete = operator(
        "Delete Store and its port",
        vec![
            Operation::Delete { element: store },
            Operation::Delete {
                element: store_port,
            },
        ],
    );
    delete.confirmed = vec![store_port];
    let event = state.apply(delete).unwrap();
    assert_eq!(event.deleted, vec![store, store_port]);
    assert!(state.locks().is_empty(), "the lock went with the element");
    let codes: Vec<_> = state.diagnostics_for(usage).map(|d| d.code).collect();
    assert_eq!(codes, vec!["unresolved"]);

    let undone = state.undo().unwrap();
    assert_eq!(undone.created, vec![store, store_port]);
    assert_eq!(state.locks(), &BTreeSet::from([store_port]));
    assert!(state.diagnostics().is_empty());
}

#[test]
fn connect_resolves_feature_chains_and_reports_ends_that_do_not_fit() {
    let mut state = state();
    let shop = id(&state, "Shop");
    let system = id(&state, "Shop::System");
    let connect = |name: &str, definition: Option<&str>, from: &str, to: &str| Operation::Connect {
        parent: system,
        kind: ElementKind::Interface,
        name: Some(name.into()),
        definition: definition.map(Reference::new),
        from: Reference::new(from),
        to: Reference::new(to),
    };
    let event = state
        .apply(operator(
            "Define and use an interface",
            vec![
                Operation::Create {
                    parent: Parent::Element(shop),
                    element: Box::new(Element::named(ElementKind::InterfaceDef, "Ordering")),
                },
                connect("ordering", Some("Ordering"), "web.orders", "store.orders"),
                connect("backwards", None, "web.orders", "web.orders"),
                connect("nowhere", None, "web.missing", "store.orders"),
            ],
        ))
        .unwrap();
    let [definition, ordering, backwards, nowhere] = event.created[..] else {
        panic!("{:?}", event.created);
    };
    assert_eq!(state.tree()[definition].name.as_deref(), Some("Ordering"));
    let codes = |element| {
        state
            .diagnostics_for(element)
            .map(|d| d.code)
            .collect::<Vec<_>>()
    };
    // `Ordering` has no ends yet, so the typed interface cannot fit it.
    assert_eq!(codes(ordering), vec!["incompatible-ends"]);
    assert_eq!(codes(backwards), vec!["incompatible-ends"]);
    assert_eq!(codes(nowhere), vec!["unresolved"]);
    let end = &state.tree()[ordering].ends[0];
    assert!(end.is_linked() && end.steps.len() == 2);
    assert_survives_saving(&state);

    let reason = rejected_reason(state.apply(operator(
        "Connect as a part",
        vec![Operation::Connect {
            parent: system,
            kind: ElementKind::Part,
            name: None,
            definition: None,
            from: Reference::new("web"),
            to: Reference::new("store"),
        }],
    )));
    assert_eq!(
        reason,
        "a connection is a connection or an interface, not a part"
    );
    let reason = rejected_reason(state.apply(operator(
        "Connect with an empty end",
        vec![connect("empty", None, "", "store.orders")],
    )));
    assert_eq!(
        reason,
        "`Shop::System::empty`: in the connection end, a name cannot be empty"
    );
}

#[test]
fn every_property_can_be_set_and_cleared_and_survives_saving() {
    let mut state = state();
    let shop = id(&state, "Shop");
    let system = id(&state, "Shop::System");
    let store_def = id(&state, "Shop::Store");
    let order = id(&state, "Shop::Order");
    let store = id(&state, "Shop::System::store");
    let item = id(&state, "Shop::OrderPort::order");
    let web_port = id(&state, "Shop::Web::orders");
    let with = |mut element: Element, target: &str, wildcard: bool| {
        element.target = Some(Reference::new(target));
        element.wildcard = wildcard;
        Box::new(element)
    };
    let event = state
        .apply(operator(
            "Add a requirement, a satisfy, an attribute and an import",
            vec![
                Operation::Create {
                    parent: Parent::Element(shop),
                    element: Box::new(Element::named(ElementKind::Requirement, "fast")),
                },
                Operation::Create {
                    parent: Parent::Element(system),
                    element: with(Element::new(ElementKind::Satisfy), "fast", false),
                },
                Operation::Create {
                    parent: Parent::Element(store_def),
                    element: Box::new(Element::named(ElementKind::Attribute, "capacity")),
                },
                Operation::Create {
                    parent: Parent::Element(shop),
                    element: with(Element::new(ElementKind::Import), "ScalarValues", true),
                },
            ],
        ))
        .unwrap();
    let [_, satisfy, capacity, import] = event.created[..] else {
        panic!("{:?}", event.created);
    };
    let values = vec![
        set(
            store,
            Property::Multiplicity(Some(Multiplicity {
                lower: 1,
                upper: Some(3),
            })),
        ),
        set(store, Property::Specializes(vec![Reference::new("web")])),
        set(store, Property::Abstract(true)),
        set(store, Property::Visibility(Visibility::Private)),
        set(item, Property::Direction(Some(Direction::Out))),
        set(capacity, Property::TypedBy(vec![Reference::new("Integer")])),
        set(
            capacity,
            Property::Value(Some(Literal::Integer("-12".into()))),
        ),
        set(
            store_def,
            Property::Specializes(vec![Reference::new("Web")]),
        ),
        set(
            store_def,
            Property::Doc(Some("Keeps orders.\nTwo lines.".into())),
        ),
        set(satisfy, Property::By(Some(Reference::new("store")))),
        set(satisfy, Property::Target(Some(Reference::new("fast")))),
        set(
            import,
            Property::Target(Some(Reference::new("ScalarValues"))),
        ),
        set(import, Property::Visibility(Visibility::Private)),
        set(web_port, Property::Conjugated(true)),
    ];
    state.apply(operator("Set everything", values)).unwrap();
    let text = print(state.tree())[0].text.clone();
    for expected in [
        "private abstract part store : Store :> web[1..3];",
        "out item order : Order;",
        "attribute capacity : Integer = -12;",
        "part def Store :> Web {",
        "doc /* Keeps orders.",
        "satisfy fast by store;",
        "private import ScalarValues::*;",
        "port orders : ~OrderPort;",
    ] {
        assert!(text.contains(expected), "{expected}\n{text}");
    }
    assert_survives_saving(&state);

    let clear = vec![
        set(store, Property::Multiplicity(None)),
        set(store, Property::Specializes(Vec::new())),
        set(store, Property::Abstract(false)),
        set(store, Property::Visibility(Visibility::Public)),
        set(item, Property::Direction(None)),
        set(capacity, Property::Value(None)),
        set(capacity, Property::TypedBy(Vec::new())),
        set(store_def, Property::Specializes(Vec::new())),
        set(store_def, Property::Doc(None)),
        set(satisfy, Property::By(None)),
        set(order, Property::Doc(None)),
        set(web_port, Property::Conjugated(false)),
    ];
    state.apply(operator("Clear everything", clear)).unwrap();
    let text = print(state.tree())[0].text.clone();
    for expected in [
        "        part store : Store;",
        "        item order : Order;",
        "attribute capacity;",
        "part def Store {",
        "satisfy fast;",
        "port orders : OrderPort;",
    ] {
        assert!(text.contains(expected), "{expected}\n{text}");
    }
    assert!(!text.contains("doc"), "{text}");
    assert_survives_saving(&state);
}

#[test]
fn properties_an_element_cannot_have_are_rejected_by_name() {
    let mut state = state();
    let store_def = id(&state, "Shop::Store");
    let store = id(&state, "Shop::System::store");
    let web_port = id(&state, "Shop::Web::orders");
    let reason = |state: &mut SystemState, operation| {
        rejected_reason(state.apply(operator("Set", vec![operation])))
    };
    assert_eq!(
        reason(
            &mut state,
            set(store_def, Property::TypedBy(vec![Reference::new("Web")]))
        ),
        "`Shop::Store`: a part def has no type (`:`)"
    );
    assert_eq!(
        reason(
            &mut state,
            set(store, Property::Ends(vec![Reference::new("a")]))
        ),
        "`Shop::System::store`: a part has no connection end"
    );
    assert_eq!(
        reason(
            &mut state,
            set(store, Property::TypedBy(vec![Reference::new("a.b")]))
        ),
        "`Shop::System::store`: the type (`:`) names an element, not a feature chain like `a.b`"
    );
    for (literal, expected) in [
        (
            Literal::Integer("12a".into()),
            "`12a` is not a whole number",
        ),
        (
            Literal::Real("1".into()),
            "`1` is not a real number such as `1.5` or `2e3`",
        ),
        (
            Literal::String("say \"hi\"".into()),
            "a string value is written with `\\\"` for a quote, `\\\\` for a backslash and `\\n` for a line break",
        ),
    ] {
        let got = reason(&mut state, set(store, Property::Value(Some(literal))));
        assert_eq!(got, format!("`Shop::System::store`: {expected}"));
    }
    // A conjugated port has exactly one type, whatever the order of the sets.
    assert_eq!(
        reason(&mut state, set(web_port, Property::TypedBy(Vec::new()))),
        "`Shop::Web::orders`: a conjugated usage (`~`) has exactly one type"
    );
    state
        .apply(operator(
            "Untype the port",
            vec![
                set(web_port, Property::TypedBy(Vec::new())),
                set(web_port, Property::Conjugated(false)),
            ],
        ))
        .unwrap();
    // Escapes are kept as written.
    let escaped = Literal::String("say \\\"hi\\\"".into());
    state
        .apply(operator(
            "Set a string",
            vec![set(store, Property::Value(Some(escaped)))],
        ))
        .unwrap();
    assert_eq!(state.revision(), 2);
    assert_survives_saving(&state);
}

#[test]
fn locks_are_reported_in_events_and_explained_by_name() {
    let mut state = state();
    let store = id(&state, "Shop::Store");
    let port = id(&state, "Shop::Store::orders");
    let event = state
        .apply(operator("Lock", vec![Operation::Lock { element: store }]))
        .unwrap();
    assert_eq!(event.updated, vec![store]);

    // The Operator's own edits of a locked part need confirmation too.
    let rejection = state
        .apply(operator(
            "Rename the locked port",
            vec![Operation::Rename {
                element: port,
                name: "incoming".into(),
            }],
        ))
        .unwrap_err();
    assert_eq!(
        state.explain(&rejection),
        "`Shop::Store` is locked; changing it needs the Operator's confirmation"
    );
    let reason = rejected_reason(state.apply(operator(
        "Unlock the port",
        vec![Operation::Unlock { element: port }],
    )));
    assert_eq!(
        reason,
        "`Shop::Store::orders` has no lock of its own; it is covered by the lock on `Shop::Store`"
    );
    let event = state
        .apply(operator(
            "Unlock",
            vec![Operation::Unlock { element: store }],
        ))
        .unwrap();
    assert_eq!(event.updated, vec![store]);
    let undone = state.undo().unwrap();
    assert_eq!(undone.updated, vec![store]);
    assert!(state.is_locked(port));
}

#[test]
fn a_rejected_change_leaves_model_and_locks_untouched() {
    let mut state = state();
    let store = id(&state, "Shop::Store");
    let before = print(state.tree());
    let result = state.apply(operator(
        "Lock, rename, delete, then fail",
        vec![
            Operation::Lock { element: store },
            Operation::Rename {
                element: store,
                name: "Warehouse".into(),
            },
            Operation::Delete { element: store },
            Operation::Rename {
                element: store,
                name: "Again".into(),
            },
        ],
    ));
    assert_eq!(
        result,
        Err(Rejection::Invalid {
            operation: 3,
            reason: "`Shop::Store` was removed by an earlier operation of this change".into()
        })
    );
    assert!(state.locks().is_empty());
    assert_eq!(print(state.tree()), before);
    assert_eq!(state.revision(), 0);
    assert!(state.undo_description().is_none());
}

#[test]
fn with_base_detects_a_change_made_in_between() {
    let mut state = state();
    let store = id(&state, "Shop::Store");
    let rename = |name: &str| {
        operator(
            "Rename",
            vec![Operation::Rename {
                element: store,
                name: name.into(),
            }],
        )
        .with_base(0)
    };
    state.apply(rename("Warehouse")).unwrap();
    let rejection = state.apply(rename("Depot")).unwrap_err();
    assert_eq!(
        rejection,
        Rejection::Stale {
            base: 0,
            current: 1
        }
    );
    assert!(rejection.to_string().contains("prepare it again"));
}

#[test]
fn undo_since_reverts_the_assistants_work_step_by_step() {
    let mut state = state();
    let shop = id(&state, "Shop");
    let store = id(&state, "Shop::Store");
    let order = id(&state, "Shop::Order");
    state
        .apply(operator(
            "Operator rename",
            vec![Operation::Rename {
                element: store,
                name: "Warehouse".into(),
            }],
        ))
        .unwrap();
    let started = state.revision();
    let assistant =
        |description: &str, operations| Change::new(Actor::Assistant, description, operations);
    let cache = state
        .apply(assistant(
            "Add a cache",
            vec![Operation::Create {
                parent: Parent::Element(shop),
                element: Box::new(Element::named(ElementKind::PartDef, "Cache")),
            }],
        ))
        .unwrap()
        .created[0];
    state
        .apply(assistant(
            "Lock the cache",
            vec![Operation::Lock { element: cache }],
        ))
        .unwrap();
    state
        .apply(assistant(
            "Delete Order",
            vec![Operation::Delete { element: order }],
        ))
        .unwrap();

    let events = state.undo_since(started);
    let descriptions: Vec<_> = events.iter().map(|e| e.description.as_str()).collect();
    assert_eq!(
        descriptions,
        ["Delete Order", "Lock the cache", "Add a cache"]
    );
    assert!(
        events
            .iter()
            .all(|e| e.kind == EventKind::Undone && e.actor == Actor::Assistant)
    );
    assert_eq!(events[0].created, vec![order]);
    assert_eq!(events[1].updated, vec![cache]);
    assert_eq!(events[2].deleted, vec![cache]);
    assert!(state.locks().is_empty());
    assert_eq!(state.undo_description(), Some("Operator rename"));
    assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());

    // Undone work can be redone, and redone work counts as new.
    state.redo().unwrap();
    assert_eq!(state.undo_since(state.revision() - 1).len(), 1);
    assert!(state.undo_since(state.revision()).is_empty());
}

#[test]
fn load_replaces_everything_and_clears_undo() {
    let mut state = state();
    let store = id(&state, "Shop::Store");
    state
        .apply(operator("Lock", vec![Operation::Lock { element: store }]))
        .unwrap();
    let tree = parse(&[Source::new("other.sysml", "package Other { part def A; }")]);
    let a = tree.find("Other::A").unwrap();
    let event = state.load(
        tree,
        BTreeSet::from([a, ElementId::from_raw(999)]),
        "Open Other",
    );
    assert_eq!(event.kind, EventKind::Loaded);
    assert_eq!(event.revision, 2);
    assert_eq!(
        state.locks(),
        &BTreeSet::from([a]),
        "locks on missing elements are dropped"
    );
    assert!(state.undo().is_none() && state.redo().is_none());
}

#[test]
fn a_doc_set_as_a_property_is_reported_as_created() {
    let mut state = state();
    let store = id(&state, "Shop::Store");
    let event = state
        .apply(operator(
            "Document the store",
            vec![set(store, Property::Doc(Some("Keeps orders.".into())))],
        ))
        .unwrap();
    let doc = state.tree()[store].children()[0];
    assert_eq!(event.created, vec![doc]);
    assert_eq!(event.updated, vec![store]);
}

#[test]
fn what_would_not_read_back_is_rejected() {
    use ElementKind::*;
    let mut state = state();
    let shop = id(&state, "Shop");
    let create = |element: Element| {
        operator(
            "Create",
            vec![Operation::Create {
                parent: Parent::Element(shop),
                element: Box::new(element),
            }],
        )
    };
    // `: Order;` would read back as a syntax error.
    let mut typed = Element::new(Reference);
    typed.typed_by = vec![agq_language::Reference::new("Order")];
    let reason = rejected_reason(state.apply(create(typed)));
    assert!(
        reason.contains("a usage without a kind keyword needs a name"),
        "{reason}"
    );
    // Names on elements that are written without one.
    for kind in [Satisfy, Import, Doc, Comment] {
        let mut element = Element::named(kind, "named");
        element.target = Some(agq_language::Reference::new("Order"));
        element.text = matches!(kind, Doc | Comment).then(|| "text".to_string());
        let reason = rejected_reason(state.apply(create(element)));
        assert!(reason.ends_with("has no name"), "{reason}");
    }
    let mut comment = Element::new(Comment);
    let reason = rejected_reason(state.apply(create(comment.clone())));
    assert!(
        reason.ends_with("a comment needs text (it may be empty)"),
        "{reason}"
    );
    comment.text = Some("ends */ early".into());
    let reason = rejected_reason(state.apply(create(comment)));
    assert!(
        reason.ends_with("comment text cannot contain `*/`"),
        "{reason}"
    );
    assert_eq!(state.revision(), 0);

    // An end connection keeps its `end` when it is saved.
    let mut end = Element::new(Connection);
    end.is_end = true;
    end.ends = vec![
        agq_language::Reference::new("a"),
        agq_language::Reference::new("b"),
    ];
    state.apply(create(end)).unwrap();
    assert_survives_saving(&state);
}

#[test]
fn a_lock_on_a_doc_comment_covers_setting_the_doc() {
    let mut state = state();
    let store = id(&state, "Shop::Store");
    state
        .apply(operator(
            "Document",
            vec![set(store, Property::Doc(Some("Keeps orders.".into())))],
        ))
        .unwrap();
    let doc = state.tree()[store].children()[0];
    state
        .apply(operator(
            "Lock the doc",
            vec![Operation::Lock { element: doc }],
        ))
        .unwrap();
    let assistant = |text: Option<&str>| {
        Change::new(
            Actor::Assistant,
            "Rewrite the doc",
            vec![set(store, Property::Doc(text.map(str::to_string)))],
        )
    };
    let locked = Err(Rejection::Locked {
        elements: vec![doc],
    });
    assert_eq!(state.apply(assistant(Some("Anything."))), locked);
    assert_eq!(state.apply(assistant(None)), locked);
    let mut confirmed = assistant(None);
    confirmed.confirmed = vec![doc];
    let event = state.apply(confirmed).unwrap();
    assert_eq!(event.deleted, vec![doc]);
    assert!(state.locks().is_empty(), "the lock went with the doc");
}

#[test]
fn deleting_a_doc_that_clearing_the_doc_removed_does_nothing() {
    let mut state = state();
    let store = id(&state, "Shop::Store");
    state
        .apply(operator(
            "Document",
            vec![set(store, Property::Doc(Some("Keeps orders.".into())))],
        ))
        .unwrap();
    let doc = state.tree()[store].children()[0];
    let event = state
        .apply(operator(
            "Clear the doc and delete it",
            vec![
                set(store, Property::Doc(None)),
                Operation::Delete { element: doc },
            ],
        ))
        .unwrap();
    assert_eq!(event.deleted, vec![doc]);
}

#[test]
fn moving_a_top_level_element_within_its_document_is_an_update() {
    let text = "package A; package B; package C;";
    let mut state = SystemState::new(parse(&[Source::new("abc.sysml", text)]), BTreeSet::new());
    let a = id(&state, "A");
    let event = state
        .apply(operator(
            "Move A to the end",
            vec![Operation::Move {
                element: a,
                parent: Parent::Document(0),
            }],
        ))
        .unwrap();
    assert_eq!(event.updated, vec![a], "only the moved package");
    assert_eq!(state.undo().unwrap().updated, vec![a]);
    // Deleting a package moves nothing else.
    let b = id(&state, "B");
    let event = state
        .apply(operator("Delete B", vec![Operation::Delete { element: b }]))
        .unwrap();
    assert!(event.updated.is_empty(), "{:?}", event.updated);
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

#[test]
fn ids_of_undone_elements_are_never_handed_out_again() {
    let mut state = state();
    let shop = id(&state, "Shop");
    let create = |name: &str| {
        operator(
            "Create",
            vec![Operation::Create {
                parent: Parent::Element(shop),
                element: Box::new(Element::named(ElementKind::PartDef, name)),
            }],
        )
    };
    let first = state.apply(create("First")).unwrap().created[0];
    let before_undo = state.tree().clone();
    state.undo().unwrap();
    let second = state.apply(create("Second")).unwrap().created[0];
    assert_ne!(first, second);
    // Between the two versions, First was deleted and Second created.
    let comparison = compare(&before_undo, state.tree());
    assert_eq!(comparison.deleted, vec![first]);
    assert_eq!(comparison.created, vec![second]);

    // Redo keeps the id it had; new ids stay above it.
    state.undo().unwrap();
    assert_eq!(state.redo().unwrap().created, vec![second]);
    let third = state.apply(create("Third")).unwrap().created[0];
    assert!(third.raw() > second.raw() && third != first);
}
