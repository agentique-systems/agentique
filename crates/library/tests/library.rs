//! The Library: blocks read from the model, found, and used through
//! ordinary System State changes.

use agq_language::{ElementId, ElementKind, Literal, Parent, Source, Tree, parse, print, validate};
use agq_library::{
    BlockRef, Extraction, Index, Library, Override, PlanError, Query, Resolution, Scope, Use,
    built_in, closure,
};
use agq_system_state::{Actor, Change, Operation, Rejection, SystemState};
use std::collections::BTreeSet;

fn state(text: &str) -> SystemState {
    let tree = parse(&[Source::new("Shop.sysml", text)]);
    assert!(validate(&tree).is_empty(), "{:?}", validate(&tree));
    SystemState::new(tree, BTreeSet::new())
}

fn find(state: &SystemState, name: &str) -> ElementId {
    state
        .tree()
        .find(name)
        .unwrap_or_else(|| panic!("no {name}"))
}

fn built(name: &str) -> BlockRef {
    BlockRef::new(Scope::BuiltIn, name)
}

fn apply(state: &mut SystemState, change: Change) {
    state.apply(change).expect("the planned change applies");
    assert!(
        state.diagnostics().is_empty(),
        "{:?}",
        state
            .diagnostics()
            .iter()
            .map(|d| format!("{}: {}", state.tree().qualified_name(d.element), d.message))
            .collect::<Vec<_>>()
    );
}

/// The model as text: what a project saves.
fn text(state: &SystemState) -> String {
    print(state.tree())
        .into_iter()
        .map(|s| s.text)
        .collect::<Vec<_>>()
        .join("\n")
}

const SHOP: &str = "package Shop {
    part def System;
}";

#[test]
fn the_built_in_library_is_valid_small_and_explained() {
    assert!(validate(built_in()).is_empty());
    let index = Index::build(&Library::built_in_only(), None);
    let blocks: Vec<_> = index
        .blocks()
        .iter()
        .filter(|b| b.reference.scope == Scope::BuiltIn && !b.standard)
        .collect();
    assert!(
        (20..=40).contains(&blocks.len()),
        "a small curated library, not {} blocks",
        blocks.len()
    );
    for block in blocks {
        assert!(!block.summary.is_empty(), "{} has no purpose", block.name);
        assert_eq!(block.category.len(), 1, "{}", block.qualified_name);
    }
}

#[test]
fn a_block_is_read_from_the_model() {
    let index = Index::build(&Library::built_in_only(), None);
    let cached = index
        .find(&built("Library::Storage::CachedStore"))
        .expect("indexed");
    assert_eq!(cached.kind, ElementKind::PartDef);
    assert_eq!(cached.category, ["Storage"]);
    assert_eq!(cached.generals, ["Store"]);
    assert!(cached.composite());
    let ports: Vec<_> = cached
        .ports
        .iter()
        .map(|p| {
            (
                p.name.as_str(),
                p.type_name.as_str(),
                p.inherited_from.as_deref(),
            )
        })
        .collect();
    assert_eq!(ports, [("access", "RequestPort", Some("Store"))]);
    assert!(cached.ports[0].inbound);
    let parts: Vec<_> = cached.parts.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(parts, ["cache", "store"]);
    assert_eq!(cached.connections, 2);
    assert_eq!(cached.requirements.len(), 1);
    assert!(cached.requirements[0].text.contains("satisfied by cache"));
    let gateway = index.find(&built("Library::Services::Gateway")).unwrap();
    let backend = gateway.ports.iter().find(|p| p.name == "backend").unwrap();
    assert_eq!(backend.type_name, "~RequestPort");
    assert!(!backend.inbound, "a conjugated request port calls out");
    // Standard definitions are blocks too, used by reference.
    let string = index.find(&built("ScalarValues::String")).unwrap();
    assert!(string.standard);
}

