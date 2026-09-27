//! Projects on disk: close and reopen, identities through rename, move and
//! hand edits, interrupted saves, checkpoints and what changed between them,
//! branches, and one window per project.
use agq_history::History;
use agq_language::{Element, ElementId, ElementKind, Parent, Reference, Tree};
use agq_system_state::{
    Actor, ApplyError, Change, Comparison, EventKind, Operation, Project, ProjectError, Property,
    compare,
};
use std::fs;
use std::path::{Path, PathBuf};

fn id(project: &Project, name: &str) -> ElementId {
    project
        .state()
        .tree()
        .find(name)
        .unwrap_or_else(|| panic!("no {name}"))
}

fn change(project: &mut Project, operation: Operation) {
    project
        .apply(Change::new(Actor::Operator, "Edit", vec![operation]))
        .unwrap();
}

fn add(project: &mut Project, parent: ElementId, element: Element) -> ElementId {
    let operation = Operation::Create {
        parent: Parent::Element(parent),
        element: Box::new(element),
    };
    project
        .apply(Change::new(Actor::Operator, "Add", vec![operation]))
        .unwrap()
        .created[0]
}

fn part(name: &str, typed_by: ElementId, type_name: &str) -> Element {
    let mut part = Element::named(ElementKind::Part, name);
    part.typed_by = vec![Reference::to(typed_by, type_name)];
    part
}

fn sorted(mut ids: Vec<ElementId>) -> Vec<ElementId> {
    ids.sort();
    ids
}

/// A new project `Shop` built by hand: Store (with capacity), Api, and a
/// System with a documented part of each.
fn shop() -> (tempfile::TempDir, PathBuf, Project) {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("shop");
    let mut project = Project::create(&folder, "Shop").unwrap();
    let shop = id(&project, "Shop");
    let store = add(
        &mut project,
        shop,
        Element::named(ElementKind::PartDef, "Store"),
    );
    add(
        &mut project,
        store,
        Element::named(ElementKind::Attribute, "capacity"),
    );
    let api = add(
        &mut project,
        shop,
        Element::named(ElementKind::PartDef, "Api"),
    );
    let system = add(
        &mut project,
        shop,
        Element::named(ElementKind::PartDef, "System"),
    );
    add(&mut project, system, part("store", store, "Store"));
    add(&mut project, system, part("api", api, "Api"));
    change(
        &mut project,
        Operation::Set {
            element: system,
            property: Property::Doc(Some("The whole shop.".into())),
        },
    );
    assert!(project.state().diagnostics().is_empty());
    (dir, folder, project)
}

fn model_file(folder: &Path) -> PathBuf {
    folder.join("model/Shop.sysml")
}

#[test]
fn reopening_restores_the_model_its_identities_and_locks() {
    let (_dir, folder, mut project) = shop();
    let store = id(&project, "Shop::Store");
    change(&mut project, Operation::Lock { element: store });
    let before = project.state().tree().clone();
    let locks = project.state().locks().clone();
    drop(project); // close

    let text = fs::read_to_string(model_file(&folder)).unwrap();
    assert!(text.contains("part store : Store;"), "{text}");
    let project = Project::open(&folder).unwrap();
    assert!(project.unmatched().is_empty(), "{:?}", project.unmatched());
    assert_eq!(
        compare(&before, project.state().tree()),
        Comparison::default()
    );
    assert_eq!(project.state().locks(), &locks);
    assert!(
        project
            .state()
            .is_locked(id(&project, "Shop::Store::capacity"))
    );
    assert!(project.state().diagnostics().is_empty());
}

