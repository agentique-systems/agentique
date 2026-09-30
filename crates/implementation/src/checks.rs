//! The supported implementation checks (ROADMAP §4.15), each with its
//! coverage stated beside its result:
//!
//! - **Dependency boundaries**: which linked modules (or crates) refer to
//!   which, against the model's `dependency` relationships.
//! - **Contract shapes**: a linked Rust struct or enum against the item or
//!   enum def it implements.
//! - **Linked tests**: the tests linked to elements, run and reported one by
//!   one.
//!
//! (Scenarios against the implementation are in [`crate::harness`].) A
//! failing check is drift at the elements it covers; a file existing at a
//! linked path is never counted as conformance.

use crate::links::{LinkKind, Links};
use crate::rust::{self, TypeKind};
use agq_execution::{Executor, Program};
use agq_language::{ElementId, ElementKind, Role, Semantics, Tree};
use agq_simulation::Verdict;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

/// Which check a result comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckKind {
    DependencyBoundaries,
    ContractShape,
    LinkedTest,
    Scenario,
}

impl CheckKind {
    pub fn label(self) -> &'static str {
        match self {
            CheckKind::DependencyBoundaries => "Dependency boundaries",
            CheckKind::ContractShape => "Contract shape",
            CheckKind::LinkedTest => "Linked test",
            CheckKind::Scenario => "Scenario against the implementation",
        }
    }

    /// What the check covers, and what it does not.
    pub fn coverage(self) -> &'static str {
        match self {
            CheckKind::DependencyBoundaries => {
                "Which linked modules or crates refer to which, against the model's dependencies. \
                 It does not see references through macros or unlinked modules, and it says \
                 nothing about behaviour at run time."
            }
            CheckKind::ContractShape => {
                "Field and variant names and simple types of the linked Rust type against the item \
                 or enum def. It does not check values, invariants or behaviour."
            }
            CheckKind::LinkedTest => {
                "Whether the linked test passes. It covers what the test exercises and nothing else."
            }
            CheckKind::Scenario => {
                "The scenario's steps and checks through the harness, with the named dependencies \
                 replaced by stand-ins. Checks that read internal state are not evaluated."
            }
        }
    }
}

/// One implementation check's result.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImplementationCheck {
    pub kind: CheckKind,
    pub name: String,
    /// The model elements it covers: where drift is shown when it fails.
    pub elements: Vec<u64>,
    pub verdict: Verdict,
    pub message: String,
    #[serde(default)]
    pub details: Vec<String>,
    /// A code location (`src/model.rs#ShortLink`).
    #[serde(default)]
    pub location: Option<String>,
}

impl ImplementationCheck {
    fn new(kind: CheckKind, name: impl Into<String>, elements: Vec<ElementId>) -> Self {
        ImplementationCheck {
            kind,
            name: name.into(),
            elements: elements.into_iter().map(ElementId::raw).collect(),
            verdict: Verdict::NotRun,
            message: String::new(),
            details: Vec::new(),
            location: None,
        }
    }
}

/// A part def (or the type of a part usage) for an element.
fn owner_def(semantics: &Semantics, element: ElementId) -> ElementId {
    match semantics.element(element).map(|e| e.kind) {
        Some(ElementKind::Part) => semantics
            .types_of(element)
            .first()
            .map(|(t, _)| *t)
            .unwrap_or(element),
        _ => element,
    }
}

/// The model's allowed dependencies: client part def to supplier part defs.
fn model_dependencies(
    tree: &Tree,
    semantics: &Semantics,
) -> BTreeMap<ElementId, BTreeSet<ElementId>> {
    let mut out: BTreeMap<ElementId, BTreeSet<ElementId>> = BTreeMap::new();
    for id in tree.walk() {
        let element = &tree[id];
        if element.kind != ElementKind::Dependency || element.ends.len() != 2 {
            continue;
        }
        let end = |i: usize| {
            semantics
                .steps(id, Role::End, &element.ends[i])
                .ok()
                .and_then(|s| s.last().copied())
                .map(|e| owner_def(semantics, e))
        };
        if let (Some(client), Some(supplier)) = (end(0), end(1)) {
            out.entry(client).or_default().insert(supplier);
        }
    }
    out
}

