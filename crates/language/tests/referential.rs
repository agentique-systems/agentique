//! Referential part and item usages (`ref part`, `ref item`; SysML 7.6.3,
//! C-55): read, written back, validated and kept by identity. A referential
//! usage refers to a part that exists elsewhere; a composite one contains it.
use agq_language::{ElementId, ElementKind, Source, Tree, parse, print, validate};
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

fn expect(text: &str, expected: &[(&str, &str)]) {
    let actual = codes(&load(text));
    let actual: Vec<(&str, &str)> = actual.iter().map(|(e, c)| (e.as_str(), *c)).collect();
    assert_eq!(actual, expected, "\n{text}");
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

/// A power bus shared by the flight computer: one composite `bus`, and
/// referential usages that refer to it.
const DRONE: &str = "package P {
    private import ScalarValues::*;

    item def Fuel;

    port def FuelPort {
        in ref item feed : Fuel;
    }

    part def PowerBus {
        attribute level : Natural = 0;
    }

    part def FlightComputer {
        ref part supply : PowerBus;
        ref item reserve : Fuel[0..1];
        abstract ref part spares : PowerBus[*] {
            doc /* Buses it may switch to. */
        }
    }

    part def Drone {
        part bus : PowerBus;
        part flightComputer : FlightComputer {
            ref part :>> supply = bus;
        }
        ref part mainBus : PowerBus = bus;
    }
}
";

#[test]
fn ref_part_and_ref_item_read_and_print_back() {
    let tree = load(DRONE);
    assert_eq!(codes(&tree), []);
    let supply = tree.find("P::FlightComputer::supply").unwrap();
    assert_eq!(tree[supply].kind, ElementKind::Part, "the kind stays part");
    assert!(tree[supply].referential);
    let reserve = tree.find("P::FlightComputer::reserve").unwrap();
    assert_eq!(tree[reserve].kind, ElementKind::Item);
    assert!(tree[reserve].referential);
    let bus = tree.find("P::Drone::bus").unwrap();
    assert!(!tree[bus].referential, "a part without `ref` is composite");
    let bound = tree.find("P::Drone::flightComputer::supply").unwrap();
    assert!(tree[bound].referential);
    assert_eq!(tree[bound].redefines[0].target(), Some(supply));
    let printed = &print(&tree)[0].text;
    for line in [
        "        in ref item feed : Fuel;\n",
        "        ref part supply : PowerBus;\n",
        "        ref item reserve : Fuel[0..1];\n",
        "        abstract ref part spares : PowerBus[*] {\n",
        "            ref part :>> supply = bus;\n",
        "        ref part mainBus : PowerBus = bus;\n",
        "        part bus : PowerBus;\n",
    ] {
        assert!(printed.contains(line), "{line:?} in\n{printed}");
    }
    assert_eq!(printed, DRONE, "the text is already canonical");
    round_trips(&tree);
}

#[test]
fn ref_is_only_for_part_and_item_usages() {
    let tree = load(
        "package P {
             part def A;
             port def Q;
             ref part def B;
             connection def C { end ref part a : A; end part b : A; }
             part def D { ref port q : Q; ref attribute n; }
         }",
    );
    let found = codes(&tree);
    let found: Vec<(&str, &str)> = found.iter().map(|(e, c)| (e.as_str(), *c)).collect();
    assert_eq!(
        found,
        [
            ("P::(syntax error)", "syntax"),
            ("P::C::(syntax error)", "syntax"),
            ("P::D::q", "unsupported"),
            ("P::D::n", "unsupported"),
        ]
    );
    // `ref x : T;` without a kind keyword is still a reference usage.
    let tree = load("package P { part def A; part def B { ref x : A; } }");
    let x = tree.find("P::B::x").unwrap();
    assert_eq!(tree[x].kind, ElementKind::Reference);
    assert!(!tree[x].referential);
    assert!(print(&tree)[0].text.contains("        x : A;\n"));
}

#[test]
fn a_shared_part_is_composite_once_and_referred_to_elsewhere() {
    expect(DRONE, &[]);
    // A usage without a kind keyword redefining a part is referential too.
    expect(
        "package P {
             part def PowerBus;
             part def FlightComputer { ref part supply : PowerBus; }
             part def Drone {
                 part bus : PowerBus;
                 part flightComputer : FlightComputer { :>> supply = bus; }
             }
         }",
        &[],
    );
    // Unbound: it refers to nothing yet, which is valid.
    expect(
        "package P { part def B; part def C { ref part b : B; ref item i : B[0..1]; } }",
        &[],
    );
}

#[test]
fn a_composite_part_bound_to_another_part_is_reported() {
    expect(
        "package P {
             part def PowerBus;
             item def Cargo;
             part def FlightComputer { ref part supply : PowerBus; part spare : PowerBus[0..1]; }
             part def Drone {
                 part bus : PowerBus;
                 item load : Cargo;
                 part backup : PowerBus = bus;
                 item copy : Cargo = load;
                 part flightComputer : FlightComputer {
                     part :>> supply = bus;
                     ref part :>> spare = bus;
                 }
             }
         }",
        &[
            ("P::Drone::backup", "wrong-value"),
            ("P::Drone::copy", "wrong-value"),
            ("P::Drone::flightComputer::supply", "wrong-value"),
        ],
    );
    let tree = load("package P { part def B; part def S { part b : B; part c : B = b; } }");
    let message = validate(&tree).remove(0).message;
    assert_eq!(
        message,
        "a composite part cannot be bound to another part: it would be owned twice; make it `ref part` to share it"
    );
}

