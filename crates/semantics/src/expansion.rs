//! Explicit compact semantic expansion for supported constructs; SysML 8.4.7 and 8.4.14.
use agq_model::*;
fn node(m: &mut Model, parent: &Element, role: &str, kind: &str, name: Option<String>) -> Element {
    let mut e = parent.clone();
    e.id = derived_id(&parent.id, role);
    e.kind = kind.into();
    e.name = name;
    e.short_name = None;
    e.owner = Some(parent.id.clone());
    e.qualified_name = format!("{}::@{role}", parent.qualified_name);
    e.is_implied = true;
    e.references.clear();
    e.modifiers.clear();
    e.documentation.clear();
    e.unsupported.clear();
    e.guard = None;
    e.value = None;
    e.name_span = None;
    e.body_end = None;
    e.multiplicity = None;
    m.elements.insert(e.id.clone(), e.clone());
    e
}
fn edge(m: &mut Model, kind: &str, a: &str, b: &str, rule: &str) {
    super::relation(m, kind, a, b, true, rule)
}
fn lib(m: &mut Model, e: &Element, kind: &str, path: &str, rule: &str) {
    super::implicit(m, e, kind, path, rule)
}
fn connector_ends(
    m: &mut Model,
    connector: &Element,
    source: &str,
    target: &str,
    base_ends: [&str; 2],
) {
    for (index, (role, target)) in [("source", source), ("target", target)]
        .into_iter()
        .enumerate()
    {
        let end = node(m, connector, role, "ReferenceUsage", None);
        edge(
            m,
            "EndFeatureMembership",
            &connector.id,
            &end.id,
            "KerML 8.4.4.6 connector ends",
        );
        edge(
            m,
            "ReferenceSubsetting",
            &end.id,
            target,
            "KerML 8.4.4.6 relatedFeature",
        );
        lib(
            m,
            &end,
            "Redefinition",
            base_ends[index],
            "KerML 8.4.4.6 positional end redefinition",
        );
        if let Some(ty) = m
            .elements
            .get(target)
            .and_then(|t| t.target("type"))
            .map(str::to_string)
        {
            edge(
                m,
                "FeatureTyping",
                &end.id,
                &ty,
                "Inherited typing through reference subsetting",
            );
        }
    }
}
fn binding_ends(m: &mut Model, binding: &Element, source: &str, target: &str) {
    lib(
        m,
        binding,
        "Subsetting",
        "Links::selfLinks",
        "checkBindingConnectorSpecialization",
    );
    connector_ends(
        m,
        binding,
        source,
        target,
        ["Links::SelfLink::thisThing", "Links::SelfLink::sameThing"],
    );
}
pub fn expand(m: &mut Model, work: &WorkControl) {
    let authored: Vec<_> = m
        .elements
        .values()
        .filter(|e| !e.library && !e.is_implied)
        .cloned()
        .collect();
    for (index, e) in authored.iter().enumerate() {
        if let Err(error) = work.update("expanding", index, authored.len()) {
            m.diagnostics
                .push(Diagnostic::error(&error.code, error.message, None));
            return;
        }
        if e.kind == "ConnectionUsage" {
            lib(
                m,
                e,
                "Subsetting",
                "Connections::binaryConnections",
                "checkConnectionUsageBinarySpecialization",
            );
            if let (Some(source), Some(target)) = (e.target("source"), e.target("target")) {
                connector_ends(
                    m,
                    e,
                    source,
                    target,
                    [
                        "Connections::BinaryConnection::source",
                        "Connections::BinaryConnection::target",
                    ],
                );
            }
        }
        if e.kind == "EntryActionUsage" {
            lib(
                m,
                e,
                "Redefinition",
                "States::StateAction::entryAction",
                "checkActionUsageStateActionRedefinition",
            );
            if let Some(o) = &e.owner {
                edge(
                    m,
                    "StateSubactionMembership",
                    o,
                    &e.id,
                    "State entry action membership",
                );
            }
        }
        if e.kind == "PortDefinition" {
            let conjugate = node(m, e, "conjugated", "ConjugatedPortDefinition", None);
            edge(
                m,
                "PortConjugation",
                &conjugate.id,
                &e.id,
                "SysML 8.3.11 ConjugatedPortDefinition; 8.4.7",
            );
            edge(
                m,
                "OwningMembership",
                &e.id,
                &conjugate.id,
                "Implicit conjugated definition",
            );
            for feature in authored
                .iter()
                .filter(|f| f.owner.as_deref() == Some(&e.id))
            {
                if !["ItemUsage", "AttributeUsage", "PortUsage"].contains(&feature.kind.as_str()) {
                    continue;
                }
                let mut inverse = node(
                    m,
                    &conjugate,
                    &feature.id,
                    &feature.kind,
                    feature.name.clone(),
                );
                inverse.references = feature.references.clone();
                inverse.modifiers = feature
                    .modifiers
                    .iter()
                    .map(|d| match d.as_str() {
                        "in" => "out".into(),
                        "out" => "in".into(),
                        _ => d.clone(),
                    })
                    .collect();
                m.elements.insert(inverse.id.clone(), inverse.clone());
                edge(
                    m,
                    "FeatureMembership",
                    &conjugate.id,
                    &inverse.id,
                    "Conjugated feature direction",
                );
                edge(
                    m,
                    "Redefinition",
                    &inverse.id,
                    &feature.id,
                    "SysML 8.4.7 conjugation projection",
                );
            }
        }
        if e.kind == "PortUsage"
            && e.modifiers.contains(&"conjugated".into())
            && let Some(t) = e.target("type")
        {
            edge(
                m,
                "FeatureTyping",
                &e.id,
                &derived_id(t, "conjugated"),
                "Conjugated port typing (SysML 7.12)",
            );
        }
        if e.kind == "TransitionUsage" {
            if let Some(owner) = e.owner.as_ref().and_then(|o| m.elements.get(o))
                && ["StateDefinition", "StateUsage"].contains(&owner.kind.as_str())
            {
                lib(
                    m,
                    e,
                    "Subsetting",
                    "States::StateAction::stateTransitions",
                    "checkTransitionUsageStateSpecialization",
                );
            }
            let Some(source) = e.target("source") else {
                continue;
            };
            let Some(target) = e.target("target") else {
                continue;
            };
            let link_source = node(m, e, "transitionLinkSource", "ReferenceUsage", None);
            lib(
                m,
                &link_source,
                "Redefinition",
                "States::StateTransitionAction::transitionLinkSource",
                "checkFeatureParameterRedefinition",
            );
            edge(
                m,
                "ParameterMembership",
                &e.id,
                &link_source.id,
                "SysML 8.4.14.3",
            );
            edge(m, "Membership", &e.id, source, "Source alias (8.2.2.18.3)");
            let succession = node(m, e, "succession", "Succession", None);
            lib(
                m,
                &succession,
                "Subsetting",
                "Occurrences::happensBeforeLinks",
                "KerML succession specialization",
            );
            if let Some(o) = &e.owner {
                edge(
                    m,
                    "TypeFeaturing",
                    &succession.id,
                    o,
                    "checkConnectorTypeFeaturing",
                );
            }
            edge(
                m,
                "OwningMembership",
                &e.id,
                &succession.id,
                "Transition succession ownership",
            );
            connector_ends(
                m,
                &succession,
                source,
                target,
                [
                    "Occurrences::HappensBefore::earlierOccurrence",
                    "Occurrences::HappensBefore::laterOccurrence",
                ],
            );
            let binding = node(m, e, "sourceBinding", "BindingConnector", None);
            edge(
                m,
                "OwningMembership",
                &e.id,
                &binding.id,
                "checkTransitionUsageSourceBindingConnector",
            );
            binding_ends(m, &binding, source, &link_source.id);
            let succession_binding = node(m, e, "successionBinding", "BindingConnector", None);
            edge(
                m,
                "OwningMembership",
                &e.id,
                &succession_binding.id,
                "checkTransitionUsageSuccessionBindingConnector",
            );
            if let Some(target) = m
                .by_path("TransitionPerformances::TransitionPerformance::transitionLink")
                .map(|e| e.id.clone())
            {
                binding_ends(m, &succession_binding, &succession.id, &target);
            }
            if let Some(o) = &e.owner {
                edge(
                    m,
                    "TypeFeaturing",
                    &binding.id,
                    o,
                    "checkConnectorTypeFeaturing",
                );
                edge(
                    m,
                    "TypeFeaturing",
                    &succession_binding.id,
                    o,
                    "checkConnectorTypeFeaturing",
                );
            }
            if let Some(payload_type) = e.target("payload_type") {
                let accepter = node(m, e, "accepter", "AcceptActionUsage", None);
                edge(
                    m,
                    "TransitionFeatureMembership",
                    &e.id,
                    &accepter.id,
                    "triggerAction (8.4.14.3)",
                );
                lib(
                    m,
                    &accepter,
                    "Redefinition",
                    "Actions::TransitionAction::accepter",
                    "checkTransitionUsageTransitionFeatureSpecialization; trigger is not standalone acceptActions",
                );
                let payload = node(m, &accepter, "payload", "ReferenceUsage", None);
                edge(
                    m,
                    "ParameterMembership",
                    &accepter.id,
                    &payload.id,
                    "Accept payloadParameter",
                );
                edge(
                    m,
                    "FeatureTyping",
                    &payload.id,
                    payload_type,
                    "Typed accept payload",
                );
                let outer = authored
                    .iter()
                    .find(|p| p.owner.as_deref() == Some(&e.id) && p.kind == "PayloadParameter")
                    .cloned()
                    .unwrap_or_else(|| node(m, e, "payload", "ReferenceUsage", None));
                edge(
                    m,
                    "ParameterMembership",
                    &e.id,
                    &outer.id,
                    "Transition payload parameter",
                );
                edge(
                    m,
                    "Subsetting",
                    &outer.id,
                    &payload.id,
                    "checkTransitionUsagePayloadSpecialization",
                );
                lib(
                    m,
                    &outer,
                    "Redefinition",
                    "States::StateTransitionAction::payload",
                    "checkFeatureParameterRedefinition",
                );
                if let Some(receiver) = e.target("receiver") {
                    let parameter = node(m, &accepter, "receiver", "ReferenceUsage", None);
                    edge(
                        m,
                        "ParameterMembership",
                        &accepter.id,
                        &parameter.id,
                        "Accept receiver parameter",
                    );
                    let receiver_binding =
                        node(m, &accepter, "receiverBinding", "BindingConnector", None);
                    edge(
                        m,
                        "OwningMembership",
                        &accepter.id,
                        &receiver_binding.id,
                        "Receiver expression binding (8.4.13.6)",
                    );
                    binding_ends(m, &receiver_binding, &parameter.id, receiver);
                }
            }
            if e.guard.is_some() {
                let mut guard = node(m, e, "guard", "OperatorExpression", None);
                guard.value = e.guard.clone();
                m.elements.insert(guard.id.clone(), guard.clone());
                edge(
                    m,
                    "TransitionFeatureMembership",
                    &e.id,
                    &guard.id,
                    "guardExpression (8.4.14.3)",
                );
                lib(
                    m,
                    &guard,
                    "FeatureTyping",
                    "ScalarValues::Boolean",
                    "Boolean guard result",
                );
                lib(
                    m,
                    &guard,
                    "Redefinition",
                    "TransitionPerformances::TransitionPerformance::guard",
                    "checkTransitionUsageTransitionFeatureSpecialization; inherited guard",
                );
            }
        }
    }
}
