//! The Library in the Studio, driven without a window: inserting blocks
//! through the one change path, "What can connect here?", opening
//! definitions and going back, specialising and overriding, creating a block
//! from a selection, conflicts, My Library across projects, and locks.
use super::*;
use crate::commands::CommandId;
use crate::edit::app_tests::studio;
use agq_language::Literal;
use agq_studio_scene::NodeOrigin;
use agq_system_state::{Change, Operation};

fn tree(app: &Studio) -> &Tree {
    app.project.as_ref().unwrap().state().tree()
}

fn find(app: &Studio, name: &str) -> ElementId {
    tree(app)
        .find(name)
        .unwrap_or_else(|| panic!("no {name}: {}", app.status))
}

fn built(name: &str) -> BlockRef {
    BlockRef::new(Scope::BuiltIn, name)
}

/// A part definition `System` in project `P`, selected.
fn system(app: &mut Studio) -> ElementId {
    let package = find(app, "P");
    app.create(
        crate::edit::CreateKind::Part,
        true,
        "System",
        Parent::Element(package),
    );
    let system = find(app, "P::System");
    let target = if app
        .scene
        .target_bounds(&SceneTarget::Container(system))
        .is_some()
    {
        SceneTarget::Container(system)
    } else {
        SceneTarget::Node(system)
    };
    app.select(target, false);
    system
}

#[test]
fn a_block_is_inserted_as_one_undoable_change_with_its_name_ready() {
    let (mut app, _folder) = studio("library-insert");
    let system = system(&mut app);
    let usage = app
        .insert_block(built("Library::Storage::CachedStore"), None, None)
        .expect("inserted");
    let definition = find(&app, "Library::Storage::CachedStore");
    let element = &tree(&app)[usage];
    assert_eq!(element.owner(), Some(system));
    assert_eq!(element.typed_by[0].target(), Some(definition));
    assert!(tree(&app).problems_free(), "{}", app.status);
    // Selected, with its name open for editing, and one step in history.
    assert_eq!(app.selection.primary, Some(SceneTarget::Node(usage)));
    assert!(matches!(app.dialog, Some(Dialog::Rename { element, .. }) if element == usage));
    app.dialog = None;
    let state = app.project.as_ref().unwrap().state();
    assert_eq!(
        state.undo_description(),
        Some("Add cachedStore : CachedStore from the Library")
    );
    assert_eq!(
        app.library.recent[0],
        built("Library::Storage::CachedStore")
    );
    app.execute(CommandId::Undo);
    assert!(tree(&app).find("Library").is_none());
    assert!(tree(&app).get(usage).is_none());
    app.execute(CommandId::Redo);
    assert!(tree(&app).find("Library::Storage::CachedStore").is_some());
}

trait ProblemsFree {
    fn problems_free(&self) -> bool;
}

impl ProblemsFree for Tree {
    fn problems_free(&self) -> bool {
        agq_language::validate(self).is_empty()
    }
}

#[test]
fn what_can_connect_here_offers_fitting_blocks_and_connects_one() {
    let (mut app, _folder) = studio("library-fit");
    let system = system(&mut app);
    let gateway = app
        .insert_block(built("Library::Services::Gateway"), None, None)
        .unwrap();
    app.dialog = None;
    let backend = find(&app, "Library::Services::Gateway::backend");
    app.select(SceneTarget::Port(gateway, backend), false);
    app.execute(CommandId::ConnectFromLibrary);
    assert_eq!(app.palette, Some(PaletteMode::Library { fit: true }));
    let fit = app.library.fit.clone().expect("a fit filter");
    let names: Vec<String> = fit
        .fits
        .iter()
        .map(|f| app.library.index.blocks()[f.block].name.clone())
        .collect();
    assert!(names.contains(&"CachedStore".to_string()), "{names:?}");
    assert!(!names.contains(&"Queue".to_string()), "{names:?}");
    assert_eq!(app.fit_parent(&fit), Some(Parent::Element(system)));
    // Choosing a block inserts it beside the gateway, connected.
    app.palette = None;
    crate::panels::library::insert(&mut app, built("Library::Storage::CachedStore"));
    app.dialog = None;
    assert!(app.library.fit.is_none());
    let text: String = agq_language::print(tree(&app))
        .into_iter()
        .map(|s| s.text)
        .collect();
    assert!(
        text.contains("interface connect gateway.backend to cachedStore.access;"),
        "{text}"
    );
    assert!(tree(&app).problems_free());
}

