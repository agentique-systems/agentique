//! Behaviour, expressions, enumerations, dependencies, scenarios and agents
//! (C-50): read, printed back, linked by identity and validated.
use agq_language::{ElementKind, Expression, Role, Source, Tree, link, parse, print, validate};

fn load(text: &str) -> Tree {
    parse(&[Source::new("test.sysml", text)])
}

fn codes(tree: &Tree) -> Vec<(String, &'static str)> {
    validate(tree)
        .into_iter()
        .map(|d| (tree.qualified_name(d.element), d.code))
        .collect()
}

fn round_trips(tree: &Tree) {
    let printed = print(tree);
    let again = parse(&printed);
    assert_eq!(
        tree.clone().without_locations(),
        again.clone().without_locations(),
        "{}",
        printed[0].text
    );
    assert_eq!(print(&again), printed);
}

pub const DISPATCH: &str = include_str!("fixtures/Dispatch.sysml");

#[test]
fn a_retrying_dispatcher_reads_prints_and_validates() {
    let tree = load(DISPATCH);
    assert_eq!(codes(&tree), [], "{:#?}", validate(&tree));
    assert_eq!(
        print(&tree)[0].text,
        DISPATCH,
        "the fixture is written canonically"
    );
    round_trips(&tree);
    let machine = tree.find("Dispatch::Dispatcher::dispatching").unwrap();
    assert_eq!(tree[machine].kind, ElementKind::State);
    assert!(tree[machine].exhibit);
    let transitions = tree[machine]
        .children()
        .iter()
        .filter(|c| tree[**c].kind == ElementKind::Transition)
        .count();
    assert_eq!(transitions, 5);
    let scenario = tree.find("Dispatch::RetryThenDeliver").unwrap();
    assert_eq!(tree[scenario].kind, ElementKind::VerificationDef);
}

#[test]
fn names_in_expressions_follow_renames() {
    let mut tree = load(DISPATCH);
    let attempts = tree.find("Dispatch::Dispatcher::attempts").unwrap();
    let before = tree.references_to(attempts).len();
    assert!(
        before >= 5,
        "guards, effects and values name `attempts`: {before}"
    );
    tree.get_mut(attempts).unwrap().name = Some("tries".into());
    link(&mut tree);
    assert_eq!(codes(&tree), []);
    let text = print(&tree)[0].text.clone();
    assert!(
        text.contains("if not r.ok and tries < maxAttempts"),
        "{text}"
    );
    assert!(text.contains("assign tries := tries + 1;"), "{text}");
    assert!(!text.contains("attempts := "), "{text}");
    // A named argument names the feature of the item it makes.
    let item_attempts = tree.find("Dispatch::Receipt::attempts").unwrap();
    tree.get_mut(item_attempts).unwrap().name = Some("tryCount".into());
    let text = print(&tree)[0].text.clone();
    assert!(text.contains("tryCount = tries"), "{text}");
    assert_eq!(codes(&parse(&print(&tree))), []);
}

#[test]
fn an_enum_value_is_checked_against_the_declared_enum() {
    let tree = load(
        "package P {
    enum def Colour { enum red; enum green; }
    enum def Size { small; large; }
    part def Lamp {
        attribute colour : Colour = Colour::red;
        attribute wrong : Colour = Size::small;
        attribute literal : Colour = 3;
    }
}",
    );
    assert_eq!(
        codes(&tree),
        [
            ("P::Lamp::wrong".to_string(), "wrong-value"),
            ("P::Lamp::literal".to_string(), "wrong-value"),
        ]
    );
    let size = tree.find("P::Size").unwrap();
    assert!(
        tree[size]
            .children()
            .iter()
            .all(|c| tree[*c].kind == ElementKind::Enum)
    );
    assert!(print(&tree)[0].text.contains("enum small;"));
}

#[test]
fn a_dependency_names_its_client_and_supplier() {
    let tree = load(
        "package P {
    part def A;
    part def B;
    dependency from A to B;
    dependency uses from B to A {
        doc /* B uses A. */
    }
    dependency from A to Missing;
}",
    );
    assert_eq!(
        codes(&tree),
        [(
            "P::(dependency from A to Missing)".to_string(),
            "unresolved"
        )]
    );
    round_trips(&tree);
}

#[test]
fn items_cross_ports_only_the_way_their_directions_allow() {
    let text = DISPATCH
        .replace(
            "send new SendRequest(id = current, attempt = attempts) via gateway",
            "send new Receipt(id = current, status = DeliveryStatus::failed, attempts = attempts) via gateway",
        )
        .replace(
            "accept n : Notification via inbox",
            "accept n : Notification via gateway",
        );
    let tree = load(&text);
    let found = codes(&tree);
    assert!(
        found.iter().any(|(_, code)| *code == "incompatible-send"),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|(_, code)| *code == "incompatible-trigger"),
        "{found:?}"
    );
}

