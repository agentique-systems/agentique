//! Referential parts in model execution (C-55): a `ref part` bound to a part
//! is that part's instance, reached by either path; one bound to nothing
//! has no behaviour of its own, and a message reaching it stops the run
//! unless a stand-in answers for it.
use agq_language::{Source, Tree, parse, validate};
use agq_simulation::digest::model_digest;
use agq_simulation::{
    Answers, EventKind, Mode, Request, RunResult, RunStatus, StopReason, Verdict, compile, run,
};
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
        "`glider.flightComputer.supply` refers to nothing in this configuration, so Load(watts = 20) sent to it through `aux` reaches nothing; bind it (`= ...`) or stand it in"
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