#[test]
fn renames_and_moves_keep_identities_through_reopen_and_checkpoints() {
    let (_dir, folder, mut project) = shop();
    let store = id(&project, "Shop::Store");
    let capacity = id(&project, "Shop::Store::capacity");
    let api = id(&project, "Shop::Api");
    let first = project.checkpoint("Add the shop").unwrap();

    change(
        &mut project,
        Operation::Rename {
            element: store,
            name: "LinkStore".into(),
        },
    );
    change(
        &mut project,
        Operation::Move {
            element: capacity,
            parent: Parent::Element(api),
        },
    );
    let second = project.checkpoint("Rename Store; move capacity").unwrap();
    drop(project);

    let project = Project::open(&folder).unwrap();
    assert!(project.unmatched().is_empty(), "{:?}", project.unmatched());
    assert_eq!(id(&project, "Shop::LinkStore"), store);
    assert_eq!(id(&project, "Shop::Api::capacity"), capacity);
    let usage = id(&project, "Shop::System::store");
    assert_eq!(
        project.state().tree()[usage].typed_by[0].target(),
        Some(store)
    );

    let at_first = project.tree_at(&first.id).unwrap();
    assert_eq!(at_first[store].name.as_deref(), Some("Store"));
    assert_eq!(at_first[capacity].owner(), Some(store));
    let at_second = project.tree_at(&second.id).unwrap();
    let changed = compare(&at_first, &at_second);
    assert_eq!(changed.created, []);
    assert_eq!(changed.deleted, []);
    // The usage typed by Store is now written `: LinkStore` but still means
    // the same element, so it is not a change.
    assert_eq!(sorted(changed.updated), sorted(vec![store, capacity, api]));
}

#[test]
fn a_hand_edited_name_is_reported_and_never_matched_by_name() {
    let (_dir, folder, mut project) = shop();
    let store = id(&project, "Shop::Store");
    let capacity = id(&project, "Shop::Store::capacity");
    let usage = id(&project, "Shop::System::store");
    // After a deletion the newest element's id is above the element count.
    let api_usage = id(&project, "Shop::System::api");
    change(&mut project, Operation::Delete { element: api_usage });
    let shop = id(&project, "Shop");
    let cache = add(
        &mut project,
        shop,
        Element::named(ElementKind::PartDef, "Cache"),
    );
    drop(project);

    let path = model_file(&folder);
    let text = fs::read_to_string(&path)
        .unwrap()
        .replace("part def Store", "part def Warehouse")
        .replace(": Store;", ": Warehouse;")
        .replace("part def Cache", "part def Buffer");
    fs::write(&path, text).unwrap();

    let mut project = Project::open(&folder).unwrap();
    assert_eq!(
        project.unmatched(),
        [
            "part def Shop::Warehouse",
            "attribute Shop::Warehouse::capacity",
            "part def Shop::Buffer"
        ]
    );
    let warehouse = id(&project, "Shop::Warehouse");
    let buffer = id(&project, "Shop::Buffer");
    let old = [store, capacity, cache];
    assert!(!old.contains(&warehouse) && !old.contains(&buffer));
    let tree = project.state().tree();
    assert!(old.iter().all(|id| !tree.contains(*id)));
    // Everything else keeps its identity; the usage now means Warehouse.
    assert_eq!(id(&project, "Shop::System::store"), usage);
    assert_eq!(tree[usage].typed_by[0].target(), Some(warehouse));
    // The old ids stay retired: a new element never takes one.
    let queue = add(
        &mut project,
        shop,
        Element::named(ElementKind::PartDef, "Queue"),
    );
    assert!(!old.contains(&queue));
    drop(project);

    // The new identities were saved: nothing is unmatched any more.
    let project = Project::open(&folder).unwrap();
    assert!(project.unmatched().is_empty(), "{:?}", project.unmatched());
    assert_eq!(id(&project, "Shop::Warehouse"), warehouse);
}

#[test]
fn an_edit_made_outside_while_the_project_is_open_is_not_overwritten() {
    let (_dir, folder, mut project) = shop();
    let path = model_file(&folder);
    let edited = fs::read_to_string(&path).unwrap().replace("Api", "Gateway");
    fs::write(&path, &edited).unwrap();

    let revision = project.state().tree().clone();
    let shop = id(&project, "Shop");
    let error = project
        .apply(Change::new(
            Actor::Operator,
            "Add Cache",
            vec![Operation::Create {
                parent: Parent::Element(shop),
                element: Box::new(Element::named(ElementKind::PartDef, "Cache")),
            }],
        ))
        .unwrap_err();
    assert!(
        matches!(&error, ApplyError::Project(ProjectError::ChangedOnDisk(paths)) if paths == &["Shop.sysml"]),
        "{error}"
    );
    // The model is as it was, in the app and on disk.
    assert_eq!(
        compare(&revision, project.state().tree()),
        Comparison::default()
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), edited);
}