/// A module's name from its path: `src/api.rs` → `api`,
/// `src/store/mod.rs` → `store`.
pub fn module_name(path: &str) -> Option<String> {
    let path = path.trim_end_matches(".rs");
    let mut parts: Vec<&str> = path.split('/').collect();
    if parts.last() == Some(&"mod") {
        parts.pop();
    }
    parts
        .last()
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
}

/// Linked modules against the model's dependencies. `read` gives a file's
/// text from the repository.
pub fn module_boundaries(
    tree: &Tree,
    links: &Links,
    read: &dyn Fn(&str) -> Option<String>,
) -> ImplementationCheck {
    let semantics = Semantics::new(tree);
    let mut modules: BTreeMap<String, (String, BTreeSet<ElementId>)> = BTreeMap::new();
    let mut shared: BTreeSet<String> = BTreeSet::new();
    for link in &links.links {
        let Some(module) = module_name(&link.path) else {
            continue;
        };
        match link.kind {
            LinkKind::Module if tree.contains(link.element()) => {
                let entry = modules
                    .entry(module)
                    .or_insert_with(|| (link.path.clone(), BTreeSet::new()));
                entry.1.insert(owner_def(&semantics, link.element()));
            }
            LinkKind::Type | LinkKind::Schema => {
                shared.insert(module);
            }
            _ => {}
        }
    }
    let elements: Vec<ElementId> = modules
        .values()
        .flat_map(|(_, e)| e.iter().copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut check = ImplementationCheck::new(
        CheckKind::DependencyBoundaries,
        "Dependency boundaries",
        elements,
    );
    if modules.is_empty() {
        check.verdict = Verdict::NotRun;
        check.message = "no modules are linked to parts, so there is nothing to compare".into();
        return check;
    }
    let dependencies = model_dependencies(tree, &semantics);
    let mut violations = Vec::new();
    let mut failing = BTreeSet::new();
    let mut unread = Vec::new();
    for (module, (path, owners)) in &modules {
        let Some(source) = read(path) else {
            unread.push(path.clone());
            continue;
        };
        for reference in rust::crate_references(&source) {
            if &reference == module || shared.contains(&reference) {
                continue;
            }
            let Some((_, suppliers)) = modules.get(&reference) else {
                continue; // not linked: not covered
            };
            let allowed = owners.iter().any(|owner| {
                suppliers.contains(owner)
                    || suppliers
                        .iter()
                        .any(|s| dependencies.get(owner).is_some_and(|d| d.contains(s)))
            });
            if !allowed {
                let names = |ids: &BTreeSet<ElementId>| {
                    ids.iter()
                        .map(|i| semantics.qualified_name(*i))
                        .collect::<Vec<_>>()
                        .join(", ")
                };
                violations.push(format!(
                    "`{path}` ({}) uses `crate::{reference}` ({}), but the model has no dependency from {} to {}",
                    names(owners),
                    names(suppliers),
                    names(owners),
                    names(suppliers)
                ));
                failing.extend(owners.iter().copied());
            }
        }
    }
    if !unread.is_empty() {
        check
            .details
            .push(format!("could not read {}", unread.join(", ")));
    }
    if violations.is_empty() {
        check.verdict = if unread.is_empty() {
            Verdict::Passed
        } else {
            Verdict::Inconclusive
        };
        check.message = format!(
            "{} linked module(s) refer only to what the model allows",
            modules.len()
        );
    } else {
        check.verdict = Verdict::Failed;
        check.message = format!("{} reference(s) the model does not allow", violations.len());
        check.elements = failing.into_iter().map(ElementId::raw).collect();
        check.details = violations;
    }
    check
}

/// The crates of a Cargo workspace against a model that maps crates with
/// usages typed by `crate_def` (the self-model's `part 'agq-x' : Crate;`)
/// and lists allowed dependencies between the parts that hold them. The
/// dogfood check: it runs on Agentique itself. `metadata` is the output of
/// `cargo metadata --format-version 1 --no-deps`.
pub fn crate_boundaries(
    tree: &Tree,
    crate_def: ElementId,
    metadata: &serde_json::Value,
) -> ImplementationCheck {
    let semantics = Semantics::new(tree);
    // Crate name → the part def that holds it.
    let mut part_of: BTreeMap<String, ElementId> = BTreeMap::new();
    for id in tree.walk() {
        let element = &tree[id];
        if element.kind != ElementKind::Part {
            continue;
        }
        let typed_by_crate = semantics.types_of(id).iter().any(|(t, _)| *t == crate_def);
        if let (true, Some(name), Some(owner)) = (typed_by_crate, &element.name, element.owner()) {
            part_of.insert(name.clone(), owner);
        }
    }
    let parts: Vec<ElementId> = part_of
        .values()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut check =
        ImplementationCheck::new(CheckKind::DependencyBoundaries, "Crate dependencies", parts);
    let dependencies = model_dependencies(tree, &semantics);
    let packages = metadata["packages"].as_array().cloned().unwrap_or_default();
    let workspace: BTreeSet<String> = packages
        .iter()
        .filter_map(|p| p["name"].as_str().map(str::to_string))
        .collect();
    let mut problems = Vec::new();
    let mut failing = BTreeSet::new();
    for (name, part) in &part_of {
        if !workspace.contains(name) {
            problems.push(format!(
                "`{name}` is mapped to {} but is not a crate of the workspace",
                semantics.qualified_name(*part)
            ));
            failing.insert(*part);
        }
    }
    for package in &packages {
        let Some(name) = package["name"].as_str() else {
            continue;
        };
        let Some(part) = part_of.get(name) else {
            problems.push(format!("`{name}` is not mapped to any part of the model"));
            continue;
        };
        for dependency in package["dependencies"]
            .as_array()
            .cloned()
            .unwrap_or_default()
        {
            if dependency["kind"].as_str() == Some("dev") {
                continue;
            }
            let Some(used) = dependency["name"].as_str() else {
                continue;
            };
            let Some(supplier) = part_of.get(used) else {
                continue;
            };
            let allowed =
                supplier == part || dependencies.get(part).is_some_and(|d| d.contains(supplier));
            if !allowed {
                problems.push(format!(
                    "`{name}` ({}) depends on `{used}` ({}), but the model has no dependency from {} to {}",
                    semantics.name(*part).unwrap_or("?"),
                    semantics.name(*supplier).unwrap_or("?"),
                    semantics.name(*part).unwrap_or("?"),
                    semantics.name(*supplier).unwrap_or("?")
                ));
                failing.insert(*part);
            }
        }
    }
    if problems.is_empty() {
        check.verdict = Verdict::Passed;
        check.message = format!(
            "{} crates in {} parts follow the model's dependencies",
            workspace.len(),
            part_of.values().collect::<BTreeSet<_>>().len()
        );
    } else {
        check.verdict = Verdict::Failed;
        check.message = format!("{} problem(s)", problems.len());
        check.details = problems;
        if !failing.is_empty() {
            check.elements = failing.into_iter().map(ElementId::raw).collect();
        }
    }
    check
}

/// Each linked Rust type against the item or enum def it implements.
pub fn contract_shapes(
    tree: &Tree,
    links: &Links,
    read: &dyn Fn(&str) -> Option<String>,
) -> Vec<ImplementationCheck> {
    let semantics = Semantics::new(tree);
    // Model type → the Rust type name that implements it.
    let rust_names: BTreeMap<ElementId, String> = links
        .links
        .iter()
        .filter(|l| l.kind == LinkKind::Type)
        .filter_map(|l| l.symbol.clone().map(|s| (l.element(), s)))
        .collect();
    let mut out = Vec::new();
    for link in links.links.iter().filter(|l| l.kind == LinkKind::Type) {
        let element = link.element();
        let name = format!(
            "Contract of {}",
            link.name.rsplit("::").next().unwrap_or(&link.name)
        );
        let mut check = ImplementationCheck::new(CheckKind::ContractShape, name, vec![element]);
        check.location = Some(link.location());
        let Some(model) = semantics.element(element) else {
            check.verdict = Verdict::Failed;
            check.message = format!("the linked element `{}` no longer exists", link.name);
            out.push(check);
            continue;
        };
        let Some(symbol) = &link.symbol else {
            check.verdict = Verdict::Unsupported;
            check.message = "the link names no Rust type (`symbol`)".into();
            out.push(check);
            continue;
        };
        let Some(source) = read(&link.path) else {
            check.verdict = Verdict::Failed;
            check.message = format!("`{}` cannot be read", link.path);
            out.push(check);
            continue;
        };
        let Some(rust_type) = rust::types(&source).into_iter().find(|t| &t.name == symbol) else {
            check.verdict = Verdict::Failed;
            check.message = format!("`{}` declares no struct or enum `{symbol}`", link.path);
            out.push(check);
            continue;
        };
        let mut differences = Vec::new();
        match (model.kind, &rust_type.kind) {
            (ElementKind::EnumDef, TypeKind::Enum) => {
                let expected: Vec<String> = semantics
                    .features(element)
                    .into_iter()
                    .filter(|f| {
                        semantics
                            .element(*f)
                            .is_some_and(|e| e.kind == ElementKind::Enum)
                    })
                    .filter_map(|f| semantics.name(f).map(rust::pascal_case))
                    .collect();
                for value in &expected {
                    if !rust_type.variants.contains(value) {
                        differences.push(format!("the model's value `{value}` has no variant"));
                    }
                }
                for variant in &rust_type.variants {
                    if !expected.contains(variant) {
                        differences.push(format!(
                            "the variant `{variant}` is not a value of the model"
                        ));
                    }
                }
            }
            (
                ElementKind::ItemDef | ElementKind::PartDef | ElementKind::AttributeDef,
                TypeKind::Struct,
            ) => {
                for feature in semantics.features(element) {
                    let Some(e) = semantics.element(feature) else {
                        continue;
                    };
                    if !matches!(
                        e.kind,
                        ElementKind::Attribute | ElementKind::Item | ElementKind::Reference
                    ) {
                        continue;
                    }
                    let model_name = semantics.name(feature).unwrap_or("?");
                    let field = rust::snake_case(model_name);
                    let Some((_, rust_ty)) = rust_type.fields.iter().find(|(n, _)| *n == field)
                    else {
                        differences
                            .push(format!("the model's `{model_name}` has no field `{field}`"));
                        continue;
                    };
                    let optional = e.multiplicity.is_some_and(|m| m.lower == 0);
                    let ty = semantics.types_of(feature).first().map(|(t, _)| *t);
                    if let Err(why) = type_fits(&semantics, ty, optional, rust_ty, &rust_names) {
                        differences.push(format!("`{field}: {rust_ty}` {why}"));
                    }
                }
            }
            (kind, found) => differences.push(format!(
                "a {} is implemented by a {}",
                kind.keyword(),
                if *found == TypeKind::Enum {
                    "enum"
                } else {
                    "struct"
                }
            )),
        }
        if differences.is_empty() {
            check.verdict = Verdict::Passed;
            check.message = format!("`{symbol}` matches `{}`", link.name);
        } else {
            check.verdict = Verdict::Failed;
            check.message = format!(
                "`{symbol}` differs from `{}` in {} way(s)",
                link.name,
                differences.len()
            );
            check.details = differences;
        }
        out.push(check);
    }
    out
}

/// Whether a Rust type as written can hold a feature's values.
fn type_fits(
    semantics: &Semantics,
    ty: Option<ElementId>,
    optional: bool,
    rust: &str,
    rust_names: &BTreeMap<ElementId, String>,
) -> Result<(), String> {
    let inner = match rust
        .strip_prefix("Option<")
        .and_then(|r| r.strip_suffix('>'))
    {
        Some(inner) if optional => inner,
        Some(_) => return Err("is optional in the code, but the model requires it".into()),
        None => rust,
    };
    let Some(ty) = ty else {
        return Ok(());
    };
    let is = |name: &str| {
        semantics
            .resolve(&format!("ScalarValues::{name}"))
            .is_some_and(|t| semantics.specializes(ty, t))
    };
    let integers = [
        "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
    ];
    let fits = if is("Boolean") {
        inner == "bool"
    } else if is("String") {
        matches!(
            inner,
            "String" | "&str" | "Box<str>" | "Arc<str>" | "Rc<str>"
        )
    } else if is("Integer") {
        integers.contains(&inner)
    } else if is("Real") {
        matches!(inner, "f32" | "f64")
    } else {
        let expected = rust_names
            .get(&ty)
            .cloned()
            .or_else(|| semantics.name(ty).map(str::to_string));
        expected.is_some_and(|e| inner == e || inner.ends_with(&format!("::{e}")))
    };
    if fits {
        Ok(())
    } else {
        Err(format!(
            "cannot hold a `{}`",
            semantics.name(ty).unwrap_or("?")
        ))
    }
}

/// Runs the linked tests (`cargo test`) and reports each.
pub fn linked_tests(
    links: &Links,
    executor: &Executor,
    folder: &str,
    timeout: Duration,
) -> Vec<ImplementationCheck> {
    let tests: Vec<_> = links
        .links
        .iter()
        .filter(|l| l.kind == LinkKind::Test && l.symbol.is_some())
        .collect();
    if tests.is_empty() {
        return Vec::new();
    }
    let names: Vec<String> = tests
        .iter()
        .filter_map(|l| l.symbol.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut args = vec!["test", "--offline", "--no-fail-fast", "--"];
    args.extend(names.iter().map(String::as_str));
    let finished = executor.run(&Program::cargo(&args), folder, timeout);
    let mut out = Vec::new();
    for link in tests {
        let symbol = link.symbol.clone().expect("filtered");
        let mut check = ImplementationCheck::new(
            CheckKind::LinkedTest,
            format!("Test {symbol}"),
            vec![link.element()],
        );
        check.location = Some(link.location());
        match &finished {
            Err(refusal) => {
                check.verdict = Verdict::NotRun;
                check.message = refusal.to_string();
            }
            Ok(run) if run.cancelled => {
                check.verdict = Verdict::NotRun;
                check.message = "cancelled".into();
            }
            Ok(run) => {
                let line = run
                    .stdout
                    .lines()
                    .find(|l| l.starts_with("test ") && l.contains(&format!("{symbol} ...")));
                match line {
                    Some(line) if line.ends_with(" ok") => {
                        check.verdict = Verdict::Passed;
                        check.message = "the test passed".into();
                    }
                    Some(line) if line.ends_with("FAILED") => {
                        check.verdict = Verdict::Failed;
                        check.message = "the test failed".into();
                        check.details = failure_of(&run.stdout, &symbol);
                    }
                    Some(line) => {
                        check.verdict = Verdict::NotRun;
                        check.message = format!(
                            "the test did not run: {}",
                            line.rsplit(" ... ").next().unwrap_or("")
                        );
                    }
                    None if !run.success && !run.stdout.contains("running ") => {
                        check.verdict = Verdict::Failed;
                        check.message = "the tests did not build".into();
                        check.details = vec![agq_execution::process::last_lines(&run.stderr, 20)];
                    }
                    None => {
                        check.verdict = Verdict::Failed;
                        check.message = format!("no test called `{symbol}` ran");
                    }
                }
            }
        }
        out.push(check);
    }
    out
}

/// The failure output `cargo test` printed for one test.
fn failure_of(stdout: &str, symbol: &str) -> Vec<String> {
    let marker = "---- ";
    let mut lines = Vec::new();
    let mut inside = false;
    for line in stdout.lines() {
        if line.starts_with(marker) {
            inside = line.contains(symbol);
            continue;
        }
        if inside {
            if line.trim().is_empty() || line.starts_with("failures:") {
                if !lines.is_empty() {
                    break;
                }
                continue;
            }
            lines.push(line.to_string());
            if lines.len() >= 12 {
                break;
            }
        }
    }
    lines
}

/// Failing checks by the element they cover: drift.
pub fn drift(checks: &[ImplementationCheck]) -> BTreeMap<ElementId, Vec<&ImplementationCheck>> {
    let mut out: BTreeMap<ElementId, Vec<&ImplementationCheck>> = BTreeMap::new();
    for check in checks.iter().filter(|c| c.verdict == Verdict::Failed) {
        for element in &check.elements {
            out.entry(ElementId::from_raw(*element))
                .or_default()
                .push(check);
        }
    }
    out
}
