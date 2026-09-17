//! Scoped reference resolution and supported well-formedness constraints.
use agq_model::*;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;
include!(concat!(env!("OUT_DIR"), "/libraries.rs"));
mod expansion;

fn libraries() -> &'static Model {
    static LIBRARIES: OnceLock<Model> = OnceLock::new();
    LIBRARIES.get_or_init(|| {
        let sources = LIBRARY_SOURCES
            .iter()
            .map(|(p, s)| (p.to_string(), s.to_string()))
            .collect();
        let mut model = agq_syntax::parse(sources, &BTreeMap::new());
        // Effective names of unnamed redefinitions are inherited (KerML Feature naming).
        // This is indexing original declarations, not manufacturing library definitions.
        for e in model.elements.values_mut() {
            if e.name.is_none()
                && let Some(r) = e.reference("redefinition")
            {
                let name = r.path.rsplit("::").next().unwrap().to_string();
                let prefix = e
                    .qualified_name
                    .rsplit_once("::")
                    .map(|(p, _)| p.to_string());
                e.qualified_name = prefix
                    .map(|p| format!("{p}::{name}"))
                    .unwrap_or_else(|| name.clone());
                e.name = Some(name);
            }
        }
        let ids: BTreeMap<_, _> = model
            .elements
            .values()
            .map(|e| {
                (
                    e.id.clone(),
                    derived_id(
                        "omg-20250201",
                        &format!("{}|{}", e.span.file, e.qualified_name),
                    ),
                )
            })
            .collect();
        model.elements = std::mem::take(&mut model.elements)
            .into_values()
            .map(|mut e| {
                e.id = ids[&e.id].clone();
                e.owner = e.owner.map(|o| ids[&o].clone());
                e.library = true;
                (e.id.clone(), e)
            })
            .collect();
        // Library bodies remain original source. This is a declaration index, not a claim of whole-library validation.
        model.diagnostics.clear();
        model
    })
}
pub fn compile(sources: BTreeMap<String, String>, identities: &BTreeMap<String, Id>) -> Model {
    compile_controlled(sources, identities, &WorkControl::default())
}
pub fn compile_controlled(
    sources: BTreeMap<String, String>,
    identities: &BTreeMap<String, Id>,
    work: &WorkControl,
) -> Model {
    let mut model = agq_syntax::parse_controlled(sources, identities, work);
    if model
        .diagnostics
        .iter()
        .any(|d| d.code == "cancelled" || d.code == "resource_limit")
    {
        return model;
    }
    let library = libraries();
    model.elements.extend(library.elements.clone());
    model.library_digest = digest(LIBRARY_LOCK);
    validate_controlled(&mut model, work);
    if let Err(error) = work.check() {
        model
            .diagnostics
            .push(Diagnostic::error(&error.code, error.message, None));
        return model;
    }
    expansion::expand(&mut model, work);
    model
}
pub fn identity_map(model: &Model) -> BTreeMap<String, Id> {
    model
        .elements
        .values()
        .filter(|e| !e.library && !e.is_implied)
        .map(|e| {
            (
                format!("{}|{}", e.span.file, e.qualified_name),
                e.id.clone(),
            )
        })
        .collect()
}
pub fn library_source(file: &str) -> Option<&'static str> {
    LIBRARY_SOURCES
        .iter()
        .find(|(p, _)| *p == file)
        .map(|(_, s)| *s)
}
/// Scalar identity is resolved against pinned declarations, never inferred from a user name.
pub fn scalar_name(element: &Element) -> Option<&str> {
    if !element.library {
        return None;
    }
    match element.qualified_name.as_str() {
        "ScalarValues::Boolean" => Some("Boolean"),
        "ScalarValues::Integer" => Some("Integer"),
        "ScalarValues::String" => Some("String"),
        _ => None,
    }
}

