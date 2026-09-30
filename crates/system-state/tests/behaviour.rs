//! Behaviour and implementation links through the System State (C-50):
//! behaviour elements are created and changed by the same typed operations,
//! names inside expressions keep their targets and unbind on deletion like
//! any reference, and `model/links.json` is saved, reopened and committed
//! with the model.
use agq_language::{
    BinaryOp, Element, ElementId, ElementKind, Expression, Literal, Parent, Reference, StateAction,
};
use agq_system_state::{Actor, ApplyError, Change, Operation, Project, Property, Rejection};

fn id(project: &Project, name: &str) -> ElementId {
    project
        .state()
        .tree()
        .find(name)
        .unwrap_or_else(|| panic!("no {name}"))
}

fn apply(project: &mut Project, operations: Vec<Operation>) -> Vec<ElementId> {
    project
        .apply(Change::new(Actor::Operator, "Edit", operations))
        .unwrap()
        .created
}

fn create(parent: ElementId, element: Element) -> Operation {
    Operation::Create {
        parent: Parent::Element(parent),
        element: Box::new(element),
    }
}

fn codes(project: &Project) -> Vec<(String, &'static str)> {
    project
        .state()
        .diagnostics()
        .iter()
        .map(|d| (project.state().tree().qualified_name(d.element), d.code))
        .collect()
}

/// `Counter` with an attribute `count`, and a state machine that counts
/// ticks while `count < limit`.
fn counter() -> (tempfile::TempDir, std::path::PathBuf, Project) {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("counter");
    let mut project = Project::create(&folder, "Counter").unwrap();
    let package = id(&project, "Counter");
    let mut tick = Element::named(ElementKind::ItemDef, "Tick");
    tick.visibility = agq_language::Visibility::Public;
    let mut port_def = Element::named(ElementKind::PortDef, "TickPort");
    port_def.visibility = agq_language::Visibility::Public;
    let created = apply(
        &mut project,
        vec![
            create(package, tick),
            create(package, port_def),
            create(package, Element::named(ElementKind::PartDef, "Counter")),
        ],
    );
    let (port_def, counter) = (created[1], created[2]);
    let mut input = Element::named(ElementKind::Item, "tick");
    input.direction = Some(agq_language::Direction::In);
    input.typed_by = vec![Reference::new("Tick")];
    let mut port = Element::named(ElementKind::Port, "ticks");
    port.typed_by = vec![Reference::new("TickPort")];
    let mut count = Element::named(ElementKind::Attribute, "count");
    count.typed_by = vec![Reference::new("ScalarValues::Natural")];
    count.value = Some(Literal::Integer("0".into()));
    let mut limit = Element::named(ElementKind::Attribute, "limit");
    limit.typed_by = vec![Reference::new("ScalarValues::Natural")];
    limit.value = Some(Literal::Integer("3".into()));
    let mut machine = Element::named(ElementKind::State, "counting");
    machine.exhibit = true;
    let created = apply(
        &mut project,
        vec![
            create(port_def, input),
            create(counter, port),
            create(counter, count),
            create(counter, limit),
            create(counter, machine),
        ],
    );
    let machine = created[4];
    let mut entry = Element::new(ElementKind::Action);
    entry.state_action = Some(StateAction::Entry);
    let mut first = Element::new(ElementKind::Succession);
    first.target = Some(Reference::new("idle"));
    let mut transition = Element::new(ElementKind::Transition);
    transition.ends = vec![Reference::new("idle"), Reference::new("idle")];
    transition.guard = Some(Expression::binary(
        BinaryOp::Less,
        Expression::name("count"),
        Expression::name("limit"),
    ));
    let created = apply(
        &mut project,
        vec![
            create(machine, entry),
            create(machine, first),
            create(machine, Element::named(ElementKind::State, "idle")),
            create(machine, transition),
        ],
    );
    let transition = created[3];
    let mut trigger = Element::named(ElementKind::Accept, "t");
    trigger.typed_by = vec![Reference::new("Tick")];
    trigger.via = Some(Reference::new("ticks"));
    let mut effect = Element::new(ElementKind::Assign);
    effect.target = Some(Reference::new("count"));
    effect.expression = Some(Expression::binary(
        BinaryOp::Add,
        Expression::name("count"),
        Expression::Literal(Literal::Integer("1".into())),
    ));
    apply(
        &mut project,
        vec![create(transition, trigger), create(transition, effect)],
    );
    (dir, folder, project)
}

