//! Referential parts in model execution (C-55): a `ref part` bound to a part
//! is that part's instance, reached by either path; one bound to nothing
//! has no behaviour of its own, and a message reaching it stops the run
//! unless a stand-in answers for it.
use agq_language::{Source, Tree, parse, validate};
use agq_simulation::digest::model_digest;
use agq_simulation::{
    Answers, EventKind, Mode, Request, RunResult, RunStatus, StopReason, Verdict, compile, run,
};

/// What keeps the system of `scenario` from running, in words.
fn blockers(tree: &Tree, scenario: &str) -> Vec<String> {
    let id = tree.find(scenario).unwrap();
    let program = compile(tree, id).unwrap_or_else(|b| panic!("blocked: {b:?}"));
    match program.system {
        Ok(_) => Vec::new(),
        Err(blockers) => blockers.into_iter().map(|b| b.message).collect(),
    }
}
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// One power bus, contained by the drone. The motors draw from it directly;
/// the flight computer refers to it as its `supply` and draws through that.
const DRONE: &str = "package Drones {
    private import ScalarValues::*;

    item def Load {
        attribute watts : Natural;
    }

    item def Ack {
        attribute total : Natural;
    }

    item def Command {
        attribute watts : Natural;
    }

    item def Done {
        attribute total : Natural;
    }

    port def PowerPort {
        in item load : Load;
        out item ack : Ack;
    }

    port def ControlPort {
        in item command : Command;
        out item done : Done;
    }

    part def PowerBus {
        port main : PowerPort;
        port aux : PowerPort;
        attribute total : Natural = 0;
        attribute requests : Natural = 0;
        exhibit state supplying {
            entry;
            then on;
            state on;
            transition first on accept l : Load via main do action {
                assign total := total + l.watts;
                then assign requests := requests + 1;
                then send new Ack(total = total) via main;
            } then on;
            transition first on accept l : Load via aux do action {
                assign total := total + l.watts;
                then assign requests := requests + 1;
                then send new Ack(total = total) via aux;
            } then on;
        }
    }

    part def FlightComputer {
        port control : ControlPort;
        port draw : ~PowerPort;
        ref part supply : PowerBus;
        connect draw to supply.aux;
        exhibit state flying {
            entry;
            then ready;
            state ready;
            state waiting;
            transition first ready accept c : Command via control do send new Load(watts = c.watts) via draw then waiting;
            transition first waiting accept a : Ack via draw do send new Done(total = a.total) via control then ready;
        }
    }

    part def Drone {
        port motors : PowerPort;
        port pilot : ControlPort;
        part bus : PowerBus;
        part flightComputer : FlightComputer {
            ref part :>> supply = bus;
        }
        connect motors to bus.main;
        connect pilot to flightComputer.control;
    }

    part def Glider {
        port pilot : ControlPort;
        part flightComputer : FlightComputer;
        connect pilot to flightComputer.control;
    }

    part def Power {
        part bus : PowerBus;
    }

    part def Quadcopter {
        port motors : PowerPort;
        port pilot : ControlPort;
        part power : Power;
        ref part mainBus : PowerBus = power.bus;
        part flightComputer : FlightComputer {
            :>> supply = mainBus;
        }
        connect motors to mainBus.main;
        connect pilot to flightComputer.control;
    }

    part def Hexacopter :> Quadcopter {
        ref part :>> mainBus[1];
    }

    part def Meter {
        ref part supply : PowerBus;
        exhibit state metering {
            entry assign supply.total := 1;
            then on;
            state on;
        }
    }

    part def Gauge {
        part meter : Meter;
    }

    verification def OneSharedBus {
        subject drone : Drone;
        send new Load(watts = 100) via drone.motors;
        then accept a : Ack via drone.motors;
        then send new Command(watts = 20) via drone.pilot;
        then accept d : Done via drone.pilot;
        then assert constraint sharedTotal {
            a.total == 100 and d.total == 120
        }
        then assert constraint oneMachine {
            drone.bus.requests == 2 and drone.flightComputer.supply.requests == 2 and drone.flightComputer.supply.total == 120
        }
    }

    verification def QuadSharesItsBus {
        subject quad : Quadcopter;
        send new Load(watts = 100) via quad.motors;
        then accept a : Ack via quad.motors;
        then send new Command(watts = 20) via quad.pilot;
        then accept d : Done via quad.pilot;
        then assert constraint sharedTotal {
            a.total == 100 and d.total == 120
        }
        then assert constraint oneMachine {
            quad.power.bus.requests == 2 and quad.mainBus.requests == 2 and quad.flightComputer.supply.total == 120
        }
    }

    verification def HexKeepsTheBinding {
        subject hex : Hexacopter;
        send new Load(watts = 100) via hex.motors;
        then accept a : Ack via hex.motors;
        then send new Command(watts = 20) via hex.pilot;
        then accept d : Done via hex.pilot;
        then assert constraint sharedTotal {
            a.total == 100 and d.total == 120 and hex.power.bus.requests == 2
        }
    }

    verification def ReadsWhatIsNotBound {
        subject glider : Glider;
        assert constraint supplyIsEmpty {
            glider.flightComputer.supply.total == 0
        }
    }

    verification def MetersWhatIsNotBound {
        subject gauge : Gauge;
    }

    verification def GlidesWithoutPower {
        subject glider : Glider;
        assert constraint nothingAsked {
            true
        }
    }

    verification def AsksForPowerItLacks {
        subject glider : Glider;
        send new Command(watts = 20) via glider.pilot;
        then accept d : Done via glider.pilot;
    }

    verification def BatteryStandsIn {
        subject glider : Glider;
        part battery : Scenarios::StandIn {
            :>> target = glider.flightComputer.supply;
            :>> outcome = Scenarios::Outcome::answer;
            :>> latencyMs = 5;
            :>> output = new Ack(total = 7);
        }
        send new Command(watts = 20) via glider.pilot;
        then accept d : Done via glider.pilot;
        then assert constraint answeredByTheStandIn {
            d.total == 7
        }
    }
}
";

