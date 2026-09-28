//! The Surface's direct manipulation, without a window.
use super::*;
use crate::edit::app_tests::studio;

#[test]
fn dragging_a_card_changes_only_the_layout() {
    let (mut app, _folder) = studio("surface-move");
    let package = app
        .project
        .as_ref()
        .unwrap()
        .state()
        .tree()
        .find("P")
        .unwrap();
    app.create(
        crate::edit::CreateKind::Part,
        false,
        "api",
        Parent::Element(package),
    );
    let api = app
        .project
        .as_ref()
        .unwrap()
        .state()
        .tree()
        .find("P::api")
        .unwrap();
    let revision = app.project.as_ref().unwrap().state().revision();
    drop_card(
        &mut app,
        api,
        Point::new(0.0, 0.0),
        Point::new(120.0, 60.0),
        false,
        None,
    );
    // The layout remembers where it was dropped; the architecture layout
    // then compacts a container's cards, so a lone card settles back.
    let remembered = app.layouts[&app.view].bounds.get(&api).copied();
    assert!(remembered.is_some(), "the drop is remembered");
    assert!(app.lookup.node(&app.scene, api).is_some());
    assert_eq!(
        app.project.as_ref().unwrap().state().revision(),
        revision,
        "moving a card is presentation only"
    );
}

#[test]
fn alt_drop_moves_the_element_into_the_container() {
    let (mut app, _folder) = studio("surface-alt-drop");
    let package = app
        .project
        .as_ref()
        .unwrap()
        .state()
        .tree()
        .find("P")
        .unwrap();
    app.create(
        crate::edit::CreateKind::Part,
        false,
        "api",
        Parent::Element(package),
    );
    app.create(
        crate::edit::CreateKind::Part,
        false,
        "service",
        Parent::Element(package),
    );
    let tree = app.project.as_ref().unwrap().state().tree();
    let (api, service) = (
        tree.find("P::api").unwrap(),
        tree.find("P::service").unwrap(),
    );
    drop_card(
        &mut app,
        api,
        Point::default(),
        Point::new(10.0, 10.0),
        true,
        Some(&SceneTarget::Node(service)),
    );
    let tree = app.project.as_ref().unwrap().state().tree();
    assert!(tree.find("P::service::api").is_some(), "{}", app.status);
}

#[test]
fn every_context_command_exists_and_edges_offer_only_delete() {
    let edge = SceneTarget::Edge("e".into());
    assert_eq!(context_commands(Some(&edge)), &[CommandId::Delete]);
    for target in [None, Some(&edge)] {
        for id in context_commands(target) {
            let _ = commands::command(*id);
        }
    }
}

#[test]
fn a_selected_card_has_a_screen_reader_name() {
    let (mut app, _folder) = studio("surface-names");
    let package = app
        .project
        .as_ref()
        .unwrap()
        .state()
        .tree()
        .find("P")
        .unwrap();
    app.create(
        crate::edit::CreateKind::Part,
        false,
        "api",
        Parent::Element(package),
    );
    let api = app
        .project
        .as_ref()
        .unwrap()
        .state()
        .tree()
        .find("P::api")
        .unwrap();
    let target = SceneTarget::Node(api);
    assert_eq!(element_name(&app, &target, true), "part api, selected");
    assert_eq!(element_name(&app, &target, false), "part api");
}
