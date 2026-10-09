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
fn the_usage_prefix_is_written_in_the_standard_order() {
    // Direction, then `abstract`, then `ref` right before the kind keyword
    // (SysML 8.2.2.6.2); other orders are read and written that way.
    let tree = load(
        "package P {
             item def Fuel;
             port def Feed { abstract in ref item a : Fuel; in abstract item b : Fuel; abstract out item c : Fuel; }
         }",
    );
    assert_eq!(codes(&tree), []);
    let printed = &print(&tree)[0].text;
    for line in [
        "        in abstract ref item a : Fuel;\n",
        "        in abstract item b : Fuel;\n",
        "        out abstract item c : Fuel;\n",
    ] {
        assert!(printed.contains(line), "{line:?} in\n{printed}");
    }
    round_trips(&tree);
    // `ref` after another prefix keyword is a syntax error, not a construct
    // outside the subset.
    let tree = load(
        "package P {
             part def A;
             part def D { ref abstract part x : A; ref in item y : A; ref end part z : A; }
         }",
    );
    let messages: Vec<(&str, String)> = validate(&tree)
        .into_iter()
        .map(|d| (d.code, d.message))
        .collect();
    assert_eq!(
        messages,
        [
            (
                "syntax",
                "`ref` comes after `abstract`, right before `part` or `item`".to_string()
            ),
            (
                "syntax",
                "`ref` comes after `in`, right before `part` or `item`".to_string()
            ),
            (
                "syntax",
                "an `end` feature is always referential; write it without `ref`".to_string()
            ),
        ]
    );
}

#[test]
fn a_shared_part_is_composite_once_and_referred_to_elsewhere() {
    expect(DRONE, &[]);
    // A usage without a kind keyword redefining a reference binds it too.
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
    // Not bound: the part it refers to is not identified, which is valid.
    expect(
        "package P { part def B; part def C { ref part b : B; ref item i : B[0..1]; } }",
        &[],
    );
    // Directed and end usages, and usages owned by a package, are always
    // referential (SysML `validateUsageIsReferential`): bound to a usage,
    // they refer to it.
    expect(
        "package P {
             item def Fuel;
             part def B;
             part def Tank { item stock : Fuel; in item intake : Fuel = stock; }
             part a : B;
             part c : B = a;
         }",
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
                 part other : FlightComputer { :>> spare = bus; }
             }
         }",
        &[
            ("P::Drone::backup", "wrong-value"),
            ("P::Drone::copy", "wrong-value"),
            ("P::Drone::flightComputer::supply", "wrong-value"),
            ("P::Drone::flightComputer::spare", "wrong-value"),
            ("P::Drone::other::spare", "wrong-value"),
        ],
    );
    let message = |text: &str| validate(&load(text)).remove(0).message;
    // Of the same owner: stricter than the standard (deviation 19).
    assert_eq!(
        message("package P { part def B; part def S { part b : B; part c : B = b; } }"),
        "`b` is a part of the same owner: bound to it, this composite part would be that part under a second name; declare it `ref part` (deviation 19)"
    );
    // Of another owner: the standard forbids it (7.6.3).
    assert_eq!(
        message(
            "package P { part def B; part def Power { part bus : B; } part def S { part power : Power; part c : B = power.bus; } }"
        ),
        "a composite part's value cannot be a part of another owner (SysML 7.6.3); declare it `ref part` to refer to `power.bus`"
    );
    // Written `ref`, but it redefines a composite part: composite too.
    assert_eq!(
        message(
            "package P {
                 part def B;
                 part def FlightComputer { part spare : B[0..1]; }
                 part def Drone { part bus : B; part fc : FlightComputer { ref part :>> spare = bus; } }
             }"
        ),
        "`spare` is composite in `FlightComputer`; declare it `ref part` there"
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
                 part shortcut : FlightComputer { :>> supply = bus; }
                 part wrongType : FlightComputer { ref part :>> supply = battery; }
                 part shorthand : FlightComputer { :>> supply = battery; }
                 ref part viaShortcut : PowerBus = shortcut.supply;
                 ref part toAttribute : PowerBus = volts;
                 ref part toItem : Fuel = fuel;
                 ref item itemToPart : Fuel = tank;
                 ref item madeFuel : Fuel = new Fuel();
                 ref item madeWrong : Fuel = new Battery();
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
            ("P::Drone::madeWrong", "wrong-value"),
            ("P::Drone::literal", "wrong-value"),
            ("P::Drone::made", "unsupported"),
        ],
    );
    let message = |text: &str| validate(&load(text)).remove(0).message;
    assert_eq!(
        message(
            "package P {
                 part def PowerBus;
                 part def Battery;
                 part def Drone { part battery : Battery; ref part supply : PowerBus = battery; }
             }"
        ),
        "`battery` (Battery) cannot be what this part refers to: it is not a PowerBus"
    );
    assert_eq!(
        message("package P { part def B; part def D { ref part b : B = 5; } }"),
        "`5` is a data value; a referential part refers to a part usage, such as `= bus` or `= power.bus`"
    );
}