#[test]
fn search_finds_blocks_by_name_purpose_and_ports() {
    let index = Index::build(&Library::built_in_only(), None);
    let names = |text: &str| -> Vec<String> {
        index
            .search(&Query {
                text,
                ..Default::default()
            })
            .into_iter()
            .map(|hit| index.blocks()[hit.block].name.clone())
            .collect()
    };
    assert_eq!(names("cache")[0], "Cache");
    assert!(names("cache").contains(&"CachedStore".to_string()));
    let rate = names("rate limit");
    assert!(
        rate.starts_with(&["RateLimiter".to_string()]) || rate[0] == "RateLimitedApi",
        "{rate:?}"
    );
    assert!(rate.contains(&"RateLimitedApi".to_string()));
    // By purpose: "publish" is in the topic's documentation.
    assert!(names("publish").contains(&"Topic".to_string()));
    // By port: a worker's port is `jobs`.
    assert!(names("jobs").contains(&"Worker".to_string()));
    assert!(names("zzz").is_empty());
    // Filters.
    let ports = index.search(&Query {
        kinds: &[ElementKind::PortDef],
        ..Default::default()
    });
    assert!(
        ports
            .iter()
            .all(|h| index.blocks()[h.block].kind == ElementKind::PortDef)
    );
    assert_eq!(ports.len(), 2);
    let mine = index.search(&Query {
        scope: Some(Scope::Mine),
        ..Default::default()
    });
    assert!(mine.is_empty());
    // The matched characters of the name are given for marking.
    let hit = &index.search(&Query {
        text: "kvs",
        ..Default::default()
    })[0];
    assert_eq!(index.blocks()[hit.block].name, "KeyValueStore");
    assert_eq!(hit.name_positions, [0, 3, 8]);
}

#[test]
fn categories_are_the_packages() {
    let index = Index::build(&Library::built_in_only(), None);
    let categories = index.categories(Some(Scope::BuiltIn));
    for expected in [
        "Interfaces",
        "Messages",
        "Messaging",
        "Resilience",
        "Services",
        "Storage",
    ] {
        assert!(
            categories.contains(&vec![expected.to_string()]),
            "{expected}"
        );
    }
}

#[test]
fn the_closure_is_what_a_block_needs_and_nothing_standard() {
    let tree = built_in();
    let cached = tree.find("Library::Storage::CachedStore").unwrap();
    let names: BTreeSet<String> = closure(tree, cached)
        .unwrap()
        .into_iter()
        .map(|u| tree.qualified_name(u))
        .collect();
    let expected: BTreeSet<String> = [
        "Library::Storage::CachedStore",
        "Library::Storage::Store",
        "Library::Storage::Cache",
        "Library::Storage::KeyValueStore",
        "Library::Storage::FreshReads",
        "Library::Interfaces::RequestPort",
        "Library::Interfaces::RequestResponse",
        "Library::Messages::Request",
        "Library::Messages::Response",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    assert_eq!(names, expected);
}

#[test]
fn using_a_block_copies_what_it_needs_and_adds_one_typed_usage() {
    let library = Library::built_in_only();
    let mut state = state(SHOP);
    let system = find(&state, "Shop::System");
    let before = text(&state);
    let request = Use::new(
        built("Library::Storage::CachedStore"),
        Parent::Element(system),
    );
    let plan = library.plan_use(&state, &request, Actor::Operator).unwrap();
    assert_eq!(
        plan.change.description,
        "Add cachedStore : CachedStore from the Library"
    );
    assert_eq!(plan.imported.len(), 9, "{:?}", plan.imported);
    assert!(plan.problems.is_empty(), "{:?}", plan.problems);
    let usage = plan.created.unwrap();
    apply(&mut state, plan.change);
    let definition = find(&state, "Library::Storage::CachedStore");
    let tree = state.tree();
    assert_eq!(tree[usage].name.as_deref(), Some("cachedStore"));
    assert_eq!(tree[usage].owner(), Some(system));
    assert_eq!(tree[usage].typed_by[0].target(), Some(definition));
    // The definition's inside is not copied into the usage: it is found
    // through the type.
    assert!(tree[usage].children().is_empty());
    // One undo step removes all of it.
    state.undo();
    assert_eq!(text(&state), before);
    state.redo();
    assert!(state.tree().find("Library::Storage::CachedStore").is_some());
}

#[test]
fn using_a_block_again_reuses_the_identical_copies() {
    let library = Library::built_in_only();
    let mut state = state(SHOP);
    let system = find(&state, "Shop::System");
    let request = Use::new(
        built("Library::Storage::CachedStore"),
        Parent::Element(system),
    );
    let change = library
        .plan_use(&state, &request, Actor::Operator)
        .unwrap()
        .change;
    apply(&mut state, change);
    let elements = state.tree().len();
    let plan = library.plan_use(&state, &request, Actor::Operator).unwrap();
    assert!(plan.imported.is_empty(), "{:?}", plan.imported);
    assert_eq!(plan.reused.len(), 9);
    apply(&mut state, plan.change);
    assert_eq!(
        state.tree().len(),
        elements + 1,
        "only the second usage is new"
    );
    let tree = state.tree();
    let usage = find(&state, "Shop::System::cachedStore2");
    assert_eq!(
        tree[usage].typed_by[0].target(),
        tree.find("Library::Storage::CachedStore")
    );
    // An index shows the project's copy in place of the built-in block.
    let index = Index::build(&library, Some((tree, state.revision())));
    let hits = index.search(&Query {
        text: "CachedStore",
        ..Default::default()
    });
    let block = &index.blocks()[hits[0].block];
    assert_eq!(block.reference.scope, Scope::Project);
    assert_eq!(block.source_label(), "Project · from Built-in");
    assert_eq!(block.usages, 2);
}

const OWN_CACHE: &str = "package Shop {
    part def System;
}
package Library {
    package Storage {
        part def Cache {
            doc /* The shop's own cache. */
            attribute ttlSeconds : ScalarValues::Positive = 60;
        }
    }
}";