#[test]
fn behaviour_in_the_wrong_place_or_without_a_first_state_is_reported() {
    let tree = load(
        "package P {
    part def A {
        exhibit state s {
            state idle;
            state busy;
            transition first idle then busy;
            transition first idle then nowhere;
        }
    }
    part def B {
        state loose;
        assert constraint c { true }
    }
}",
    );
    let found = codes(&tree);
    assert!(
        found.contains(&("P::A::s".to_string(), "initial-state")),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|(at, code)| at.starts_with("P::A::s::") && *code == "unresolved"),
        "{found:?}"
    );
    assert!(
        found.contains(&("P::B::loose".to_string(), "misplaced-behaviour")),
        "{found:?}"
    );
    assert!(
        found.contains(&("P::B::c".to_string(), "misplaced-behaviour")),
        "{found:?}"
    );
}

#[test]
fn an_agents_fallback_shares_its_contract_and_is_not_an_agent() {
    let text = "package P {
    private import ScalarValues::*;
    item def Link { attribute url : String; }
    enum def Decision { enum allow; enum review; enum block; }
    item def Verdict :> Agents::AgentOutput { attribute decision : Decision; }
    port def LinkIn { in item link : Link; }
    port def VerdictOut { out item verdict : Verdict; }
    abstract part def Screening {
        port link : LinkIn;
        port verdict : VerdictOut;
    }
    part def Blocklist :> Screening;
    part def Other;
    part def Nested :> Screening, Agents::Agent {
        :>> mode = Agents::AgentMode::deliberate;
    }
    part def Good :> Screening, Agents::Agent {
        :>> mode = Agents::AgentMode::fast;
        :>> minConfidence = 0.8;
        part :>> fallback : Blocklist;
    }
    part def NotContract :> Screening, Agents::Agent {
        :>> mode = Agents::AgentMode::fast;
        part :>> fallback : Other;
    }
    part def AgentAsFallback :> Screening, Agents::Agent {
        :>> mode = Agents::AgentMode::fast;
        part :>> fallback : Nested;
    }
    part def WrongMode :> Screening, Agents::Agent {
        :>> mode = Decision::allow;
    }
}";
    let tree = load(text);
    assert_eq!(
        codes(&tree),
        [
            ("P::NotContract::fallback".to_string(), "wrong-fallback"),
            ("P::AgentAsFallback::fallback".to_string(), "agent-fallback"),
            ("P::WrongMode::mode".to_string(), "wrong-value"),
        ]
    );
    round_trips(&tree);
}

#[test]
fn the_built_in_library_is_valid_and_offers_agents_and_scenarios() {
    let tree = load(
        "package P {
    part def Uses :> Agents::Agent {
        :>> mode = Agents::AgentMode::fast;
    }
    part def Answer :> Scenarios::StandIn {
        :>> outcome = Scenarios::Outcome::timeout;
    }
}",
    );
    assert_eq!(codes(&tree), []);
    // The library itself validates without problems.
    let library = parse(&[Source::new("library.sysml", agq_language::LIBRARY_TEXT)]);
    assert_eq!(
        validate(&library)
            .into_iter()
            .filter(|d| d.code != "duplicate-name")
            .map(|d| (library.qualified_name(d.element), d.code))
            .collect::<Vec<_>>(),
        []
    );
}

#[test]
fn every_name_in_an_expression_is_a_linked_reference() {
    let tree = load(DISPATCH);
    let mut values = 0;
    for (_, role, reference) in tree.references() {
        if role == Role::Value {
            values += 1;
            assert!(reference.is_linked(), "{reference} is not linked");
        }
    }
    assert!(values > 20, "{values}");
    // `new T(a = …)` holds its argument as the chain `T.a`.
    let scenario = tree
        .find("Dispatch::RetryThenDeliver::failFirst::output")
        .unwrap();
    let Some(Expression::New { arguments, .. }) = &tree[scenario].expression else {
        panic!("a new expression");
    };
    let feature = arguments[0].feature.target().unwrap();
    assert_eq!(tree.qualified_name(feature), "Dispatch::SendResult::id");
}

#[test]
fn unsupported_behaviour_stays_explicit() {
    let tree = load(
        "package P {
    part def A {
        exhibit state s {
            entry; then idle;
            state idle;
            transition first idle accept when ready then idle;
            state def Inner;
        }
        perform action run;
        attribute n = items->size();
        attribute m = f(1);
    }
}",
    );
    let found: Vec<&str> = codes(&tree).into_iter().map(|(_, code)| code).collect();
    assert_eq!(found, ["unsupported"; 5], "{:#?}", validate(&tree));
    assert_eq!(
        print(&tree)[0]
            .text
            .lines()
            .filter(|l| l.contains("when ready"))
            .count(),
        1,
        "kept verbatim"
    );
}