fn names(path: &str) -> Vec<&str> {
    path.split("::").flat_map(|x| x.split('.')).collect()
}
fn direct<'a>(m: &'a Model, owner: Option<&str>, name: &str) -> Vec<&'a Element> {
    m.elements
        .values()
        .filter(|e| {
            e.owner.as_deref() == owner
                && (e.name.as_deref() == Some(name) || e.short_name.as_deref() == Some(name))
        })
        .collect()
}
fn descend(m: &Model, start: &str, parts: &[&str], seen: &mut BTreeSet<String>) -> Option<Vec<Id>> {
    if parts.is_empty() {
        return Some(vec![start.into()]);
    }
    if !seen.insert(format!("{start}/{parts:?}")) {
        return None;
    }
    let e = m.elements.get(start)?;
    let mut children = direct(m, Some(start), parts[0]);
    if children.is_empty()
        && let Some(type_ref) = e.reference("type")
    {
        let target = type_ref.target.clone().or_else(|| {
            resolve_inner(m, e.owner.as_deref(), &type_ref.path, seen)
                .ok()
                .and_then(|x| x.last().cloned())
        });
        if let Some(t) = target {
            children = direct(m, Some(&t), parts[0]);
        }
    }
    if children.len() != 1 {
        return None;
    }
    let mut result = vec![start.into()];
    result.extend(descend(m, &children[0].id, &parts[1..], seen)?);
    Some(result)
}
fn resolve_inner(
    m: &Model,
    scope: Option<&str>,
    path: &str,
    seen: &mut BTreeSet<String>,
) -> Result<Vec<Id>> {
    if seen.len() > 256 {
        return Err(Error::new(
            "resource_limit",
            "Reference expansion exceeds 256 links",
        ));
    }
    let parts = names(path);
    if parts.is_empty() {
        return Err(Error::new("unresolved_reference", "Empty reference"));
    }
    let mut current = scope;
    loop {
        let local = direct(m, current, parts[0]);
        let mut matches = vec![];
        for e in &local {
            if let Some(r) = descend(m, &e.id, &parts[1..], &mut seen.clone()) {
                matches.push(r)
            }
        }
        if local.is_empty() {
            for import in m
                .elements
                .values()
                .filter(|e| e.owner.as_deref() == current && e.kind == "NamespaceImport")
            {
                if let Some(r) = import.reference("import") {
                    let marker = format!("import:{}:{path}", import.id);
                    if !seen.insert(marker) {
                        continue;
                    }
                    if let Ok(target) = resolve_inner(
                        m,
                        import
                            .owner
                            .as_deref()
                            .and_then(|o| m.elements.get(o))
                            .and_then(|e| e.owner.as_deref()),
                        &r.path,
                        seen,
                    ) {
                        let id = target.last().unwrap();
                        if import.modifiers.contains(&"wildcard".into()) {
                            for e in direct(m, Some(id), parts[0]) {
                                if let Some(r) = descend(m, &e.id, &parts[1..], &mut seen.clone()) {
                                    matches.push(r)
                                }
                            }
                        } else if m.elements[id].name.as_deref() == Some(parts[0])
                            && let Some(r) = descend(m, id, &parts[1..], &mut seen.clone())
                        {
                            matches.push(r)
                        }
                    }
                }
            }
        }
        matches.sort();
        matches.dedup();
        if matches.len() == 1 {
            return Ok(matches.remove(0));
        }
        if matches.len() > 1 {
            return Err(Error::new(
                "ambiguous_reference",
                format!("Ambiguous reference {path}"),
            ));
        }
        if current.is_none() {
            break;
        }
        current = current
            .and_then(|o| m.elements.get(o))
            .and_then(|e| e.owner.as_deref());
    }
    Err(Error::new(
        "unresolved_reference",
        format!("Cannot resolve {path}"),
    ))
}
pub fn resolve(m: &Model, scope: Option<&str>, path: &str) -> Result<Vec<Id>> {
    resolve_inner(m, scope, path, &mut BTreeSet::new())
}
pub fn locate(m: &Model, path_or_id: &str) -> Result<Vec<Id>> {
    if m.elements.contains_key(path_or_id) {
        Ok(vec![path_or_id.into()])
    } else {
        resolve(m, None, path_or_id)
    }
}
fn relation(m: &mut Model, kind: &str, source: &str, target: &str, implied: bool, rule: &str) {
    m.relationships.push(Relationship {
        id: derived_id(source, &format!("{kind}/{target}/{rule}")),
        kind: kind.into(),
        source: source.into(),
        target: target.into(),
        implied,
        rule: rule.into(),
    });
}
fn implicit(m: &mut Model, e: &Element, kind: &str, path: &str, rule: &str) {
    if let Some(t) = m.by_path(path) {
        let id = t.id.clone();
        relation(m, kind, &e.id, &id, true, rule)
    } else {
        m.diagnostics.push(
            Diagnostic::error(
                "library_dependency_missing",
                format!("Required official declaration {path} absent from library index"),
                Some(e.span.clone()),
            )
            .at(&e.id),
        );
    }
}
pub fn validate(m: &mut Model) {
    validate_controlled(m, &WorkControl::default())
}
fn validate_controlled(m: &mut Model, work: &WorkControl) {
    let ids: Vec<_> = m
        .elements
        .values()
        .filter(|e| !e.library)
        .map(|e| e.id.clone())
        .collect();
    let mut ownership_names = BTreeSet::new();
    for (index, id) in ids.iter().enumerate() {
        if let Err(e) = work.update("resolving", index, ids.len()) {
            m.diagnostics
                .push(Diagnostic::error(&e.code, e.message, None));
            return;
        }
        let e = m.elements[id].clone();
        if let Some(name) = &e.name
            && !ownership_names.insert((e.owner.clone(), name.clone()))
        {
            m.diagnostics.push(
                Diagnostic::error(
                    "invalid_model",
                    format!("Duplicate member {name}"),
                    Some(e.span.clone()),
                )
                .at(id),
            );
        }
        if let Some(owner) = &e.owner {
            relation(
                m,
                if e.definition() || e.kind == "Package" {
                    "OwningMembership"
                } else {
                    "FeatureMembership"
                },
                owner,
                id,
                true,
                "KerML 8.2.3; SysML 8.2",
            );
        }
        let mut refs = e.references.clone();
        for r in &mut refs {
            match resolve(
                m,
                if r.role == "expression" {
                    Some(id.as_str())
                } else {
                    e.owner.as_deref()
                },
                &r.path,
            ) {
                Ok(chain) => {
                    r.target = chain.last().cloned();
                    let text = m
                        .sources
                        .get(&r.span.file)
                        .map(|s| &s[r.span.start..r.span.end])
                        .unwrap_or("");
                    let (ts, _) = agq_syntax::lex(&r.span.file, text);
                    let nts: Vec<_> = ts
                        .into_iter()
                        .filter(|t| t.text != "::" && t.text != ".")
                        .collect();
                    r.segments = nts
                        .iter()
                        .zip(&chain)
                        .map(|(t, id)| {
                            let mut s = t.span.clone();
                            s.start += r.span.start;
                            s.end += r.span.start;
                            s.line = r.span.line;
                            (s, id.clone())
                        })
                        .collect();
                    if let Some(t) = &r.target {
                        relation(
                            m,
                            match r.role.as_str() {
                                "type" => "FeatureTyping",
                                "specialization" => {
                                    if e.definition() {
                                        "Subclassification"
                                    } else {
                                        "Subsetting"
                                    }
                                }
                                "import" => "NamespaceImport",
                                "redefinition" => "Redefinition",
                                // A locator/guard reference is not a KerML ReferenceSubsetting.
                                // ElementReference is explicitly an Agentique inspection edge.
                                _ => "ElementReference",
                            },
                            id,
                            t,
                            false,
                            "Explicit textual reference",
                        );
                    }
                }
                Err(err) => m
                    .diagnostics
                    .push(Diagnostic::error(&err.code, err.message, Some(r.span.clone())).at(id)),
            }
        }
        m.elements.get_mut(id).unwrap().references = refs;
    }
    for (index, id) in ids.iter().enumerate() {
        if let Err(e) = work.update("validating", index, ids.len()) {
            m.diagnostics
                .push(Diagnostic::error(&e.code, e.message, None));
            return;
        }
        let e = m.elements[id].clone();
        if !e.unsupported.is_empty() || e.kind == "Unsupported" {
            m.diagnostics.push(
                Diagnostic::error(
                    "unsupported_feature",
                    format!(
                        "Preserved without semantic interpretation: {}",
                        e.unsupported.join(" ")
                    ),
                    Some(e.span.clone()),
                )
                .at(id)
                .warning(),
            );
        }
        if let Some((min, Some(max))) = e.multiplicity
            && min > max
        {
            m.diagnostics.push(
                Diagnostic::error(
                    "invalid_model",
                    "Multiplicity lower bound exceeds upper bound",
                    Some(e.span.clone()),
                )
                .at(id),
            );
        }
        if let Some(t) = e.target("type") {
            let target = &m.elements[t];
            let expected = match e.kind.as_str() {
                "PartUsage" => Some("PartDefinition"),
                "ItemUsage" => Some("ItemDefinition"),
                "PortUsage" => Some("PortDefinition"),
                "StateUsage" | "ExhibitStateUsage" => Some("StateDefinition"),
                _ => None,
            };
            let compatible = expected.is_none_or(|kind| {
                target.kind == kind
                    || (kind == "ItemDefinition"
                        && ["PartDefinition", "PortDefinition"].contains(&target.kind.as_str()))
            });
            let attribute_type = e.kind != "AttributeUsage"
                || ["DataType", "AttributeDefinition"].contains(&target.kind.as_str());
            if !compatible || !attribute_type {
                m.diagnostics.push(
                    Diagnostic::error(
                        "invalid_model",
                        format!("{} cannot be typed by {}", e.kind, target.kind),
                        Some(e.span.clone()),
                    )
                    .at(id),
                );
            }
        }
        let base = match e.kind.as_str() {
            "PartDefinition" => Some(("Subclassification", "Parts::Part")),
            "ConnectionDefinition" => Some(("Subclassification", "Connections::Connection")),
            "AttributeDefinition" => Some(("Subclassification", "Base::DataValue")),
            "ItemDefinition" => Some(("Subclassification", "Items::Item")),
            "PortDefinition" => Some(("Subclassification", "Ports::Port")),
            "StateDefinition" => Some(("Subclassification", "States::StateAction")),
            "ActionDefinition" => Some(("Subclassification", "Actions::Action")),
            "RequirementDefinition" => {
                Some(("Subclassification", "Requirements::RequirementCheck"))
            }
            "VerificationCaseDefinition" => {
                Some(("Subclassification", "VerificationCases::VerificationCase"))
            }
            "ViewDefinition" => Some(("Subclassification", "Views::View")),
            "PartUsage" => Some(("Subsetting", "Parts::parts")),
            "ConnectionUsage" => Some(("Subsetting", "Connections::connections")),
            "ActionUsage" => Some(("Subsetting", "Actions::actions")),
            "ViewUsage" => Some(("Subsetting", "Views::views")),
            "VerificationCaseUsage" => Some(("Subsetting", "VerificationCases::verificationCases")),
            "ItemUsage" => Some(("Subsetting", "Items::items")),
            "PortUsage" => Some(("Subsetting", "Ports::ports")),
            "StateUsage" => Some(("Subsetting", "States::stateActions")),
            "ExhibitStateUsage" => Some(("Subsetting", "Parts::Part::exhibitedStates")),
            "AttributeUsage" => Some(("Subsetting", "Base::dataValues")),
            "RequirementUsage" => Some(("Subsetting", "Requirements::requirementChecks")),
            "TransitionUsage" => Some(("Subsetting", "Actions::transitionActions")),
            "EntryActionUsage" => Some(("Subsetting", "Actions::actions")),
            _ => None,
        };
        if let Some((kind, target)) = base {
            implicit(
                m,
                &e,
                kind,
                target,
                "SysML 8.4 tables 31–32 (specific rules in coverage)",
            );
        }
        if let Some(owner) = e.owner.as_ref().and_then(|o| m.elements.get(o)).cloned() {
            if e.kind == "StateUsage"
                && (owner.kind == "StateDefinition" || owner.kind == "StateUsage")
                && !owner.modifiers.contains(&"parallel".into())
            {
                implicit(
                    m,
                    &e,
                    "Subsetting",
                    "States::StateAction::exclusiveStates",
                    "checkStateUsageExclusiveStateSpecialization; 8.4.14.2",
                );
            }
            if e.kind == "PartUsage"
                && (owner.kind == "PartDefinition" || owner.kind == "PartUsage")
            {
                implicit(
                    m,
                    &e,
                    "Subsetting",
                    "Items::Item::subparts",
                    "checkPartUsageSubpartSpecialization",
                );
            }
            if e.kind == "PortUsage"
                && (owner.kind == "PartDefinition" || owner.kind == "PartUsage")
            {
                implicit(
                    m,
                    &e,
                    "Subsetting",
                    "Parts::Part::ownedPorts",
                    "checkPortUsageOwnedPortSpecialization",
                );
            }
        }
        if e.kind == "TransitionUsage" {
            for role in ["source", "target"] {
                if let Some(t) = e.target(role)
                    && m.elements[t].kind != "StateUsage"
                {
                    m.diagnostics.push(
                        Diagnostic::error(
                            "unsupported_feature",
                            format!("AGQ-SEQ-01 requires a state {role}"),
                            Some(e.span.clone()),
                        )
                        .at(id)
                        .warning(),
                    );
                }
            }
            if let Some(owner) = e.owner.as_ref().and_then(|o| m.elements.get(o))
                && owner.modifiers.contains(&"parallel".into())
            {
                m.diagnostics.push(
                    Diagnostic::error(
                        "invalid_model",
                        "Transitions between concurrent states are prohibited (SysML 7.18.3)",
                        Some(e.span.clone()),
                    )
                    .at(id),
                );
            }
        }
        if let Some(Expr::Literal { value }) = &e.value {
            if !value.valid() {
                m.diagnostics.push(
                    Diagnostic::error(
                        "evaluation_error",
                        "Invalid or oversized scalar literal",
                        Some(e.span.clone()),
                    )
                    .at(id),
                );
            }
            if let Some(t) = e.target("type") {
                let n = scalar_name(&m.elements[t]);
                if n.is_some_and(|n| n != value.scalar_type()) {
                    m.diagnostics.push(
                        Diagnostic::error(
                            "invalid_model",
                            "Attribute literal type mismatch",
                            Some(e.span.clone()),
                        )
                        .at(id),
                    );
                }
            }
        }
    }
    // Owned membership is constructed as a tree; explicit specialization cycles are invalid.
    fn cycle(m: &Model, id: &str, stack: &mut BTreeSet<String>) -> bool {
        if stack.len() > 128 {
            return true;
        }
        if !stack.insert(id.into()) {
            return true;
        }
        for r in &m.elements[id].references {
            if r.role == "specialization"
                && let Some(t) = &r.target
                && cycle(m, t, stack)
            {
                return true;
            }
        }
        stack.remove(id);
        false
    }
    for id in ids {
        if cycle(m, &id, &mut BTreeSet::new()) {
            m.diagnostics.push(
                Diagnostic::error(
                    "invalid_model",
                    "Cyclic specialization or specialization depth exceeds 128",
                    Some(m.elements[&id].span.clone()),
                )
                .at(&id),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn published_library_index_retains_declarations_after_unknown_bodies() {
        let (file, source) = LIBRARY_SOURCES
            .iter()
            .find(|(p, _)| p.ends_with("/Items.sysml"))
            .unwrap();
        let m = agq_syntax::parse(
            BTreeMap::from([(file.to_string(), source.to_string())]),
            &BTreeMap::new(),
        );
        assert!(
            m.by_path("Items::items").is_some(),
            "{:?}\n{:?}",
            m.diagnostics,
            m.elements
                .values()
                .map(|e| &e.qualified_name)
                .collect::<Vec<_>>()
        );
    }
    #[test]
    fn self_model_resolves() {
        let m = compile(
            BTreeMap::from([
                (
                    "behaviour.sysml".into(),
                    include_str!("../../../models/AgentiqueBehaviour.sysml").into(),
                ),
                (
                    "architecture.sysml".into(),
                    include_str!("../../../models/AgentiqueArchitecture.sysml").into(),
                ),
            ]),
            &BTreeMap::new(),
        );
        assert!(m.accepted(), "{:?}", m.diagnostics);
        let chain = locate(
            &m,
            "AgentiqueArchitecture::agentique.engine.models.requestLifecycle.input",
        )
        .unwrap();
        assert_eq!(m.elements[chain.last().unwrap()].kind, "PortUsage");
    }
    #[test]
    fn invalid_type_and_resolution() {
        let m = compile(
            BTreeMap::from([(
                "a.sysml".into(),
                "package P { item def X; part a : X; part b : Missing; }".into(),
            )]),
            &BTreeMap::new(),
        );
        assert!(m.diagnostics.iter().any(|d| d.code == "invalid_model"));
        assert!(
            m.diagnostics
                .iter()
                .any(|d| d.code == "unresolved_reference")
        );
    }
    #[test]
    fn required_implicit_relationships_have_real_targets() {
        let m=compile(BTreeMap::from([("a.sysml".into(),"package P { item def Message; port def Channel { in item signal : Message; } part def Device { port socket : ~Channel; } state def Cycle { port input; entry; then waiting; state waiting; state done; transition finish first waiting accept Message via input if true then done; } }".into())]),&BTreeMap::new());
        assert!(m.accepted(), "{:?}", m.diagnostics);
        for r in &m.relationships {
            assert!(
                m.elements.contains_key(&r.source),
                "missing source {}",
                r.rule
            );
            assert!(
                m.elements.contains_key(&r.target),
                "missing target {}",
                r.rule
            );
        }
        for element in m.elements.values().filter(|e| !e.library) {
            assert!(
                m.relationships
                    .iter()
                    .filter(|r| r.source == element.id && r.kind == "ReferenceSubsetting")
                    .count()
                    <= 1,
                "KerML validateFeatureOwnedReferenceSubsetting: {}",
                element.qualified_name
            );
            if ["BindingConnector", "Succession"].contains(&element.kind.as_str()) {
                assert_eq!(
                    m.relationships
                        .iter()
                        .filter(|r| r.source == element.id && r.kind == "EndFeatureMembership")
                        .count(),
                    2,
                    "Binary connector must own two ends"
                );
            }
        }
        for path in [
            "Base::dataValues",
            "Actions::transitionActions",
            "Actions::TransitionAction::accepter",
            "States::StateTransitionAction::payload",
            "States::StateAction::entryAction",
            "Occurrences::happensBeforeLinks",
        ] {
            assert!(m.by_path(path).is_some(), "{path}");
        }
        let channel = m.by_path("P::Channel").unwrap();
        let inverse = m.elements[&derived_id(&channel.id, "conjugated")].clone();
        assert_eq!(inverse.kind, "ConjugatedPortDefinition");
        assert!(m.children(&inverse.id).any(|e|e.name.as_deref()==Some("signal")&&e.modifiers.contains(&"out".into())));
        let rebuilt = compile(m.sources.clone(), &identity_map(&m));
        assert_eq!(
            m.elements.keys().collect::<Vec<_>>(),
            rebuilt.elements.keys().collect::<Vec<_>>()
        );
    }
}