#[test]
fn an_interrupted_save_opens_to_the_old_or_the_new_state() {
    let (_dir, folder, project) = shop();
    let store = id(&project, "Shop::Store");
    let old: Tree = project.state().tree().clone();
    drop(project);
    let (_, old_files) = History::open(&folder).unwrap();

    // The new state renames, deletes, adds and locks.
    let mut project = Project::open(&folder).unwrap();
    let api = id(&project, "Shop::Api");
    let system = id(&project, "Shop::System");
    project
        .apply(Change::new(
            Actor::Operator,
            "Rework the shop",
            vec![
                Operation::Rename {
                    element: store,
                    name: "LinkStore".into(),
                },
                Operation::Delete { element: api },
                Operation::Create {
                    parent: Parent::Element(system),
                    element: Box::new(Element::named(ElementKind::Part, "cache")),
                },
                Operation::Lock { element: system },
            ],
        ))
        .unwrap();
    let new: Tree = project.state().tree().clone();
    drop(project);
    let (_, new_files) = History::open(&folder).unwrap();

    let (mut saw_old, mut saw_new) = (0, 0);
    for steps in 0.. {
        let (mut history, _) = History::open(&folder).unwrap();
        history.save(&old_files).unwrap();
        let finished = history.save_steps(&new_files, steps).unwrap();
        drop(history); // the app dies here

        let project = Project::open(&folder).unwrap();
        assert!(project.unmatched().is_empty(), "after {steps} steps");
        let tree = project.state().tree();
        if compare(&old, tree) == Comparison::default() {
            assert!(project.state().locks().is_empty());
            saw_old += 1;
        } else {
            assert_eq!(
                compare(&new, tree),
                Comparison::default(),
                "after {steps} steps: neither old nor new"
            );
            assert!(project.state().locks().contains(&system));
            saw_new += 1;
        }
        if finished {
            break;
        }
    }
    assert!(saw_old > 0 && saw_new > 0, "{saw_old} {saw_new}");
}

#[test]
fn checkpoints_show_exactly_what_changed() {
    let (_dir, _folder, mut project) = shop();
    let shop = id(&project, "Shop");
    let store = id(&project, "Shop::Store");
    let capacity = id(&project, "Shop::Store::capacity");
    let api = id(&project, "Shop::Api");
    let added = project.checkpoint("Add the shop").unwrap();
    assert!(!project.has_uncommitted_changes().unwrap());
    assert!(matches!(
        project.checkpoint("Nothing"),
        Err(ProjectError::NoChanges)
    ));

    change(
        &mut project,
        Operation::Rename {
            element: api,
            name: "Gateway".into(),
        },
    );
    change(&mut project, Operation::Delete { element: capacity });
    let cache = add(
        &mut project,
        shop,
        Element::named(ElementKind::PartDef, "Cache"),
    );
    assert!(project.has_uncommitted_changes().unwrap());
    let changed = project
        .checkpoint("Rename Api; remove capacity; add Cache")
        .unwrap();

    let checkpoints = project.checkpoints().unwrap();
    let messages: Vec<&str> = checkpoints.iter().map(|c| c.message.as_str()).collect();
    assert_eq!(
        messages,
        [
            "Rename Api; remove capacity; add Cache",
            "Add the shop",
            "Create Shop"
        ]
    );
    assert_eq!(checkpoints[0], changed);
    assert_eq!(checkpoints[1], added);
    assert!(checkpoints[0].time >= checkpoints[2].time);

    let before = project.tree_at(&added.id).unwrap();
    let after = project.tree_at(&changed.id).unwrap();
    let what = compare(&before, &after);
    assert_eq!(what.created, [cache]);
    assert_eq!(what.deleted, [capacity]);
    assert_eq!(sorted(what.updated), sorted(vec![shop, store, api]));
    assert_eq!(
        compare(&after, project.state().tree()),
        Comparison::default()
    );
}