#[test]
fn a_different_definition_with_the_same_name_is_never_overwritten() {
    let library = Library::built_in_only();
    let mut state = state(OWN_CACHE);
    let system = find(&state, "Shop::System");
    let own = find(&state, "Library::Storage::Cache");
    let mut request = Use::new(
        built("Library::Storage::CachedStore"),
        Parent::Element(system),
    );
    let Err(PlanError::Conflicts(conflicts)) = library.plan_use(&state, &request, Actor::Operator)
    else {
        panic!("a conflict")
    };
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].qualified_name, "Library::Storage::Cache");
    assert_eq!(conflicts[0].existing, own);
    assert!(
        conflicts[0].difference.contains("documentation"),
        "{}",
        conflicts[0].difference
    );
    // Copy under another name: the project's Cache stays as it is.
    request.resolution = Resolution::Rename;
    let plan = library.plan_use(&state, &request, Actor::Operator).unwrap();
    assert_eq!(
        plan.renamed,
        [(
            "Library::Storage::Cache".to_string(),
            "Library::Storage::Cache2".to_string()
        )]
    );
    apply(&mut state, plan.change);
    let tree = state.tree();
    let cache = find(&state, "Library::Storage::CachedStore::cache");
    assert_eq!(
        tree[cache].typed_by[0].target(),
        tree.find("Library::Storage::Cache2")
    );
    assert_eq!(tree[cache].typed_by[0].to_string(), "Cache2");
    assert!(text(&state).contains("attribute ttlSeconds : ScalarValues::Positive = 60;"));
}

#[test]
fn a_conflict_can_be_settled_by_using_the_projects_definition() {
    let library = Library::built_in_only();
    let mut state = state(OWN_CACHE);
    let system = find(&state, "Shop::System");
    let own = find(&state, "Library::Storage::Cache");
    let mut request = Use::new(
        built("Library::Storage::CachedStore"),
        Parent::Element(system),
    );
    request.resolution = Resolution::UseExisting;
    let plan = library.plan_use(&state, &request, Actor::Operator).unwrap();
    // The project's cache has no ports, so the copied connections cannot
    // find theirs: that is reported, not hidden.
    assert!(!plan.problems.is_empty());
    state.apply(plan.change).unwrap();
    let cache = find(&state, "Library::Storage::CachedStore::cache");
    assert_eq!(state.tree()[cache].typed_by[0].target(), Some(own));
}

#[test]
fn values_override_inherited_attributes_in_the_usage_only() {
    let library = Library::built_in_only();
    let mut state = state(SHOP);
    let system = find(&state, "Shop::System");
    let mut request = Use::new(built("Library::Storage::Cache"), Parent::Element(system));
    request.name = Some("sessions".into());
    request.values = vec![("ttlSeconds".into(), Literal::Integer("60".into()))];
    let plan = library.plan_use(&state, &request, Actor::Operator).unwrap();
    apply(&mut state, plan.change);
    let saved = text(&state);
    assert!(
        saved.contains("part sessions : Library::Storage::Cache {"),
        "{saved}"
    );
    assert!(saved.contains("attribute :>> ttlSeconds = 60;"), "{saved}");
    assert!(saved.contains("attribute ttlSeconds : ScalarValues::Positive = 300;"));
    // Only attributes take values.
    request.values = vec![("backend".into(), Literal::Integer("1".into()))];
    let Err(PlanError::Invalid(reason)) = library.plan_use(&state, &request, Actor::Operator)
    else {
        panic!("refused")
    };
    assert!(
        reason.contains("`backend` is a port, not an attribute"),
        "{reason}"
    );
    request.values = vec![("nothing".into(), Literal::Integer("1".into()))];
    let Err(PlanError::Invalid(reason)) = library.plan_use(&state, &request, Actor::Operator)
    else {
        panic!("refused")
    };
    assert!(reason.contains("no attribute `nothing`"), "{reason}");
    // Attributes of inner parts are reached by their path, and share the
    // redefinition of the part.
    let mut nested = Use::new(
        built("Library::Storage::CachedStore"),
        Parent::Element(system),
    );
    nested.name = Some("pages".into());
    nested.values = vec![
        ("cache.ttlSeconds".into(), Literal::Integer("30".into())),
        ("cache.maxEntries".into(), Literal::Integer("1000".into())),
    ];
    let plan = library.plan_use(&state, &nested, Actor::Operator).unwrap();
    apply(&mut state, plan.change);
    let saved = text(&state);
    assert!(saved.contains("part :>> cache {"), "{saved}");
    assert_eq!(saved.matches("part :>> cache").count(), 1, "{saved}");
    assert!(saved.contains("attribute :>> ttlSeconds = 30;"), "{saved}");
    assert!(
        saved.contains("attribute :>> maxEntries = 1000;"),
        "{saved}"
    );
}

