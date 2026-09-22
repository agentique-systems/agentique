//! Final SysML transition payload structure and ReferenceUsage naming dispatch.
use crate::{SysmlBaselineProfile, SysmlElementProposal, SysmlProducerResult};
use agq_kerml::{classes as kc, properties as kp};
use agq_kerml_semantics::{Completeness, Diagnostic, KerMlQueries, QueryResult, SearchDependency};
use agq_kernel::{
    DerivationKey, ElementId, MetaclassId, PropertyId,
    derived::PropertyState,
    provenance::FactKey,
    value::{SlotValue, Value},
};
use agq_sysml::{classes as sc, properties as sp};
use std::collections::BTreeMap;

fn is(q: &KerMlQueries<'_>, subject: ElementId, class: MetaclassId) -> bool {
    q.model().element(subject).is_some_and(|r| {
        q.model()
            .registry()
            .is_subtype(r.metaclass(), class)
            .unwrap_or(false)
    })
}
fn merge<T, U>(out: &mut QueryResult<T>, other: QueryResult<U>) {
    out.merge_evidence(other).expect("same transition context");
}
fn pending<T>(out: &mut QueryResult<T>, subject: ElementId, message: &str) {
    out.completeness = out.completeness.max(Completeness::Incomplete);
    out.diagnostics.insert(Diagnostic {
        code: "SQ_TRANSITION_PAYLOAD_PENDING",
        subject,
        message: message.into(),
    });
}
fn enumeration<T>(
    q: &KerMlQueries<'_>,
    out: &mut QueryResult<T>,
    subject: ElementId,
    property: PropertyId,
) -> Option<String> {
    merge(
        out,
        q.canonical_fact_evidence(FactKey::Property {
            element: subject,
            property,
        }),
    );
    let Ok(PropertyState::Computed(slot)) = q.model().property_state(subject, property) else {
        return None;
    };
    let SlotValue::Scalar(Value::Enumeration(literal)) = slot.value() else {
        return None;
    };
    let agq_kernel::metamodel::ValueKind::Enumeration(domain) =
        q.model().registry().property(property).ok()?.value_kind
    else {
        return None;
    };
    q.model()
        .registry()
        .enumeration(domain)
        .ok()?
        .literals
        .get(literal)
        .cloned()
}
fn input_parameters(q: &KerMlQueries<'_>, subject: ElementId) -> QueryResult<Vec<ElementId>> {
    let parameters = q.structural_parameter_features(subject);
    let ids = parameters.value.clone();
    let mut out = parameters.map(|_| vec![]);
    for parameter in ids {
        if matches!(
            enumeration(q, &mut out, parameter, kp::FEATURE_DIRECTION).as_deref(),
            Some("in" | "inout")
        ) {
            out.value.push(parameter);
        }
    }
    out
}
fn trigger_payload(
    q: &KerMlQueries<'_>,
    transition: ElementId,
) -> QueryResult<Option<(ElementId, ElementId)>> {
    let owned = q.owned_relationships_of_type(transition, sc::TRANSITION_FEATURE_MEMBERSHIP);
    let relationships = owned.value.clone();
    let mut out = owned.map(|_| None);
    for membership in relationships {
        merge(
            &mut out,
            q.canonical_fact_evidence(FactKey::Element(membership)),
        );
        if !is(q, membership, sc::TRANSITION_FEATURE_MEMBERSHIP) {
            continue;
        }
        if enumeration(
            q,
            &mut out,
            membership,
            sp::TRANSITION_FEATURE_MEMBERSHIP_KIND,
        )
        .as_deref()
            != Some("trigger")
        {
            continue;
        }
        let member = q.member(membership);
        let trigger = member.value;
        merge(&mut out, member);
        let Some(trigger) = trigger else {
            pending(
                &mut out,
                membership,
                "Trigger membership endpoint is pending",
            );
            return out;
        };
        merge(
            &mut out,
            q.canonical_fact_evidence(FactKey::Element(trigger)),
        );
        if !is(q, trigger, sc::ACCEPT_ACTION_USAGE) {
            continue;
        }
        let parameters = q.structural_parameter_features(trigger);
        let payload = parameters.value.first().copied();
        merge(&mut out, parameters);
        if let Some(payload) = payload {
            out.value = Some((trigger, payload));
        } else {
            pending(
                &mut out,
                trigger,
                "First trigger action payload parameter is pending",
            );
        }
        return out;
    }
    out
}