fn load(text: &str) -> Tree {
    parse(&[Source::new("Drones.sysml", text)])
}

fn run_model(tree: &Tree, scenario: &str) -> RunResult {
    let id = tree
        .find(scenario)
        .unwrap_or_else(|| panic!("no {scenario}"));
    let program = compile(tree, id).unwrap_or_else(|b| panic!("blocked: {b:?}"));
    run(
        &program,
        model_digest(tree, id),
        &Request::new(Mode::Model),
        Answers::StandIns,
        Arc::new(AtomicBool::new(false)),
    )
}

fn verdicts(result: &RunResult) -> Vec<(&str, Verdict)> {
    result
        .checks
        .iter()
        .map(|c| (c.name.as_str(), c.verdict))
        .collect()
}

fn texts(result: &RunResult, kind: EventKind) -> Vec<String> {
    result
        .trace
        .iter()
        .filter(|e| e.kind == kind)
        .map(|e| e.text.clone())
        .collect()
}

#[test]
fn the_model_is_valid() {
    let tree = load(DRONE);
    assert_eq!(validate(&tree), []);
}

#[test]
fn both_paths_reach_the_one_bus_it_refers_to() {
    let tree = load(DRONE);
    let result = run_model(&tree, "Drones::OneSharedBus");
    assert_eq!(result.status, RunStatus::Completed, "{:#?}", result.stop);
    assert_eq!(
        verdicts(&result),
        [
            ("sharedTotal", Verdict::Passed),
            ("oneMachine", Verdict::Passed),
            ("no unexpected output", Verdict::Passed)
        ]
    );
    // The trace names the real instance: the bus, never a copy of it.
    let received = texts(&result, EventKind::Received);
    assert!(
        received
            .iter()
            .any(|t| t.starts_with("`drone.bus` received") && t.contains("through `aux`")),
        "{received:?}"
    );
    assert!(
        received
            .iter()
            .all(|t| !t.contains("drone.flightComputer.supply")),
        "{received:?}"
    );
    let assigned = texts(&result, EventKind::Assigned);
    assert_eq!(
        assigned
            .iter()
            .filter(|t| t.contains("requests := "))
            .count(),
        2,
        "one state machine counts both messages: {assigned:?}"
    );
}

#[test]
fn an_unbound_reference_does_not_keep_the_system_from_running() {
    let tree = load(DRONE);
    let result = run_model(&tree, "Drones::GlidesWithoutPower");
    assert_eq!(result.status, RunStatus::Completed, "{:#?}", result.stop);
    assert!(result.all_passed(), "{:?}", verdicts(&result));
}

#[test]
fn a_message_into_an_unbound_reference_stops_the_run_with_the_reason() {
    let tree = load(DRONE);
    let result = run_model(&tree, "Drones::AsksForPowerItLacks");
    assert_eq!(result.status, RunStatus::Stopped);
    let stop = result.stop.clone().unwrap();
    assert_eq!(stop.reason, StopReason::MissingStandIn, "{stop:?}");
    assert_eq!(
        stop.message,
        "`glider.flightComputer.supply` is not bound: the part it refers to is not identified in this model, so Load(watts = 20) sent to it through `aux` reaches no part; bind it (`= ...`) or stand it in"
    );
    let supply = tree.find("Drones::FlightComputer::supply").unwrap();
    assert_eq!(stop.element, Some(supply.raw()));
}