#[test]
fn a_block_can_be_used_and_connected_in_one_change() {
    let library = Library::built_in_only();
    let mut state = state(SHOP);
    let system = find(&state, "Shop::System");
    let mut gateway = Use::new(built("Library::Services::Gateway"), Parent::Element(system));
    gateway.name = Some("front".into());
    let change = library
        .plan_use(&state, &gateway, Actor::Operator)
        .unwrap()
        .change;
    apply(&mut state, change);
    let front = find(&state, "Shop::System::front");
    let backend = find(&state, "Library::Services::Gateway::backend");
    let mut store = Use::new(
        built("Library::Storage::CachedStore"),
        Parent::Element(system),
    );
    store.connect = Some(agq_library::ConnectTo {
        card: front,
        port: backend,
        with: None,
    });
    let plan = library.plan_use(&state, &store, Actor::Operator).unwrap();
    assert_eq!(
        plan.connected.as_deref(),
        Some("cachedStore.access to front.backend")
    );
    assert!(
        plan.change
            .description
            .ends_with("connected to front.backend")
    );
    let events = plan.change.operations.len();
    apply(&mut state, plan.change);
    assert!(events > 1);
    let saved = text(&state);
    assert!(
        saved.contains("interface connect front.backend to cachedStore.access;"),
        "{saved}"
    );
    // A port that does not fit is explained with the model's own rule.
    let mut worker = Use::new(built("Library::Messaging::Worker"), Parent::Element(system));
    worker.connect = Some(agq_library::ConnectTo {
        card: front,
        port: backend,
        with: None,
    });
    let Err(PlanError::Invalid(reason)) = library.plan_use(&state, &worker, Actor::Operator) else {
        panic!("refused")
    };
    assert!(reason.contains("no port of `worker` fits"), "{reason}");
}

#[test]
fn nested_composites_bring_their_insides() {
    let library = Library::built_in_only();
    let mut state = state(SHOP);
    let system = find(&state, "Shop::System");
    let plan = library
        .plan_use(
            &state,
            &Use::new(
                built("Library::Messaging::EventProcessing"),
                Parent::Element(system),
            ),
            Actor::Operator,
        )
        .unwrap();
    for needed in [
        "Library::Messaging::AsyncWorker",
        "Library::Messaging::Queue",
        "Library::Messaging::Worker",
        "Library::Messaging::Topic",
        "Library::Messaging::JobsProcessed",
        "Library::Interfaces::MessageChannel",
    ] {
        assert!(plan.imported.iter().any(|i| i == needed), "{needed}");
    }
    apply(&mut state, plan.change);
}