#[test]
fn behaviour_is_built_by_typed_operations_and_survives_reopening() {
    let (_dir, folder, project) = counter();
    assert_eq!(codes(&project), []);
    let text = agq_language::print(project.state().tree())[0].text.clone();
    assert!(
        text.contains(
            "transition first idle accept t : Tick via ticks if count < limit do assign count := count + 1 then idle;"
        ),
        "{text}"
    );
    let transition =
        project.state().tree()[id(&project, "Counter::Counter::counting")].children()[3];
    drop(project);
    let project = Project::open(&folder).unwrap();
    assert_eq!(project.unmatched(), [] as [String; 0]);
    let machine = id(&project, "Counter::Counter::counting");
    assert_eq!(
        project.state().tree()[machine].children()[3],
        transition,
        "identity kept"
    );
    assert_eq!(codes(&project), []);
}

#[test]
fn a_name_in_a_guard_is_a_reference_like_any_other() {
    let (_dir, _folder, mut project) = counter();
    let limit = id(&project, "Counter::Counter::limit");
    // Renamed: the guard follows.
    apply(
        &mut project,
        vec![Operation::Rename {
            element: limit,
            name: "most".into(),
        }],
    );
    let text = agq_language::print(project.state().tree())[0].text.clone();
    assert!(text.contains("if count < most"), "{text}");
    // Deleted: the guard keeps the name and reports it, as after reopening.
    apply(&mut project, vec![Operation::Delete { element: limit }]);
    assert!(
        codes(&project)
            .iter()
            .any(|(at, code)| at.contains("counting") && *code == "unresolved"),
        "{:?}",
        codes(&project)
    );
    // An attribute with that name binds it again.
    let counter = id(&project, "Counter::Counter");
    let mut most = Element::named(ElementKind::Attribute, "most");
    most.typed_by = vec![Reference::new("ScalarValues::Natural")];
    apply(&mut project, vec![create(counter, most)]);
    assert_eq!(codes(&project), []);
}

#[test]
fn a_guard_and_a_trigger_change_through_set_and_undo() {
    let (_dir, _folder, mut project) = counter();
    let machine = id(&project, "Counter::Counter::counting");
    let transition = project.state().tree()[machine].children()[3];
    apply(
        &mut project,
        vec![Operation::Set {
            element: transition,
            property: Property::Guard(Some(Expression::Literal(Literal::Boolean(true)))),
        }],
    );
    let text = agq_language::print(project.state().tree())[0].text.clone();
    assert!(text.contains("if true do assign"), "{text}");
    project.undo().unwrap();
    let text = agq_language::print(project.state().tree())[0].text.clone();
    assert!(text.contains("if count < limit"), "{text}");
}

#[test]
fn behaviour_that_would_not_read_back_is_refused() {
    let (_dir, _folder, mut project) = counter();
    let machine = id(&project, "Counter::Counter::counting");
    let result = project.apply(Change::new(
        Actor::Assistant,
        "Send nothing",
        vec![create(machine, Element::new(ElementKind::Send))],
    ));
    let Err(ApplyError::Rejection(Rejection::Invalid { reason, .. })) = result else {
        panic!("{result:?}");
    };
    assert!(reason.contains("a send needs what it sends"), "{reason}");
    let transition = project.state().tree()[machine].children()[3];
    let result = project.apply(Change::new(
        Actor::Operator,
        "Second effect",
        vec![create(transition, Element::new(ElementKind::Action))],
    ));
    assert!(
        matches!(
            result,
            Err(ApplyError::Rejection(Rejection::Invalid { .. }))
        ),
        "{result:?}"
    );
}

#[test]
fn implementation_links_are_saved_reopened_and_committed_with_the_model() {
    let (_dir, folder, mut project) = counter();
    assert_eq!(project.links(), None);
    let links = "{\n  \"format\": 1,\n  \"links\": []\n}\n";
    project.save_links(Some(links.into())).unwrap();
    assert!(project.has_uncommitted_changes().unwrap());
    project.checkpoint("Link the code").unwrap();
    drop(project);
    assert_eq!(
        std::fs::read_to_string(folder.join("model").join("links.json")).unwrap(),
        links
    );
    let mut project = Project::open(&folder).unwrap();
    assert_eq!(project.links(), Some(links));
    assert!(!project.has_uncommitted_changes().unwrap());
    // A model change keeps the links.
    let counter = id(&project, "Counter::Counter");
    apply(
        &mut project,
        vec![Operation::Rename {
            element: counter,
            name: "Tally".into(),
        }],
    );
    assert_eq!(project.links(), Some(links));
    project.save_links(None).unwrap();
    assert!(!folder.join("model").join("links.json").exists());
}