#[test]
fn a_stand_in_answers_for_an_unbound_reference() {
    let tree = load(DRONE);
    let result = run_model(&tree, "Drones::BatteryStandsIn");
    assert_eq!(result.status, RunStatus::Completed, "{:#?}", result.stop);
    assert_eq!(
        verdicts(&result),
        [
            ("answeredByTheStandIn", Verdict::Passed),
            ("no unexpected output", Verdict::Passed)
        ]
    );
    let stand_ins = texts(&result, EventKind::StandIn);
    assert_eq!(
        stand_ins,
        ["Stand-in `battery` answers for `glider.flightComputer.supply` (call 1)"]
    );
    assert_eq!(result.logical_ms, 5);
}

#[test]
fn a_stand_in_for_a_bound_reference_answers_for_the_part_it_refers_to() {
    let text = DRONE.replace(
        "    verification def GlidesWithoutPower {",
        "    verification def BusStandsIn {
        subject drone : Drone;
        part bus : Scenarios::StandIn {
            :>> target = drone.flightComputer.supply;
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new Ack(total = 3);
        }
        send new Load(watts = 1) via drone.motors;
        then accept a : Ack via drone.motors;
        then assert constraint sameBus {
            a.total == 3
        }
    }

    verification def GlidesWithoutPower {",
    );
    let tree = load(&text);
    let result = run_model(&tree, "Drones::BusStandsIn");
    assert_eq!(result.status, RunStatus::Completed, "{:#?}", result.stop);
    assert!(result.all_passed(), "{:?}", verdicts(&result));
    assert_eq!(
        texts(&result, EventKind::StandIn),
        ["Stand-in `bus` answers for `drone.bus` (call 1)"]
    );
}

#[test]
fn a_keyword_less_binding_through_another_reference_reaches_the_same_bus() {
    // `mainBus = power.bus`, and the flight computer's `:>> supply = mainBus;`:
    // both lead to the one bus inside the power unit, and the motors' port
    // passes items inward to it through `mainBus`.
    let tree = load(DRONE);
    let result = run_model(&tree, "Drones::QuadSharesItsBus");
    assert_eq!(result.status, RunStatus::Completed, "{:#?}", result.stop);
    assert!(result.all_passed(), "{:?}", verdicts(&result));
    let received = texts(&result, EventKind::Received);
    assert!(
        received
            .iter()
            .any(|t| t.starts_with("`quad.power.bus` received") && t.contains("through `main`")),
        "{received:?}"
    );
    assert!(
        received
            .iter()
            .any(|t| t.starts_with("`quad.power.bus` received") && t.contains("through `aux`")),
        "{received:?}"
    );
}

#[test]
fn a_redefinition_without_a_value_keeps_the_binding() {
    let tree = load(DRONE);
    let result = run_model(&tree, "Drones::HexKeepsTheBinding");
    assert_eq!(result.status, RunStatus::Completed, "{:#?}", result.stop);
    assert!(result.all_passed(), "{:?}", verdicts(&result));
}

#[test]
fn a_bound_reference_with_features_of_its_own_does_not_run() {
    let text = DRONE.replace(
        "    part def Hexacopter :> Quadcopter {
        ref part :>> mainBus[1];
    }",
        "    part def Hexacopter :> Quadcopter {
        ref part :>> mainBus {
            attribute spare : Natural;
        }
    }",
    );
    let tree = load(&text);
    assert_eq!(
        blockers(&tree, "Drones::HexKeepsTheBinding"),
        [
            "`hex.mainBus` refers to `power.bus` and declares `spare` of its own; what it refers to has that part's features, so declare them there"
        ]
    );
}

#[test]
fn a_ref_item_is_a_value_and_cannot_refer_to_a_part() {
    let text = DRONE.replace(
        "    part def Gauge {
        part meter : Meter;
    }",
        "    part def Gauge {
        part meter : Meter;
    }

    part def Tank :> Load;

    part def Carrier {
        part tank : Tank;
        ref item cargo : Load = tank;
    }

    verification def Carries {
        subject carrier : Carrier;
    }",
    );
    let tree = load(&text);
    assert_eq!(validate(&tree), []);
    assert_eq!(
        blockers(&tree, "Drones::Carries"),
        [
            "`carrier.cargo` is a `ref item`, a value in runs; it cannot refer to the part `tank`, which runs as an instance"
        ]
    );
}