#[test]
fn specialising_changes_only_the_specialisation() {
    let library = Library::built_in_only();
    let mut state = state(SHOP);
    let system = find(&state, "Shop::System");
    let mut request = Use::new(
        built("Library::Storage::CachedStore"),
        Parent::Element(system),
    );
    request.name = Some("sessions".into());
    let plan = library.plan_use(&state, &request, Actor::Operator).unwrap();
    let sessions = plan.created.unwrap();
    apply(&mut state, plan.change);
    let cached = find(&state, "Library::Storage::CachedStore");
    let plan = library
        .plan_specialize(
            &state,
            cached,
            "SessionStore",
            Some(sessions),
            Actor::Operator,
        )
        .unwrap();
    assert_eq!(
        plan.change.description,
        "Specialise CachedStore as SessionStore for sessions"
    );
    let special = plan.created.unwrap();
    apply(&mut state, plan.change);
    // Beside the Operator's own definitions, not in the copied Library.
    assert_eq!(find(&state, "Shop::SessionStore"), special);
    assert_eq!(state.tree()[sessions].typed_by[0].target(), Some(special));
    // Override the cache's time to live in the specialisation only.
    let cache = find(&state, "Library::Storage::CachedStore::cache");
    let ttl = find(&state, "Library::Storage::Cache::ttlSeconds");
    let plan = library
        .plan_override(
            &state,
            special,
            &[cache, ttl],
            Override::Value(Literal::Integer("60".into())),
            Actor::Operator,
        )
        .unwrap();
    apply(&mut state, plan.change);
    let saved = text(&state);
    assert!(
        saved.contains("part def SessionStore :> Library::Storage::CachedStore {"),
        "{saved}"
    );
    assert!(saved.contains("part :>> cache {"), "{saved}");
    assert!(saved.contains("attribute :>> ttlSeconds = 60;"), "{saved}");
    // The original definition is unchanged: still an unchanged copy.
    let index = Index::build(&library, Some((state.tree(), state.revision())));
    let original = &index.blocks()[index.project_block(cached).unwrap()];
    assert_eq!(original.source_label(), "Project · from Built-in");
    // Overriding again changes the same redefinition.
    let plan = library
        .plan_override(
            &state,
            special,
            &[cache, ttl],
            Override::Value(Literal::Integer("90".into())),
            Actor::Operator,
        )
        .unwrap();
    assert!(
        plan.change
            .operations
            .iter()
            .all(|o| matches!(o, Operation::Set { .. }))
    );
    apply(&mut state, plan.change);
    assert!(text(&state).contains("attribute :>> ttlSeconds = 90;"));
    // A feature the owner has itself is changed there, not overridden.
    let own = find(&state, "Library::Storage::Cache::ttlSeconds");
    let cache_def = find(&state, "Library::Storage::Cache");
    assert!(
        library
            .plan_override(
                &state,
                cache_def,
                &[own],
                Override::Value(Literal::Integer("1".into())),
                Actor::Operator
            )
            .is_err()
    );
}

#[test]
fn project_definitions_are_blocks_where_they_are() {
    let library = Library::built_in_only();
    let mut state = state(
        "package Shop {
    part def Checkout { doc /* Takes payment for an order. */ }
    part def System;
}",
    );
    let index = Index::build(&library, Some((state.tree(), state.revision())));
    let hit = &index.search(&Query {
        text: "payment",
        scope: Some(Scope::Project),
        ..Default::default()
    })[0];
    let block = &index.blocks()[hit.block];
    assert_eq!(block.name, "Checkout");
    assert_eq!(block.summary, "Takes payment for an order.");
    let system = find(&state, "Shop::System");
    let elements = state.tree().len();
    let plan = library
        .plan_use(
            &state,
            &Use::new(block.reference.clone(), Parent::Element(system)),
            Actor::Operator,
        )
        .unwrap();
    assert_eq!(plan.change.description, "Add checkout : Checkout");
    assert!(plan.imported.is_empty());
    apply(&mut state, plan.change);
    assert_eq!(state.tree().len(), elements + 1);
}

#[test]
fn standard_definitions_are_used_by_reference() {
    let library = Library::built_in_only();
    let mut state = state(SHOP);
    let system = find(&state, "Shop::System");
    let mut request = Use::new(built("ScalarValues::String"), Parent::Element(system));
    request.name = Some("region".into());
    let plan = library.plan_use(&state, &request, Actor::Operator).unwrap();
    assert!(plan.imported.is_empty());
    apply(&mut state, plan.change);
    assert!(text(&state).contains("attribute region : ScalarValues::String;"));
}

const CHECKOUT: &str = "package Shop {
    item def Order;
    port def PaymentPort {
        doc /* Where payments are asked for. */
        in item order : Order;
    }
    part def Checkout {
        doc /* Takes payment for an order. */
        port pay : ~PaymentPort;
        attribute retries : ScalarValues::Natural = 2;
    }
    part def System;
}";