fn reference_subsetting(
    q: &KerMlQueries<'_>,
    subject: ElementId,
) -> QueryResult<Option<Option<ElementId>>> {
    let owned = q.owned_relationships_of_type(subject, kc::REFERENCE_SUBSETTING);
    let relationships = owned.value.clone();
    let mut out = owned.map(|_| None);
    for relationship in relationships {
        merge(
            &mut out,
            q.canonical_fact_evidence(FactKey::Element(relationship)),
        );
        if !is(q, relationship, kc::REFERENCE_SUBSETTING) {
            continue;
        }
        if out.value.is_some() {
            pending(
                &mut out,
                subject,
                "Multiple owned ReferenceSubsettings prevent a unique naming source",
            );
            out.value = Some(None);
            return out;
        }
        merge(
            &mut out,
            q.canonical_fact_evidence(FactKey::Property {
                element: relationship,
                property: kp::REFERENCE_SUBSETTING_REFERENCED_FEATURE,
            }),
        );
        let target = q
            .model()
            .navigation_slot(relationship, kp::REFERENCE_SUBSETTING_REFERENCED_FEATURE)
            .and_then(|slot| match slot.value() {
                SlotValue::Scalar(Value::Reference(target)) => Some(*target),
                _ => None,
            });
        if target.is_none() {
            pending(
                &mut out,
                relationship,
                "Owned ReferenceSubsetting target is pending",
            );
        }
        out.value = Some(target);
    }
    out
}

/// Published SysML naming overrides, including the second transition input's
/// payload source. The outer option distinguishes fallback from an applicable null.
/// Even fallback retains all inspected ownership and parameter search evidence.
pub fn current_sysml_naming_source(
    q: &KerMlQueries<'_>,
    subject: ElementId,
) -> QueryResult<Option<Option<ElementId>>> {
    let mut out = q
        .canonical_fact_evidence(FactKey::Element(subject))
        .map(|_| None);
    if !is(q, subject, sc::USAGE) {
        return out;
    }
    if is(q, subject, sc::PERFORM_ACTION_USAGE) {
        out.search_dependencies
            .insert(SearchDependency::ValidationRule(
                "PerformActionUsage::namingFeature",
            ));
        let reference = reference_subsetting(q, subject);
        let referenced = reference.value;
        merge(&mut out, reference);
        if let Some(referenced) = referenced {
            let performed = referenced.and_then(|referenced| {
                let target = q.feature_target(referenced);
                let performed = target
                    .value
                    .filter(|&feature| is(q, feature, sc::OCCURRENCE_USAGE));
                merge(&mut out, target);
                performed
            });
            if performed != Some(subject) {
                out.value = Some(performed);
                return out;
            }
        }
    }
    let membership = q.owning_relationship(subject);
    let owner_membership = membership.value;
    merge(&mut out, membership);
    if is(q, subject, sc::CONSTRAINT_USAGE)
        && owner_membership
            .is_some_and(|member| is(q, member, sc::REQUIREMENT_CONSTRAINT_MEMBERSHIP))
    {
        out.search_dependencies
            .insert(SearchDependency::ValidationRule(
                "ConstraintUsage::namingFeature",
            ));
        let reference = reference_subsetting(q, subject);
        let referenced = reference.value;
        merge(&mut out, reference);
        if let Some(referenced) = referenced {
            out.value = Some(referenced.and_then(|referenced| {
                let target = q.feature_target(referenced);
                let value = target.value;
                merge(&mut out, target);
                value
            }));
            return out;
        }
    }
    if owner_membership.is_some_and(|member| is(q, member, sc::VARIANT_MEMBERSHIP)) {
        out.search_dependencies
            .insert(SearchDependency::ValidationRule("Usage::namingFeature"));
        let reference = reference_subsetting(q, subject);
        out.value = Some(reference.value.flatten());
        merge(&mut out, reference);
        return out;
    }
    if !is(q, subject, sc::REFERENCE_USAGE) {
        return out;
    }
    out.search_dependencies
        .insert(SearchDependency::ValidationRule(
            "ReferenceUsage::namingFeature",
        ));
    let owner = q.owning_type(subject);
    let transition = owner.value;
    merge(&mut out, owner);
    let Some(transition) = transition else {
        return out;
    };
    merge(
        &mut out,
        q.canonical_fact_evidence(FactKey::Element(transition)),
    );
    if !is(q, transition, sc::TRANSITION_USAGE) {
        return out;
    }
    let inputs = input_parameters(q, transition);
    let second = inputs.value.get(1).copied();
    merge(&mut out, inputs);
    if second != Some(subject) {
        return out;
    }
    let payload = trigger_payload(q, transition);
    out.value = Some(payload.value.map(|(_, payload)| payload));
    merge(&mut out, payload);
    out
}