#[test]
fn a_binding_that_leads_to_no_running_part_does_not_run() {
    // `spare` has no instance (multiplicity 0): a reference bound to it
    // never reaches one, and the run says so instead of guessing.
    let text = DRONE.replace(
        "    part def Gauge {
        part meter : Meter;
    }",
        "    part def Gauge {
        part meter : Meter;
        part spare : PowerBus[0];
        ref part backup : PowerBus = spare;
    }",
    );
    let tree = load(&text);
    assert_eq!(validate(&tree), []);
    assert_eq!(
        blockers(&tree, "Drones::MetersWhatIsNotBound"),
        [
            "`gauge.backup` is bound to `spare`, which does not lead to a part that runs in this configuration"
        ]
    );
}

#[test]
fn reading_or_setting_what_an_unbound_reference_refers_to_says_why_not() {
    let tree = load(DRONE);
    // A check reading through it cannot be evaluated.
    let result = run_model(&tree, "Drones::ReadsWhatIsNotBound");
    assert_eq!(result.status, RunStatus::Completed, "{:#?}", result.stop);
    assert_eq!(result.checks[0].verdict, Verdict::Inconclusive);
    assert!(
        result.checks[0].message.contains(
            "`glider.flightComputer.supply` is not bound: the part it refers to is not identified in this model; bind it (`= ...`) or stand it in"
        ),
        "{}",
        result.checks[0].message
    );
    // A part's behaviour setting an attribute through it stops the run.
    let result = run_model(&tree, "Drones::MetersWhatIsNotBound");
    assert_eq!(result.status, RunStatus::Stopped);
    let stop = result.stop.unwrap();
    assert_eq!(stop.reason, StopReason::EvaluationError);
    assert_eq!(
        stop.message,
        "`supply.total`: `gauge.meter.supply` is not bound: the part it refers to is not identified in this model; bind it (`= ...`) or stand it in"
    );
}

#[test]
fn bindings_to_themselves_keep_the_scenario_from_starting() {
    let text = DRONE.replace(
        "    part def Gauge {
        part meter : Meter;
    }",
        "    part def Gauge {
        part meter : Meter;
        ref part a : PowerBus = b;
        ref part b : PowerBus = a;
    }",
    );
    let tree = load(&text);
    let id = tree.find("Drones::MetersWhatIsNotBound").unwrap();
    let blockers = compile(&tree, id).expect_err("does not start");
    let messages: Vec<&str> = blockers.iter().map(|b| b.message.as_str()).collect();
    assert_eq!(
        messages,
        [
            "`a` and `b` are bound to each other, so neither refers to a part (wrong-value)",
            "`b` and `a` are bound to each other, so neither refers to a part (wrong-value)",
        ]
    );
}

#[test]
fn a_reference_bound_to_one_not_bound_is_the_same_instance() {
    // The monitor's supply is bound to the flight computer's, which is not
    // bound: both are one instance, so one stand-in answers both, and it
    // counts the two calls as its first and second.
    let text = DRONE.replace(
        "    part def Gauge {
        part meter : Meter;
    }",
        "    part def Gauge {
        part meter : Meter;
    }

    part def Monitor {
        port control : ControlPort;
        port draw : ~PowerPort;
        ref part supply : PowerBus;
        connect draw to supply.main;
        exhibit state watching {
            entry;
            then ready;
            state ready;
            state waiting;
            transition first ready accept c : Command via control do send new Load(watts = c.watts) via draw then waiting;
            transition first waiting accept a : Ack via draw do send new Done(total = a.total) via control then ready;
        }
    }

    part def Twin {
        port pilot : ControlPort;
        port watch : ControlPort;
        part flightComputer : FlightComputer;
        part monitor : Monitor {
            :>> supply = flightComputer.supply;
        }
        connect pilot to flightComputer.control;
        connect watch to monitor.control;
    }

    verification def OneStandInAnswersBoth {
        subject twin : Twin;
        part battery : Scenarios::StandIn {
            :>> target = twin.flightComputer.supply;
            :>> outcome = Scenarios::Outcome::answer;
            :>> output = new Ack(total = 7);
        }
        send new Command(watts = 20) via twin.pilot;
        then accept d : Done via twin.pilot;
        then send new Command(watts = 5) via twin.watch;
        then accept e : Done via twin.watch;
        then assert constraint bothAnswered {
            d.total == 7 and e.total == 7
        }
    }",
    );
    let tree = load(&text);
    assert_eq!(validate(&tree), []);
    let result = run_model(&tree, "Drones::OneStandInAnswersBoth");
    assert_eq!(result.status, RunStatus::Completed, "{:#?}", result.stop);
    assert!(result.all_passed(), "{:?}", verdicts(&result));
    assert_eq!(
        texts(&result, EventKind::StandIn),
        [
            "Stand-in `battery` answers for `twin.flightComputer.supply` (call 1)",
            "Stand-in `battery` answers for `twin.flightComputer.supply` (call 2)",
        ]
    );
}