#[test]
fn my_library_keeps_blocks_for_other_projects_which_stay_self_contained() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("library").join("My Library.sysml");
    let mut library = Library::with_mine(path.clone());
    let shop = state(CHECKOUT);
    let checkout = find(&shop, "Shop::Checkout");
    let plan = library
        .plan_save(shop.tree(), checkout, "Payments", false)
        .unwrap();
    assert_eq!(plan.block.to_string(), "mine:Library::Payments::Checkout");
    assert_eq!(plan.added.len(), 3, "{:?}", plan.added);
    assert!(plan.problems.is_empty(), "{:?}", plan.problems);
    library.save(plan).unwrap();
    let saved = std::fs::read_to_string(&path).unwrap();
    assert!(saved.contains("package Payments {"), "{saved}");
    assert!(saved.contains("port pay : ~PaymentPort;"), "{saved}");
    // Another session reads it back.
    let library = Library::with_mine(path.clone());
    assert!(library.problems().is_empty(), "{:?}", library.problems());
    let index = Index::build(&library, None);
    let block = index
        .find(&BlockRef::new(Scope::Mine, "Library::Payments::Checkout"))
        .expect("in My Library");
    assert_eq!(block.summary, "Takes payment for an order.");
    // Another project uses it: it gets its own copies.
    let mut other = state("package Store { part def System; }");
    let system = find(&other, "Store::System");
    let plan = library
        .plan_use(
            &other,
            &Use::new(block.reference.clone(), Parent::Element(system)),
            Actor::Operator,
        )
        .unwrap();
    apply(&mut other, plan.change);
    let copied = text(&other);
    // Self-contained: the project's text alone reads back without problems.
    let alone = parse(&[Source::new("Store.sysml", &copied)]);
    assert!(validate(&alone).is_empty(), "{:?}", validate(&alone));
    // Changing My Library later does not change the project.
    let mut changed = state(&CHECKOUT.replace("= 2;", "= 5;"));
    let checkout = find(&changed, "Shop::Checkout");
    let mut library = library;
    let Err(PlanError::Conflicts(conflicts)) =
        library.plan_save(changed.tree(), checkout, "Payments", false)
    else {
        panic!("saving a changed block again asks first")
    };
    assert_eq!(conflicts[0].qualified_name, "Library::Payments::Checkout");
    let plan = library
        .plan_save(changed.tree(), checkout, "Payments", true)
        .unwrap();
    assert_eq!(plan.replaced, ["Library::Payments::Checkout"]);
    library.save(plan).unwrap();
    assert!(std::fs::read_to_string(&path).unwrap().contains("= 5;"));
    assert_eq!(text(&other), copied, "the project keeps its own copy");
    let _ = &mut changed;
}

#[test]
fn a_block_can_be_removed_from_my_library_unless_another_uses_it() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("My Library.sysml");
    let mut library = Library::with_mine(path);
    let shop = state(CHECKOUT);
    let checkout = find(&shop, "Shop::Checkout");
    let plan = library
        .plan_save(shop.tree(), checkout, "Payments", false)
        .unwrap();
    library.save(plan).unwrap();
    let refused = library
        .remove_mine("Library::Payments::PaymentPort")
        .unwrap_err();
    assert!(
        refused.contains("used by `Library::Payments::Checkout`"),
        "{refused}"
    );
    library.remove_mine("Library::Payments::Checkout").unwrap();
    assert!(library.mine().find("Library::Payments::Checkout").is_none());
    assert!(
        library
            .mine()
            .find("Library::Payments::PaymentPort")
            .is_some()
    );
}

#[test]
fn a_malformed_my_library_is_reported_and_the_rest_still_works() {
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("My Library.sysml");
    std::fs::write(
        &path,
        "package Library {
    package Mine {
        part def Good { doc /* Works. */ }
        part def Broken { part x : Missing; }
        part def Garbled { port p : ; }
    }
}",
    )
    .unwrap();
    let library = Library::with_mine(path);
    assert!(library.problems().len() >= 2, "{:?}", library.problems());
    let index = Index::build(&library, None);
    let good = index
        .find(&BlockRef::new(Scope::Mine, "Library::Mine::Good"))
        .unwrap();
    assert_eq!(good.problems, 0);
    assert!(
        index
            .find(&BlockRef::new(Scope::Mine, "Library::Mine::Broken"))
            .unwrap()
            .problems
            > 0
    );
    assert!(index.find(&built("Library::Storage::Cache")).is_some());
    // A block that needs something missing cannot be used, and says why.
    let state = state(SHOP);
    let system = find(&state, "Shop::System");
    let Err(PlanError::Missing(missing)) = library.plan_use(
        &state,
        &Use::new(
            BlockRef::new(Scope::Mine, "Library::Mine::Broken"),
            Parent::Element(system),
        ),
        Actor::Operator,
    ) else {
        panic!("refused")
    };
    assert_eq!(missing[0].reference, "Missing");
    // The good one can.
    assert!(
        library
            .plan_use(
                &state,
                &Use::new(
                    BlockRef::new(Scope::Mine, "Library::Mine::Good"),
                    Parent::Element(system)
                ),
                Actor::Operator
            )
            .is_ok()
    );
}

#[test]
fn a_deleted_definition_is_no_longer_a_block() {
    let library = Library::built_in_only();
    let mut state = state(
        "package Shop {
    part def Checkout;
    part def System;
}",
    );
    let checkout = find(&state, "Shop::Checkout");
    let mut index = Index::build(&library, Some((state.tree(), state.revision())));
    let reference = BlockRef::new(Scope::Project, "Shop::Checkout");
    assert!(index.find(&reference).is_some());
    state
        .apply(Change::new(
            Actor::Operator,
            "Delete Checkout",
            vec![Operation::Delete { element: checkout }],
        ))
        .unwrap();
    assert!(!index.is_current(&library, Some(state.revision())));
    index.refresh(&library, Some((state.tree(), state.revision())));
    assert!(index.find(&reference).is_none());
    let system = find(&state, "Shop::System");
    let Err(PlanError::Invalid(reason)) = library.plan_use(
        &state,
        &Use::new(reference, Parent::Element(system)),
        Actor::Operator,
    ) else {
        panic!("refused")
    };
    assert!(reason.contains("no building block"), "{reason}");
}