#[test]
fn opening_a_definition_shows_its_inside_and_back_returns() {
    let (mut app, _folder) = studio("library-drill");
    system(&mut app);
    let usage = app
        .insert_block(built("Library::Storage::CachedStore"), None, None)
        .unwrap();
    app.dialog = None;
    app.select(SceneTarget::Node(usage), false);
    let camera = app.camera;
    app.execute(CommandId::OpenDefinition);
    let definition = find(&app, "Library::Storage::CachedStore");
    assert_eq!(app.focus, Some(definition));
    assert_eq!(
        app.breadcrumbs(),
        ["P".to_string(), "cachedStore : CachedStore".to_string()]
    );
    let cache = find(&app, "Library::Storage::CachedStore::cache");
    assert!(
        app.lookup.node(&app.scene, cache).is_some(),
        "the inside shows"
    );
    assert!(
        app.lookup.node(&app.scene, usage).is_none(),
        "only the inside"
    );
    // Backspace (Back) returns where it was.
    app.execute(CommandId::LeaveFocus);
    assert_eq!(app.focus, None);
    assert!(app.drill.is_empty());
    assert_eq!(app.selection.primary, Some(SceneTarget::Node(usage)));
    assert_eq!(app.camera.center, camera.center);
    // Double-clicking a composite's usage opens it too.
    assert!(is_composite_usage(tree(&app), usage));
}

#[test]
fn a_specialisation_shows_what_it_inherits_and_overrides_leave_the_original() {
    let (mut app, _folder) = studio("library-special");
    system(&mut app);
    let usage = app
        .insert_block(built("Library::Storage::CachedStore"), None, None)
        .unwrap();
    app.dialog = None;
    let cached = find(&app, "Library::Storage::CachedStore");
    app.start_specialize(usage);
    assert!(
        matches!(app.dialog, Some(Dialog::Specialize { definition, .. }) if definition == cached)
    );
    app.dialog = None;
    app.specialize(cached, "SessionStore", Some(usage));
    let special = find(&app, "P::SessionStore");
    assert_eq!(tree(&app)[usage].typed_by[0].target(), Some(special));
    // It opened, showing the inherited parts inside it.
    assert_eq!(app.focus, Some(special));
    let cache = find(&app, "Library::Storage::CachedStore::cache");
    let node = app
        .lookup
        .node(&app.scene, cache)
        .expect("inherited part shown");
    assert_eq!(node.semantic.origin, NodeOrigin::Inherited);
    assert_eq!(node.semantic.owner, Some(special));
    assert!(node.semantic.detail.contains("· from CachedStore"));
    // Override the cache's time to live in the specialisation only.
    let ttl = find(&app, "Library::Storage::Cache::ttlSeconds");
    app.override_feature(
        special,
        &[cache, ttl],
        Override::Value(Literal::Integer("60".into())),
    );
    let redefinition = tree(&app)[special]
        .children()
        .iter()
        .copied()
        .find(|c| !tree(&app)[*c].redefines.is_empty())
        .expect("an override");
    let node = app
        .lookup
        .node(&app.scene, redefinition)
        .expect("override shown");
    assert_eq!(node.semantic.origin, NodeOrigin::Override);
    // Its type is the redefined feature's; the card says it overrides.
    assert!(node.semantic.detail.contains("Cache"));
    assert!(node.semantic.detail.ends_with("· override"));
    assert!(tree(&app).problems_free());
    // The copied block is unchanged: still an unchanged copy.
    app.library_index();
    let index = &app.library.index;
    let block = &index.blocks()[index.project_block(cached).unwrap()];
    assert_eq!(block.source_label(), "Project · from Built-in");
}

#[test]
fn a_selection_becomes_a_building_block_after_showing_its_boundary() {
    let (mut app, folder) = studio("library-extract");
    let model = folder.0.join("P").join("model").join("P.sysml");
    std::fs::write(
        &model,
        "package P {
    item def Query;
    port def Serve { in item query : Query; out item answer : Query; }
    part def Client { port calls : ~Serve; }
    part def Front { port access : Serve; port backend : ~Serve; }
    part def Back { port access : Serve; }
    part def System {
        part client : Client;
        part front : Front;
        part back : Back;
        interface connect client.calls to front.access;
        interface connect front.backend to back.access;
    }
}",
    )
    .unwrap();
    let project = folder.0.join("P");
    app.open_project(&project);
    let (front, back) = (
        find(&app, "P::System::front"),
        find(&app, "P::System::back"),
    );
    app.select(SceneTarget::Node(front), false);
    app.select(SceneTarget::Node(back), true);
    assert!(app.context().parts);
    app.execute(CommandId::CreateBlock);
    let Some(Dialog::ExtractBlock { extraction, .. }) = app.dialog.take() else {
        panic!("the boundary is shown first: {}", app.status)
    };
    assert_eq!(extraction.boundary.len(), 1);
    assert!(app.extract(&extraction, "Backend", ""));
    assert_eq!(find(&app, "P::Backend::front"), front);
    assert!(tree(&app).find("P::System::backend").is_some());
    assert!(tree(&app).problems_free());
    app.execute(CommandId::Undo);
    assert_eq!(find(&app, "P::System::front"), front);
}

