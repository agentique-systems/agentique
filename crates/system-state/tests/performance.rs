//! Edits must feel live (ROADMAP §3.3): one edit on a model of about
//! 2,000 elements, applied, revalidated and described as an event.

use agq_language::{Element, ElementKind, Parent, Source, parse};
use agq_system_state::{Actor, Change, Operation, SystemState};
use std::collections::BTreeSet;
use std::fmt::Write;
use std::time::{Duration, Instant};

/// About 2,000 elements: 170 components, each a part def with ports, an
/// attribute and a doc comment, used and connected in one system.
fn large_model() -> String {
    let mut text = String::from(
        "package Large {\n    item def Message;\n    port def Link { in item payload : Message; }\n",
    );
    for i in 0..170 {
        let _ = writeln!(
            text,
            "    part def Component{i} {{\n        doc /* Component {i}. */\n        port input : Link;\n        port output : ~Link;\n        attribute size : ScalarValues::Integer = {i};\n        part inner{i} : Inner{i};\n    }}\n    part def Inner{i} {{ port a : Link; port b : ~Link; attribute weight : ScalarValues::Real = 1.5; }}"
        );
    }
    text.push_str("    part def System {\n");
    for i in 0..170 {
        let _ = writeln!(text, "        part c{i} : Component{i};");
    }
    for i in 0..169 {
        let _ = writeln!(
            text,
            "        connection link{i} connect c{i}.output to c{}.input;",
            i + 1
        );
    }
    text.push_str("    }\n}\n");
    text
}

fn timed(state: &mut SystemState, change: Change) -> Duration {
    let start = Instant::now();
    state.apply(change).expect("the edit applies");
    start.elapsed()
}

#[test]
fn a_single_edit_on_two_thousand_elements_feels_live() {
    let tree = parse(&[Source::new("large.sysml", large_model())]);
    let mut state = SystemState::new(tree, BTreeSet::new());
    assert!(
        state.tree().len() >= 2_000,
        "{} elements",
        state.tree().len()
    );
    assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());

    let component = state.tree().find("Large::Component50").unwrap();
    let system = state.tree().find("Large::System").unwrap();
    let mut rename = Duration::MAX;
    let mut create = Duration::MAX;
    // The best of a few runs: the machine is shared.
    for round in 0..5 {
        let name = format!("Renamed{round}");
        rename = rename.min(timed(
            &mut state,
            Change::new(
                Actor::Operator,
                "Rename",
                vec![Operation::Rename {
                    element: component,
                    name,
                }],
            ),
        ));
        create = create.min(timed(
            &mut state,
            Change::new(
                Actor::Operator,
                "Create",
                vec![Operation::Create {
                    parent: Parent::Element(system),
                    element: Box::new(Element::named(ElementKind::Part, &format!("extra{round}"))),
                }],
            ),
        ));
    }
    let undo = {
        let start = Instant::now();
        state.undo().unwrap();
        start.elapsed()
    };
    println!(
        "{} elements: rename {rename:?}, create {create:?}, undo {undo:?} (apply + validate + event)",
        state.tree().len()
    );
    assert!(state.diagnostics().is_empty(), "{:?}", state.diagnostics());
    // The target is 50 ms in a debug build (measured about 47 ms, and 8 ms
    // in release, on the development machine); the bound leaves room for a
    // busy machine and catches work that grows faster than the model.
    let limit = Duration::from_millis(if cfg!(debug_assertions) { 100 } else { 20 });
    assert!(rename < limit && create < limit, "{rename:?} {create:?}");
}