/// Runtime implementation of pinned SysML naming operations for the shared resolver.
#[derive(Default)]
pub struct SysmlNamingExtension;
impl agq_kerml_semantics::SemanticNamingExtension for SysmlNamingExtension {
    fn naming_source(
        &self,
        queries: &KerMlQueries<'_>,
        subject: ElementId,
    ) -> QueryResult<Option<Option<ElementId>>> {
        current_sysml_naming_source(queries, subject)
    }
}

pub(crate) fn plan_transition_payload(
    q: &KerMlQueries<'_>,
    profile: SysmlBaselineProfile,
    subject: ElementId,
) -> SysmlProducerResult {
    let rule = "checkTransitionUsagePayloadSpecialization";
    let payload = trigger_payload(q, subject);
    let selected = payload.value;
    let mut result = SysmlProducerResult {
        rule,
        evidence: payload.map(|_| ()),
        relationships: vec![],
        properties: vec![],
        elements: vec![],
    };
    result
        .evidence
        .search_dependencies
        .insert(SearchDependency::ValidationRule(rule));
    let Some((trigger, payload)) = selected else {
        return result;
    };
    let inputs = input_parameters(q, subject);
    let parameter = inputs.value.get(1).copied();
    merge(&mut result.evidence, inputs);
    let Some(parameter) = parameter else {
        pending(
            &mut result.evidence,
            subject,
            "Transition with a trigger requires its second input parameter",
        );
        return result;
    };
    let ancestors = q.all_supertypes(parameter);
    let mut satisfied = false;
    for &ancestor in &ancestors.value {
        if !is(q, ancestor, kc::FEATURE) {
            continue;
        }
        let chain = q.chaining_features(ancestor);
        satisfied |= chain.value.ends_with(&[trigger, payload]);
        merge(&mut result.evidence, chain);
    }
    merge(&mut result.evidence, ancestors);
    if satisfied || result.evidence.completeness != Completeness::Complete {
        return result;
    }
    let key = |role| DerivationKey {
        rule: profile.rule_id(rule),
        subject,
        output: crate::profile::output_key(role, parameter.as_u128()),
    };
    let chain = key("payload-chain");
    let first = key("payload-chain-trigger");
    let second = key("payload-chain-parameter");
    let subset = key("payload-subsetting");
    let reference = |id| SlotValue::Scalar(Value::Reference(id));
    // Explicit ordered ownedRelationship preserves trigger-before-payload;
    // unordered attachment merging must never choose a chain's semantic order.
    result.elements = vec![
        SysmlElementProposal {
            key: chain,
            metaclass: kc::FEATURE,
            owner: None,
            slots: BTreeMap::from([(
                kp::ELEMENT_OWNED_RELATIONSHIP,
                SlotValue::Ordered(vec![
                    Value::Reference(first.element_id()),
                    Value::Reference(second.element_id()),
                ]),
            )]),
        },
        SysmlElementProposal {
            key: first,
            metaclass: kc::FEATURE_CHAINING,
            owner: None,
            slots: BTreeMap::from([(kp::FEATURE_CHAINING_CHAINING_FEATURE, reference(trigger))]),
        },
        SysmlElementProposal {
            key: second,
            metaclass: kc::FEATURE_CHAINING,
            owner: None,
            slots: BTreeMap::from([(kp::FEATURE_CHAINING_CHAINING_FEATURE, reference(payload))]),
        },
        SysmlElementProposal {
            key: subset,
            metaclass: kc::SUBSETTING,
            owner: Some(parameter),
            slots: BTreeMap::from([
                (kp::SUBSETTING_SUBSETTING_FEATURE, reference(parameter)),
                (
                    kp::SUBSETTING_SUBSETTED_FEATURE,
                    reference(chain.element_id()),
                ),
                (
                    kp::RELATIONSHIP_OWNED_RELATED_ELEMENT,
                    SlotValue::Ordered(vec![Value::Reference(chain.element_id())]),
                ),
            ]),
        },
    ];
    result
}