#[test]
fn a_different_definition_with_the_same_name_is_asked_about_never_overwritten() {
    let (mut app, folder) = studio("library-conflict");
    let project = folder.0.join("P");
    std::fs::write(
        project.join("model").join("P.sysml"),
        "package P { part def System; }
package Library { package Storage { part def Cache { doc /* Ours. */ } } }",
    )
    .unwrap();
    app.open_project(&project);
    let system = find(&app, "P::System");
    app.select(SceneTarget::Node(system), false);
    assert!(
        app.insert_block(built("Library::Storage::CachedStore"), None, None)
            .is_none()
    );
    let Some(Dialog::LibraryConflict { request, conflicts }) = app.dialog.take() else {
        panic!("a conflict is asked about: {}", app.status)
    };
    assert_eq!(conflicts[0].qualified_name, "Library::Storage::Cache");
    app.resolve_conflict(request, Resolution::Rename);
    assert!(tree(&app).find("Library::Storage::Cache2").is_some());
    let ours = find(&app, "Library::Storage::Cache");
    assert!(
        agq_language::print_element(tree(&app), ours)
            .unwrap()
            .contains("Ours.")
    );
}

#[test]
fn my_library_brings_a_block_into_another_project_as_its_own_copy() {
    let (mut app, folder) = studio("library-mine");
    let package = find(&app, "P");
    app.create(
        crate::edit::CreateKind::Part,
        true,
        "Checkout",
        Parent::Element(package),
    );
    let checkout = find(&app, "P::Checkout");
    app.start_save(checkout);
    assert!(matches!(app.dialog, Some(Dialog::SaveToLibrary { .. })));
    app.dialog = None;
    app.save_to_library(checkout, "Payments", false).unwrap();
    let saved = app.library.source.mine_path().unwrap().to_path_buf();
    assert!(
        saved.starts_with(&folder.0),
        "My Library sits beside the session"
    );
    assert!(
        std::fs::read_to_string(&saved)
            .unwrap()
            .contains("part def Checkout")
    );
    // Another project.
    app.create_project(&folder.0.join("Q"), "Q");
    let block = BlockRef::new(Scope::Mine, "Library::Payments::Checkout");
    let package = find(&app, "Q");
    app.select(SceneTarget::Container(package), false);
    let usage = app
        .insert_block(block, Some(Parent::Element(package)), None)
        .expect("inserted");
    app.dialog = None;
    let copy = find(&app, "Library::Payments::Checkout");
    assert_eq!(tree(&app)[usage].typed_by[0].target(), Some(copy));
    // Removing it from My Library leaves the project's copy.
    app.remove_from_library(&BlockRef::new(Scope::Mine, "Library::Payments::Checkout"));
    assert!(tree(&app).find("Library::Payments::Checkout").is_some());
}

#[test]
fn inside_a_locked_part_the_lock_asks_and_a_stale_answer_is_refused() {
    let (mut app, _folder) = studio("library-lock");
    let system = system(&mut app);
    app.operation("Lock System", Operation::Lock { element: system });
    assert!(
        app.insert_block(built("Library::Storage::Cache"), None, None)
            .is_none()
    );
    assert!(
        matches!(app.dialog, Some(Dialog::Confirm { ref locked, .. }) if locked == &vec![system])
    );
    // Another change lands while the question is open: the block's change
    // names elements it creates itself, so it is not applied to the changed
    // model.
    let package = find(&app, "P");
    app.project
        .as_mut()
        .unwrap()
        .apply(Change::new(
            agq_system_state::Actor::Operator,
            "Create other",
            vec![Operation::Create {
                parent: Parent::Element(package),
                element: Box::new(agq_language::Element::named(ElementKind::Part, "other")),
            }],
        ))
        .unwrap();
    app.answer(true);
    assert!(tree(&app).find("Library").is_none(), "{}", app.status);
    assert!(app.status.contains("do it again"), "{}", app.status);
    // Asked again with the model as it is now, it applies.
    app.insert_block(built("Library::Storage::Cache"), None, None);
    app.answer(true);
    assert!(tree(&app).find("Library::Storage::Cache").is_some());
}

#[test]
fn a_link_to_a_block_shows_it_in_the_library() {
    let (mut app, _folder) = studio("library-link");
    app.show_block(built("Library::Messaging::Queue"));
    assert_eq!(app.left, LeftTab::Library);
    assert_eq!(
        app.library.selected,
        Some(built("Library::Messaging::Queue"))
    );
    app.show_block(built("Library::Nothing"));
    assert!(app.status.contains("has no"), "{}", app.status);
}
