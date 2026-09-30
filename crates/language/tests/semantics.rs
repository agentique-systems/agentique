//! `Semantics` answers with the same lookup and rules as `validate`.

use agq_language::{Semantics, Source, parse, validate};

const MODEL: &str = "package P {
    item def Request;
    item def Order :> Request;
    port def Serve { in item request : Request; out item response : Request; }
    port def TakeOrders { in item request : Order; out item response : Request; }
    part def Service { port api : Serve; attribute limit : ScalarValues::Natural = 5; }
    part def Shop :> Service { :>> limit = 10; port orders : TakeOrders; }
    part def Client { port calls : ~Serve; port orderCalls : ~TakeOrders; }
    part def System {
        part shop : Shop;
        part client : Client;
        port front : Serve;
        interface connect client.calls to shop.api;
        connection connect front to shop.api;
    }
}";

#[test]
fn features_are_owned_and_inherited_with_redefinitions_in_place() {
    let tree = parse(&[Source::new("p.sysml", MODEL)]);
    assert!(validate(&tree).is_empty(), "{:?}", validate(&tree));
    let semantics = Semantics::new(&tree);
    let shop = tree.find("P::Shop").unwrap();
    let names: Vec<&str> = semantics
        .features(shop)
        .into_iter()
        .map(|id| tree.effective_name(id).unwrap())
        .collect();
    assert_eq!(names, ["limit", "orders", "api"]);
    let api = tree.find("P::Service::api").unwrap();
    assert!(semantics.is_inherited(shop, api));
    let redefined = tree.find("P::Shop::limit").unwrap();
    assert!(!semantics.is_inherited(shop, redefined));
    // A usage has its type's features.
    let usage = tree.find("P::System::shop").unwrap();
    assert!(semantics.features(usage).contains(&api));
    assert!(semantics.specializes(shop, tree.find("P::Service").unwrap()));
}

#[test]
fn ports_fit_by_the_connection_rule() {
    let tree = parse(&[Source::new("p.sysml", MODEL)]);
    let semantics = Semantics::new(&tree);
    let find = |name: &str| tree.find(name).unwrap();
    let api = find("P::Service::api");
    let calls = find("P::Client::calls");
    let orders = find("P::Shop::orders");
    let order_calls = find("P::Client::orderCalls");
    let front = find("P::System::front");
    // Facing each other: the conjugate fits.
    assert_eq!(semantics.ports_fit(calls, api, false), Ok(()));
    assert_eq!(semantics.ports_fit(order_calls, orders, false), Ok(()));
    // Same directions face each other: no.
    let problem = semantics.ports_fit(api, api, false).unwrap_err();
    assert!(problem.contains("request"), "{problem}");
    // Sending a general Request where an Order is received: no.
    let problem = semantics.ports_fit(calls, orders, false).unwrap_err();
    assert!(problem.contains("sends"), "{problem}");
    // Passing items on to an inner part keeps the directions.
    assert_eq!(semantics.ports_fit(front, api, true), Ok(()));
    assert!(semantics.ports_fit(front, api, false).is_err());
    // Only ports.
    assert!(semantics.ports_fit(find("P::Shop"), api, false).is_err());
}

#[test]
fn conjugated_types_are_reported_with_their_flag() {
    let tree = parse(&[Source::new("p.sysml", MODEL)]);
    let semantics = Semantics::new(&tree);
    let calls = tree.find("P::Client::calls").unwrap();
    let serve = tree.find("P::Serve").unwrap();
    assert_eq!(semantics.types_of(calls), [(serve, true)]);
    // Library elements are elements too.
    let limit = tree.find("P::Service::limit").unwrap();
    let (natural, _) = semantics.types_of(limit)[0];
    assert!(semantics.is_library(natural));
    assert_eq!(
        semantics.element(natural).and_then(|e| e.name.as_deref()),
        Some("Natural")
    );
}