#[test]
fn what_a_ref_part_refers_to_must_be_a_part_of_its_types() {
    expect(
        "package P {
             private import ScalarValues::*;
             part def PowerBus;
             part def HighVoltageBus :> PowerBus;
             part def Battery;
             item def Fuel;
             part def Tank :> Fuel;
             part def FlightComputer { ref part supply : PowerBus; }
             part def Drone {
                 part bus : HighVoltageBus;
                 part battery : Battery;
                 item fuel : Fuel;
                 part tank : Tank;
                 attribute volts : Natural = 5;
                 part fine : FlightComputer { ref part :>> supply = bus; }
                 part wrongType : FlightComputer { ref part :>> supply = battery; }
                 part shorthand : FlightComputer { :>> supply = battery; }
                 ref part toAttribute : PowerBus = volts;
                 ref part toItem : Fuel = fuel;
                 ref item itemToPart : Fuel = tank;
                 ref part literal : PowerBus = 5;
                 ref part made : PowerBus = new PowerBus();
                 ref part untyped = bus;
             }
         }",
        &[
            ("P::Drone::wrongType::supply", "wrong-value"),
            ("P::Drone::shorthand::supply", "wrong-value"),
            ("P::Drone::toAttribute", "wrong-value"),
            ("P::Drone::toItem", "wrong-type"),
            ("P::Drone::toItem", "wrong-value"),
            ("P::Drone::literal", "wrong-value"),
            ("P::Drone::made", "wrong-value"),
        ],
    );
    let tree = load(
        "package P {
             part def PowerBus;
             part def Battery;
             part def Drone { part battery : Battery; ref part supply : PowerBus = battery; }
         }",
    );
    let message = validate(&tree).remove(0).message;
    assert_eq!(
        message,
        "`battery` (Battery) cannot be what this part refers to: it is not a PowerBus"
    );
}

#[test]
fn a_port_connected_through_a_reference_faces_the_part_referred_to() {
    // `draw` passes items on to a part inside (same directions), but faces
    // the bus it refers to (mirrored directions).
    expect(
        "package P {
             item def Load;
             port def Power { in item load : Load; }
             part def Bus { port p : Power; }
             part def Inner { port p : Power; }
             part def Computer {
                 port draw : ~Power;
                 ref part supply : Bus;
                 part inner : Inner;
                 connection toSupply connect draw to supply.p;
                 connection toInner connect draw to inner.p;
             }
         }",
        &[("P::Computer::toInner", "incompatible-ends")],
    );
}

#[test]
fn a_part_def_may_refer_to_its_own_kind_but_not_contain_it() {
    // Agentique contains an Orchestrator, which refers to a test instance
    // of Agentique: no cycle, since a reference contains nothing.
    expect(
        "package P {
             part def Agentique { part orchestrator : Orchestrator; }
             part def Orchestrator { ref part testInstance : Agentique; }
         }",
        &[],
    );
    expect(
        "package P {
             part def Agentique { part orchestrator : Orchestrator; }
             part def Orchestrator { part testInstance : Agentique; }
         }",
        &[
            ("P::Agentique", "composition-cycle"),
            ("P::Orchestrator", "composition-cycle"),
        ],
    );
    // Redefined as composite, the reference becomes containment again.
    expect(
        "package P {
             part def Node { ref part next : Node; }
             part def Chain :> Node { part :>> next : Chain; }
         }",
        &[("P::Chain", "composition-cycle")],
    );
}

#[test]
fn ids_and_the_flag_survive_print_parse_and_rekey() {
    let mut tree = load(DRONE);
    // Rename the referred-to part: the value stays bound to it by identity.
    let bus = tree.find("P::Drone::bus").unwrap();
    tree.get_mut(bus).unwrap().name = Some("mainPower".into());
    let bound = tree.find("P::Drone::flightComputer::supply").unwrap();
    let printed = print(&tree);
    assert!(
        printed[0]
            .text
            .contains("            ref part :>> supply = mainPower;\n"),
        "{}",
        printed[0].text
    );
    // Read it back and give each element its stored id, as a project does.
    let mut again = parse(&printed);
    let stored: HashMap<ElementId, ElementId> = again.walk().into_iter().zip(tree.walk()).collect();
    again.rekey(&stored).unwrap();
    assert_eq!(print(&again), printed);
    for id in tree.walk() {
        let (before, after) = (&tree[id], &again[id]);
        assert_eq!(
            (after.kind, after.referential, after.owner()),
            (before.kind, before.referential, before.owner()),
            "{}",
            tree.qualified_name(id)
        );
    }
    assert!(again[bound].referential);
    let value = again[bound].expression.as_ref().unwrap().references()[0].target();
    assert_eq!(value, Some(bus));
    assert_eq!(codes(&again), []);
}