#[test]
fn using_a_block_inside_a_locked_part_needs_the_operators_confirmation() {
    let library = Library::built_in_only();
    let mut state = state(SHOP);
    let system = find(&state, "Shop::System");
    state
        .apply(Change::new(
            Actor::Operator,
            "Lock System",
            vec![Operation::Lock { element: system }],
        ))
        .unwrap();
    let plan = library
        .plan_use(
            &state,
            &Use::new(built("Library::Storage::Cache"), Parent::Element(system)),
            Actor::Assistant,
        )
        .unwrap();
    let Err(Rejection::Locked { elements }) = state.apply(plan.change.clone()) else {
        panic!("locked")
    };
    assert_eq!(elements, [system]);
    let mut confirmed = plan.change;
    confirmed.confirmed = elements;
    apply(&mut state, confirmed);
}

#[test]
fn compatible_blocks_are_found_by_the_models_own_rule() {
    let library = Library::built_in_only();
    let mut state = state(SHOP);
    let system = find(&state, "Shop::System");
    let change = library
        .plan_use(
            &state,
            &Use::new(built("Library::Services::Gateway"), Parent::Element(system)),
            Actor::Operator,
        )
        .unwrap()
        .change;
    apply(&mut state, change);
    let backend = find(&state, "Library::Services::Gateway::backend");
    let index = Index::build(&library, Some((state.tree(), state.revision())));
    let fits: Vec<(String, Vec<String>)> = library
        .compatible(&index, state.tree(), backend, false)
        .into_iter()
        .map(|fit| (index.blocks()[fit.block].name.clone(), fit.ports))
        .collect();
    let names: Vec<&str> = fits.iter().map(|(n, _)| n.as_str()).collect();
    for expected in [
        "CachedStore",
        "Store",
        "KeyValueStore",
        "Service",
        "RateLimitedApi",
    ] {
        assert!(names.contains(&expected), "{expected} in {names:?}");
    }
    for unfit in ["Queue", "Worker", "Topic", "Scheduler"] {
        assert!(!names.contains(&unfit), "{unfit} in {names:?}");
    }
    let store = fits.iter().find(|(n, _)| n == "CachedStore").unwrap();
    assert_eq!(store.1, ["access"]);
    // The ports on the Surface a dragged block could face.
    let block = index.position(&built("Library::Messaging::Queue")).unwrap();
    assert!(
        library
            .fitting_ports(&index, block, state.tree(), &[backend])
            .is_empty()
    );
    let block = index.position(&built("Library::Storage::Store")).unwrap();
    assert_eq!(
        library.fitting_ports(&index, block, state.tree(), &[backend]),
        [backend]
    );
}

const WIRED: &str = "package Shop {
    item def Query;
    item def Answer;
    port def Serve { in item query : Query; out item answer : Answer; }
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
    requirement def Durable { doc /* Data survives a restart. */ subject back : Back; }
    part system : System;
    requirement durable : Durable;
    satisfy durable by system.back;
}";

#[test]
fn a_selection_becomes_a_block_in_one_change_and_keeps_its_identity() {
    let library = Library::built_in_only();
    let mut state = state(WIRED);
    let front = find(&state, "Shop::System::front");
    let back = find(&state, "Shop::System::back");
    let before = text(&state);
    let extraction = Extraction::analyse(&state, &[front, back]).unwrap();
    assert!(extraction.blockers.is_empty(), "{:?}", extraction.blockers);
    assert_eq!(extraction.internal.len(), 1);
    assert_eq!(extraction.boundary.len(), 1);
    assert_eq!(extraction.boundary[0].name, "access");
    assert_eq!(extraction.boundary[0].part, front);
    let plan = extraction
        .plan(&state, "Backend", "backend", Actor::Operator)
        .unwrap();
    assert_eq!(
        plan.change.description,
        "Create building block Backend from front, back"
    );
    apply(&mut state, plan.change);
    let saved = text(&state);
    // The parts moved inside, with their identity.
    assert_eq!(find(&state, "Shop::Backend::front"), front);
    assert_eq!(find(&state, "Shop::Backend::back"), back);
    assert!(saved.contains("part backend : Backend;"), "{saved}");
    assert!(saved.contains("port access : Serve;"), "{saved}");
    assert!(saved.contains("connect access to front.access;"), "{saved}");
    assert!(
        saved.contains("interface connect client.calls to backend.access;"),
        "{saved}"
    );
    assert!(
        saved.contains("satisfy durable by system.backend.back;"),
        "{saved}"
    );
    state.undo();
    assert_eq!(text(&state), before);
    let _ = library;
}