#[test]
fn branches_have_separate_histories() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("shop");
    let mut project = Project::create(&folder, "Shop").unwrap();
    let shop = id(&project, "Shop");
    assert_eq!(project.current_branch().unwrap(), "main");
    project.create_branch("idea").unwrap();

    let store = add(
        &mut project,
        shop,
        Element::named(ElementKind::PartDef, "Store"),
    );
    assert!(matches!(
        project.switch_branch("idea"),
        Err(ProjectError::UncommittedChanges)
    ));
    assert!(project.state().tree().contains(store));
    project.checkpoint("Add Store").unwrap();

    let event = project.switch_branch("idea").unwrap();
    assert_eq!(event.kind, EventKind::Loaded);
    assert_eq!(event.deleted, [store]);
    assert_eq!(project.current_branch().unwrap(), "idea");
    let cache = add(
        &mut project,
        shop,
        Element::named(ElementKind::PartDef, "Cache"),
    );
    assert_ne!(cache, store, "an id used on another branch is not reused");
    project.checkpoint("Add Cache").unwrap();
    let messages = |project: &Project| -> Vec<String> {
        let checkpoints = project.checkpoints().unwrap();
        checkpoints.into_iter().map(|c| c.message).collect()
    };
    assert_eq!(messages(&project), ["Add Cache", "Create Shop"]);

    let event = project.switch_branch("main").unwrap();
    assert_eq!((event.created, event.deleted), (vec![store], vec![cache]));
    assert_eq!(messages(&project), ["Add Store", "Create Shop"]);
    assert_eq!(project.branches().unwrap(), ["idea", "main"]);
    drop(project);

    let project = Project::open(&folder).unwrap();
    assert_eq!(project.current_branch().unwrap(), "main");
    assert!(project.state().tree().contains(store));
}

#[test]
fn a_second_window_cannot_open_the_same_project() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("shop");
    let first = Project::create(&folder, "Shop").unwrap();
    assert!(matches!(Project::open(&folder), Err(ProjectError::Locked)));
    drop(first);
    Project::open(&folder).unwrap();
}

#[test]
fn creating_refuses_bad_names_and_existing_models() {
    let dir = tempfile::tempdir().unwrap();
    assert!(matches!(
        Project::create(&dir.path().join("a"), "a/b"),
        Err(ProjectError::InvalidName(_))
    ));
    let folder = dir.path().join("shop");
    drop(Project::create(&folder, "Shop").unwrap());
    assert!(matches!(
        Project::create(&folder, "Shop"),
        Err(ProjectError::AlreadyExists(_))
    ));
    assert!(matches!(
        Project::open(&dir.path().join("none")),
        Err(ProjectError::NotAProject(_))
    ));
}

#[test]
fn a_hand_written_model_gets_identities_on_first_open() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("url-shortener");
    fs::create_dir_all(folder.join("model")).unwrap();
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../models/url-shortener/UrlShortener.sysml");
    let text = fs::read_to_string(source).unwrap();
    fs::write(folder.join("model/UrlShortener.sysml"), &text).unwrap();

    let project = Project::open(&folder).unwrap();
    let count = project.state().tree().len();
    assert_eq!(project.unmatched().len(), count);
    assert!(project.state().diagnostics().is_empty());
    let before = project.state().tree().clone();
    drop(project);
    // Opening wrote the identity file and left the text as written.
    assert_eq!(
        fs::read_to_string(folder.join("model/UrlShortener.sysml")).unwrap(),
        text
    );

    let project = Project::open(&folder).unwrap();
    assert!(project.unmatched().is_empty(), "{:?}", project.unmatched());
    assert_eq!(
        compare(&before, project.state().tree()),
        Comparison::default()
    );
}