#[test]
fn a_reference_bound_to_itself_refers_to_no_part() {
    expect(
        "package P {
             part def B;
             part def S {
                 part real : B;
                 ref part a : B = a;
                 ref part x : B = y;
                 ref part y : B = x;
                 ref part z : B = w;
                 ref part w : B = real;
             }
         }",
        &[
            ("P::S::a", "wrong-value"),
            ("P::S::x", "wrong-value"),
            ("P::S::y", "wrong-value"),
        ],
    );
    let messages: Vec<String> = validate(&load(
        "package P { part def B; part def S { ref part a : B = a; ref part x : B = y; ref part y : B = x; } }",
    ))
    .into_iter()
    .map(|d| d.message)
    .collect();
    assert_eq!(
        messages,
        [
            "`a` is bound to itself, so it refers to no part",
            "`x` and `y` are bound to each other, so neither refers to a part",
            "`y` and `x` are bound to each other, so neither refers to a part",
        ]
    );
}

#[test]
fn a_binding_is_kept_and_never_changed_in_a_redefinition() {
    expect(
        "package P {
             part def B;
             part def S { part bus : B; part other : B; ref part main : B = bus; }
             part def Keeps :> S { ref part :>> main[1]; }
             part def Rebinds :> S { ref part :>> main = other; }
             part def Contains :> S { part :>> main; }
         }",
        &[
            ("P::Rebinds::main", "wrong-value"),
            ("P::Contains::main", "wrong-value"),
        ],
    );
    let messages: Vec<String> = validate(&load(
        "package P {
             part def B;
             part def S { part bus : B; part other : B; ref part main : B = bus; }
             part def Rebinds :> S { ref part :>> main = other; }
             part def Contains :> S { part :>> main; }
         }",
    ))
    .into_iter()
    .map(|d| d.message)
    .collect();
    assert_eq!(
        messages,
        [
            "`main` is already bound where it is declared (in `P::S`); a redefinition cannot bind it again (KerML `validateFeatureValueOverriding`)",
            "it redefines `main`, which is bound to `bus`: as a composite part it would hold that part as its own; declare it `ref part`",
        ]
    );
}

#[test]
fn a_port_connected_through_a_reference_faces_or_passes_inward_by_its_binding() {
    // Unbound, or bound outside: the part referred to is not inside, so
    // `draw` faces it (mirrored directions), while it passes items on to a
    // part inside (same directions).
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
    // Bound to a part inside the connection's owner: inside, so the
    // owner's port passes items on to it (same directions).
    expect(
        "package P {
             item def Load;
             port def Power { in item load : Load; }
             part def Bus { port p : Power; }
             part def Computer { ref part supply : Bus; }
             part def Drone {
                 port feed : Power;
                 port socket : ~Power;
                 part bus : Bus;
                 part computer : Computer { ref part :>> supply = bus; }
                 connection inward connect feed to computer.supply.p;
                 connection mirrored connect socket to computer.supply.p;
             }
         }",
        &[("P::Drone::mirrored", "incompatible-ends")],
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
    // A `ref` (or keyword-less) redefinition of a composite part is
    // composite too: it has the values of what it redefines.
    expect(
        "package P {
             part def Node { part next : Node[0..1]; }
             part def Chain :> Node { ref part :>> next : Chain[1]; }
             part def Ring :> Node { :>> next : Ring[1]; }
         }",
        &[
            ("P::Chain", "composition-cycle"),
            ("P::Ring", "composition-cycle"),
        ],
    );
    let tree = load(
        "package P {
             part def Node { part next : Node[0..1]; }
             part def Chain :> Node { ref part :>> next : Chain[1]; }
         }",
    );
    assert_eq!(
        validate(&tree).remove(0).message,
        "it contains itself through the required part `P::Chain::next` (it redefines the composite `P::Node::next`, so it is composite too); give that part a lower bound of 0"
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

#[test]
fn a_composite_binding_is_judged_by_the_part_it_reaches() {
    let message = |text: &str| validate(&load(text)).remove(0).message;
    // Through a reference of the same owner, it reaches a composite part
    // of another owner: the standard's rule (7.6.3).
    assert_eq!(
        message(
            "package P {
                 part def B;
                 part def Power { part bus : B; }
                 part def S { part power : Power; ref part r : B = power.bus; part c : B = r; }
             }"
        ),
        "a composite part's value cannot be a part of another owner (SysML 7.6.3); declare it `ref part` to refer to `r`"
    );
    // A part owned by a package, or a reference that is not bound, is no
    // composite part of another owner: deviation 19.
    assert_eq!(
        message("package P { part def B; part a : B; part def S { part c : B = a; } }"),
        "bound to `a`, this composite part would be that part under a second name; declare it `ref part` (deviation 19)"
    );
    assert_eq!(
        message("package P { part def B; part def S { ref part r : B; part c : B = r; } }"),
        "bound to `r`, this composite part would be that part under a second name; declare it `ref part` (deviation 19)"
    );
}

#[test]
fn a_reference_to_a_usage_of_unwritten_kind_is_unsupported() {
    // `x : B;` has no kind keyword and redefines nothing: the subset does
    // not infer that it is a part.
    let text = "package P { part def B; part def S { x : B; ref part r : B = x; } }";
    expect(text, &[("P::S::r", "unsupported")]);
    assert_eq!(
        validate(&load(text)).remove(0).message,
        "binding to `x`, a usage without a kind keyword that redefines no part or item, is not supported; declare it `part` or `ref part`"
    );
}