#[test]
fn an_unclear_boundary_is_reported_before_anything_changes() {
    let model = state(
        "package Shop {
    part def A;
    part def B;
    part def System {
        part a : A;
        part b : B;
        connection connect a to b;
    }
}",
    );
    let a = find(&model, "Shop::System::a");
    let extraction = Extraction::analyse(&model, &[a]).unwrap();
    assert_eq!(extraction.blockers.len(), 1);
    assert!(
        extraction.blockers[0].contains("not a port"),
        "{}",
        extraction.blockers[0]
    );
    assert!(extraction.plan(&model, "X", "x", Actor::Operator).is_err());
    // Parts from different owners.
    let other = state(
        "package Shop {
    part def X { part a; }
    part def Y { part b; }
}",
    );
    let (a, b) = (find(&other, "Shop::X::a"), find(&other, "Shop::Y::b"));
    assert!(Extraction::analyse(&other, &[a, b]).is_err());
}

#[test]
fn block_references_are_written_and_read_back() {
    let reference = built("Library::Storage::Cache");
    assert_eq!(reference.to_string(), "built-in:Library::Storage::Cache");
    assert_eq!(BlockRef::parse(&reference.to_string()), Some(reference));
    assert_eq!(
        BlockRef::parse("mine:Library::A").unwrap().scope,
        Scope::Mine
    );
    assert!(BlockRef::parse("elsewhere:Library::A").is_none());
    assert!(BlockRef::parse("project:").is_none());
}

#[test]
fn the_description_is_words_not_sysml() {
    let library = Library::built_in_only();
    let index = Index::build(&library, None);
    let block = index
        .position(&built("Library::Storage::CachedStore"))
        .unwrap();
    let text = library.describe(&index, block, None);
    for expected in [
        "built-in:Library::Storage::CachedStore",
        "Purpose: A store with a cache",
        "Ports: access : RequestPort (from Store) — serves or receives",
        "RequestPort carries in request : Request, out response : Response",
        "Parts: cache : Cache; store : KeyValueStore",
        "Inner attributes: cache.ttlSeconds : Positive = 300",
        "Needs (copied with it unless the project has them)",
        "In the project: not yet.",
    ] {
        assert!(text.contains(expected), "{expected}\n{text}");
    }
    assert!(!text.contains("part def CachedStore"), "no SysML text");
    assert!(!text.contains('{'));
    let preview = library.preview(&index, block, None).unwrap();
    assert_eq!(preview.ports.len(), 1);
    assert_eq!(preview.parts.len(), 2);
    assert_eq!(preview.links.len(), 2);
    // The cache calls the store: the store is to its right.
    assert!(preview.parts[1].column > preview.parts[0].column);
}

/// A My Library of 5,000 definitions still searches in a moment.
#[test]
fn search_stays_fast_in_a_large_library() {
    use std::fmt::Write;
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("My Library.sysml");
    let mut text = String::from("package Library {\n");
    for package in 0..50 {
        let _ = writeln!(text, "    package Area{package} {{");
        for i in 0..100 {
            let _ = writeln!(
                text,
                "        part def Component{package}x{i} {{ doc /* Handles case {i} of area {package}. */ attribute size : ScalarValues::Natural = {i}; }}"
            );
        }
        text.push_str("    }\n");
    }
    text.push_str("}\n");
    std::fs::write(&path, text).unwrap();
    let library = Library::with_mine(path);
    let started = std::time::Instant::now();
    let index = Index::build(&library, None);
    let built = started.elapsed();
    assert!(index.blocks().len() > 5_000);
    let started = std::time::Instant::now();
    let mut found = 0;
    for query in ["comp", "component12x7", "area 3", "case", "xyz"] {
        found += index
            .search(&Query {
                text: query,
                limit: 200,
                ..Default::default()
            })
            .len();
    }
    let searched = started.elapsed() / 5;
    println!(
        "index of {} blocks built in {built:?}; a search takes {searched:?}",
        index.blocks().len()
    );
    assert!(found > 0);
    // Budget (§3.3): a keystroke's search in 8 ms or less at 5,000 blocks.
    // Debug builds are several times slower; the ceiling is for release.
    let ceiling = if cfg!(debug_assertions) { 400 } else { 8 };
    assert!(
        searched < std::time::Duration::from_millis(ceiling),
        "{searched:?}"
    );
}

#[allow(dead_code)]
fn unused(_: &Tree) {}
