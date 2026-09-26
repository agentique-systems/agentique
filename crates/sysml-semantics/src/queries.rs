use crate::*;
use agq_kerml::{classes as kc, properties as kp};
use agq_kerml_semantics::{
    Completeness, Diagnostic, EffectiveNames, KerMlQueries, QueryResult, StandardRole,
};
use agq_kernel::{
    ElementId, MetaclassId, ModelView, PropertyId,
    derived::PropertyState,
    provenance::FactKey,
    value::{SlotValue, Value},
};
use agq_sysml::{classes as sc, properties as sp};
use std::collections::{BTreeMap, BTreeSet};

#[path = "structural_queries.rs"]
mod structural;
pub use structural::*;

#[cfg(test)]
#[path = "qualified_names_tests.rs"]
mod qualified_names_tests;

#[cfg(test)]
#[path = "projection_tests.rs"]
mod projection_tests;

/// Unfinished SysML implications are separate from determinate current-graph
/// answers. No entry authorizes a new standards correction.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PendingSysmlRule {
    /// The required canonical base edge is missing (or its anchor is unavailable).
    StandardGeneralization(StandardSysmlRole),
    KerMlGeneralization(StandardRole),
    /// No compatible certificate closes the required producer effects.
    ProducerClosure,
    /// Final formal text says singular subitem; the pinned library has subitems.
    CompositeItemSubsettingAuthorityGap,
    Variation,
    Individual,
    Portion,
    MayTimeVary,
    SpecializedSemantics,
}

/// An answer over the same kernel IDs, retaining the composed KerML evidence.
/// `kerml.completeness` describes that graph query; use [`Self::completeness`]
/// for the SysML projection including its diagnostics and pending implications.
#[derive(Clone, Debug)]
pub struct SysmlQueryResult<T> {
    pub context: SysmlSemanticContextId,
    pub kerml: QueryResult<T>,
    pub pending: BTreeSet<(ElementId, PendingSysmlRule)>,
    pub diagnostics: BTreeSet<Diagnostic>,
    /// Out-of-domain endpoints remain visible, with Invalid or pending-domain
    /// diagnostics distinguishing incorrect assertions from unfinished typing.
    pub rejected_targets: BTreeSet<ElementId>,
    /// Proven general ancestors and valid members outside a selectByKind subset.
    /// Their original evidence remains in the composed/supporting query answers.
    pub filtered_targets: BTreeSet<ElementId>,
    /// Additional current-graph queries used to inspect producer prerequisites.
    pub supporting_queries: Vec<QueryResult<Vec<ElementId>>>,
    pub supporting_names: Vec<QueryResult<EffectiveNames>>,
    /// Canonical observations reuse KerML's complete proof/search expansion.
    pub observations: BTreeMap<FactKey, QueryResult<()>>,
    additional_completeness: Completeness,
}

/// Raw qualified-name components in namespace order, excluding the root namespace.
/// Supported segments have one established full name; no short name is substituted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QualifiedNamePath {
    pub segments: Vec<BTreeSet<String>>,
}
impl<T> SysmlQueryResult<T> {
    pub fn value(&self) -> &T {
        &self.kerml.value
    }
    pub fn completeness(&self) -> Completeness {
        self.supporting_queries
            .iter()
            .map(|q| q.completeness)
            .chain(self.supporting_names.iter().map(|q| q.completeness))
            .chain(self.observations.values().map(|q| q.completeness))
            .chain([
                self.kerml.completeness,
                self.additional_completeness,
                if self.pending.is_empty() {
                    Completeness::Complete
                } else {
                    Completeness::Incomplete
                },
            ])
            .max()
            .expect("base answer")
    }
    fn problem(
        &mut self,
        status: Completeness,
        code: &'static str,
        subject: ElementId,
        message: &str,
    ) {
        self.additional_completeness = self.additional_completeness.max(status);
        self.diagnostics.insert(Diagnostic {
            code,
            subject,
            message: message.into(),
        });
    }
}

/// Borrowed SysML queries composed with the unchanged KerML evaluator.
pub struct SysmlQueries<'m> {
    model: &'m ModelView,
    kerml: KerMlQueries<'m>,
    context: SysmlSemanticContextId,
    bindings: StandardSysmlBindings,
}

/// Immutable query input retained after complete SysML context authentication.
/// Its graph, bindings and composed language identities cannot be replaced.
#[derive(Clone, Debug)]
pub struct RetainedSysmlContext {
    kerml: agq_kerml_semantics::RetainedSemanticContext,
    context: SysmlSemanticContextId,
    bindings: StandardSysmlBindings,
}

impl RetainedSysmlContext {
    /// Start a bounded evaluator over exactly the retained immutable revision.
    pub fn queries(&self) -> SysmlQueries<'_> {
        let context = self.kerml.borrow();
        SysmlQueries {
            model: context.model(),
            kerml: KerMlQueries::new(context),
            context: self.context.clone(),
            bindings: self.bindings.clone(),
        }
    }
}
impl<'m> SysmlQueries<'m> {
    pub fn new(context: SysmlSemanticContext<'m>) -> Self {
        Self {
            model: context.model,
            kerml: KerMlQueries::new(context.kerml),
            context: context.id,
            bindings: context.bindings,
        }
    }
    pub fn context(&self) -> &SysmlSemanticContextId {
        &self.context
    }
    /// Retain the exact composed semantic input, excluding temporary query caches.
    pub fn retain_context(&self) -> RetainedSysmlContext {
        RetainedSysmlContext {
            kerml: self.kerml.retain_context(),
            context: self.context.clone(),
            bindings: self.bindings.clone(),
        }
    }
    /// Start a fresh evaluator over the same borrowed model, authenticated
    /// context and bindings. Query caches are independent; graph fingerprints,
    /// producer closure and dependency identities are preserved without rebinding.
    pub fn fork(&self) -> Self {
        Self {
            model: self.model,
            kerml: self.kerml.fork(),
            context: self.context.clone(),
            bindings: self.bindings.clone(),
        }
    }
    pub fn kerml(&self) -> &KerMlQueries<'m> {
        &self.kerml
    }
    pub fn model(&self) -> &'m ModelView {
        self.model
    }

    /// Plan positive SysML contributions over this exact current graph. The
    /// caller supplies publication roots and schedules resulting canonical facts.
    pub fn producer_plan(&self, roots: &[ElementId], subject: ElementId) -> SysmlProducerPlan {
        plan_sysml_producers(
            &self.kerml,
            self.context.dependencies.sysml_profile,
            &self.bindings,
            roots,
            subject,
        )
    }

    /// Structural derivation over the current graph, without a closure claim.
    pub fn current_may_time_vary(
        &self,
        roots: &[ElementId],
        usage: ElementId,
    ) -> SysmlQueryResult<Option<bool>> {
        self.wrap(current_usage_may_time_vary(
            &self.kerml,
            self.context.dependencies.sysml_profile,
            &self.bindings,
            roots,
            usage,
        ))
    }

    fn wrap<T>(&self, kerml: QueryResult<T>) -> SysmlQueryResult<T> {
        SysmlQueryResult {
            context: self.context.clone(),
            kerml,
            pending: BTreeSet::new(),
            diagnostics: BTreeSet::new(),
            rejected_targets: BTreeSet::new(),
            filtered_targets: BTreeSet::new(),
            supporting_queries: vec![],
            supporting_names: vec![],
            observations: BTreeMap::new(),
            additional_completeness: Completeness::Complete,
        }
    }
    fn is(&self, id: ElementId, class: MetaclassId) -> bool {
        self.model.element(id).is_some_and(|r| {
            self.model
                .registry()
                .is_subtype(r.metaclass(), class)
                .unwrap_or(false)
        })
    }
    fn observe<T>(&self, out: &mut SysmlQueryResult<T>, fact: FactKey) {
        out.observations
            .entry(fact)
            .or_insert_with(|| self.kerml.canonical_fact_evidence(fact));
    }
    fn check<T>(
        &self,
        out: &mut SysmlQueryResult<T>,
        id: ElementId,
        classes: &[MetaclassId],
    ) -> bool {
        self.observe(out, FactKey::Element(id));
        if self.model.element(id).is_none() {
            out.problem(
                Completeness::Incomplete,
                "SQ_MISSING_ELEMENT",
                id,
                "Canonical element is unavailable",
            );
            return false;
        }
        if !classes.iter().any(|class| self.is(id, *class)) {
            out.problem(
                Completeness::Invalid,
                "SQ_ELEMENT_KIND",
                id,
                "Canonical element has the wrong SysML metaclass",
            );
            return false;
        }
        true
    }
    fn boolean<T>(
        &self,
        out: &mut SysmlQueryResult<T>,
        id: ElementId,
        property: PropertyId,
    ) -> Option<bool> {
        self.observe(
            out,
            FactKey::Property {
                element: id,
                property,
            },
        );
        match self.model.property_state(id, property) {
            Ok(PropertyState::Computed(slot)) => match slot.value() {
                SlotValue::Scalar(Value::Boolean(value)) => Some(*value),
                _ => {
                    out.problem(
                        Completeness::Invalid,
                        "SQ_BOOLEAN_KIND",
                        id,
                        "Expected a canonical Boolean",
                    );
                    None
                }
            },
            Ok(PropertyState::Invalid(_)) => {
                out.problem(
                    Completeness::Invalid,
                    "SQ_INVALID_PROPERTY",
                    id,
                    "Canonical Boolean computation is invalid",
                );
                None
            }
            _ => {
                out.problem(
                    Completeness::Incomplete,
                    "SQ_PENDING_PROPERTY",
                    id,
                    "Canonical Boolean has not been established",
                );
                None
            }
        }
    }

    /// Direct FeatureTyping endpoints. Usage::definition targets Classifier,
    /// including valid KerML Classifiers that are not SysML Definitions.
    pub fn direct_usage_types(&self, usage: ElementId) -> SysmlQueryResult<Vec<ElementId>> {
        let out = self.wrap(self.kerml.direct_feature_types(usage));
        self.type_projection(out, usage, sc::USAGE, kc::CLASSIFIER)
    }
    /// Derived Usage::definition over the current graph; no producer-closure claim.
    pub fn current_usage_types(&self, usage: ElementId) -> SysmlQueryResult<Vec<ElementId>> {
        let out = self.prune_general_types(self.wrap(self.kerml.feature_types(usage)));
        self.type_projection(out, usage, sc::USAGE, kc::CLASSIFIER)
    }
    /// Effective Usage::definition, with unfinished SysML implications visible.
    pub fn effective_usage_types(&self, usage: ElementId) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.current_usage_types(usage);
        self.pending_implications(&mut out, usage);
        out
    }
    fn type_projection(
        &self,
        mut out: SysmlQueryResult<Vec<ElementId>>,
        subject: ElementId,
        subject_kind: MetaclassId,
        target_kind: MetaclassId,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        self.check(&mut out, subject, &[subject_kind]);
        let targets = std::mem::take(&mut out.kerml.value);
        for target in targets {
            if self.check(&mut out, target, &[target_kind]) {
                out.kerml.value.push(target);
            } else {
                out.rejected_targets.insert(target);
            }
        }
        out
    }
    pub fn current_attribute_definitions(
        &self,
        usage: ElementId,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        self.current_typed_domain(usage, sc::ATTRIBUTE_USAGE, kc::DATA_TYPE)
    }
    /// OccurrenceUsage::occurrenceDefinition narrows Usage::definition to Class.
    /// Operational v3 separately resolves the bounded ConnectionUsage plain
    /// Association conflict; other incompatible classifiers remain Invalid.
    pub fn current_occurrence_definitions(
        &self,
        usage: ElementId,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        self.current_typed_domain(usage, sc::OCCURRENCE_USAGE, kc::CLASS)
    }
    /// ItemUsage::itemDefinition selects Structure, including ordinary KerML Structure.
    pub fn current_item_definitions(&self, usage: ElementId) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.current_occurrence_definitions(usage);
        self.check(&mut out, usage, &[sc::ITEM_USAGE]);
        self.select_type_subset(&mut out, kc::STRUCTURE);
        out
    }
    pub fn current_part_definitions(&self, usage: ElementId) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.current_item_definitions(usage);
        self.check(&mut out, usage, &[sc::PART_USAGE]);
        self.select_type_subset(&mut out, sc::PART_DEFINITION);
        if out.value().is_empty() && out.completeness() != Completeness::Invalid {
            out.problem(
                Completeness::Incomplete,
                "SQ_PART_DEFINITION_PENDING",
                usage,
                "Required PartDefinition typing is not established before SysML producer closure",
            );
            out.pending.insert((
                usage,
                PendingSysmlRule::StandardGeneralization(StandardSysmlRole::Parts),
            ));
        }
        out
    }
    pub fn current_port_definitions(&self, usage: ElementId) -> SysmlQueryResult<Vec<ElementId>> {
        self.current_typed_domain(usage, sc::PORT_USAGE, sc::PORT_DEFINITION)
    }
    /// ConnectionUsage::connectionDefinition is the AssociationStructure subset
    /// of itemDefinition and narrows Connector::association. The occurrence
    /// domain check preserves its inherited validation obligations.
    pub fn current_connection_definitions(
        &self,
        usage: ElementId,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.current_item_definitions(usage);
        self.check(&mut out, usage, &[sc::CONNECTION_USAGE]);
        self.select_type_subset(&mut out, kc::ASSOCIATION_STRUCTURE);
        out
    }

    /// KerML's producer-safe feature_types deliberately prunes canonical edges.
    /// A partial SysML graph can still expose a general type supplied by virtual
    /// KerML library ancestry. Reuse all_supertypes (and all its evidence) to
    /// establish that redundancy before applying the narrowed SysML domain.
    /// This adapter neither creates relationships nor certifies producer closure.
    fn prune_general_types(
        &self,
        mut out: SysmlQueryResult<Vec<ElementId>>,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        let types: BTreeSet<_> = out.value().iter().copied().collect();
        let mut general = BTreeSet::new();
        if types.len() > 1 {
            for &specific in &types {
                let ancestors = self.kerml.all_supertypes(specific);
                general.extend(
                    ancestors
                        .value
                        .iter()
                        .copied()
                        .filter(|id| *id != specific && types.contains(id)),
                );
                out.supporting_queries.push(ancestors);
            }
        }
        out.kerml.value.retain(|id| !general.contains(id));
        out.filtered_targets.extend(general);
        out
    }

    fn current_typed_domain(
        &self,
        usage: ElementId,
        subject_kind: MetaclassId,
        domain: MetaclassId,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.current_usage_types(usage);
        let direct = self.kerml.direct_feature_types(usage);
        let untyped = direct.value.is_empty();
        out.supporting_queries.push(direct);
        self.check(&mut out, usage, &[subject_kind]);
        let targets = std::mem::take(&mut out.kerml.value);
        for target in targets {
            self.observe(&mut out, FactKey::Element(target));
            if self.is(target, domain) {
                out.kerml.value.push(target);
            } else if domain == kc::CLASS
                && self
                    .context
                    .dependencies
                    .sysml_profile
                    .permits_connection_association_types()
                && self.is(usage, sc::CONNECTION_USAGE)
                && self.is(target, kc::ASSOCIATION)
            {
                // AGQ-SYSML20-005: the pinned Flows library uses a plain
                // Association here. Preserve its canonical typing and the broad
                // definition result; only this typed projection excludes it.
                out.filtered_targets.insert(target);
            } else if untyped && self.is(target, kc::CLASSIFIER) {
                // The required SysML library base can supply a more specific type
                // to an untyped usage. Retain the nonconforming candidate as
                // rejected evidence and report the outstanding domain honestly.
                out.rejected_targets.insert(target);
                out.problem(Completeness::Incomplete, "SQ_TYPE_DOMAIN_PENDING", usage,
                    "Current inherited types do not establish the narrowed domain before SysML base typing");
                out.pending
                    .insert((usage, PendingSysmlRule::ProducerClosure));
            } else {
                self.check(&mut out, target, &[domain]);
                out.rejected_targets.insert(target);
            }
        }
        out
    }

    fn select_type_subset(
        &self,
        out: &mut SysmlQueryResult<Vec<ElementId>>,
        selected: MetaclassId,
    ) {
        let values = std::mem::take(&mut out.kerml.value);
        for target in values {
            self.observe(out, FactKey::Element(target));
            if self.is(target, selected) {
                out.kerml.value.push(target);
            } else {
                out.filtered_targets.insert(target);
            }
        }
    }
    /// General KerML specialization endpoints are retained, not filtered to SysML Definition.
    pub fn direct_specializations(
        &self,
        definition: ElementId,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.wrap(self.kerml.direct_specializations(definition));
        self.check(&mut out, definition, &[sc::DEFINITION]);
        out
    }
    /// Reflexive transitive set, matching KerML all_supertypes exactly.
    pub fn current_supertypes(&self, definition: ElementId) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.wrap(self.kerml.all_supertypes(definition));
        self.check(&mut out, definition, &[sc::DEFINITION]);
        out
    }
    /// Definition::ownedUsage / Usage::nestedUsage from canonical owned memberships.
    pub fn owned_usages(&self, owner: ElementId) -> SysmlQueryResult<Vec<ElementId>> {
        self.usage_projection(self.kerml.direct_features(owner), owner)
    }
    /// Identity set of effective usages on the current graph. The vector is a
    /// carrier of set members, never a claim about normative feature ordering.
    pub fn current_effective_usages(&self, owner: ElementId) -> SysmlQueryResult<Vec<ElementId>> {
        self.usage_projection(self.kerml.effective_features(owner), owner)
    }
    fn usage_projection(
        &self,
        answer: QueryResult<Vec<ElementId>>,
        owner: ElementId,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.wrap(answer);
        self.check(&mut out, owner, &[sc::DEFINITION, sc::USAGE]);
        let values = std::mem::take(&mut out.kerml.value);
        for id in values {
            // Non-Usage Features are valid KerML members and simply outside this
            // selectByKind projection. Observe every tested metaclass nonetheless.
            self.observe(&mut out, FactKey::Element(id));
            if self.is(id, sc::USAGE) {
                out.kerml.value.push(id);
            }
        }
        out
    }
    /// Effective usage identities plus outstanding implications for the owner,
    /// every current ancestor and every returned usage. No records are allocated.
    pub fn effective_usages(&self, owner: ElementId) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.current_effective_usages(owner);
        let ancestors = self.kerml.all_supertypes(owner);
        let subjects: BTreeSet<_> = ancestors
            .value
            .iter()
            .copied()
            .chain(out.kerml.value.iter().copied())
            .chain([owner])
            .collect();
        out.supporting_queries.push(ancestors);
        for subject in subjects {
            self.pending_implications(&mut out, subject);
        }
        out
    }
    /// Subsetting may target ordinary KerML Features; those identities are retained.
    pub fn subsetted_features(&self, usage: ElementId) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.wrap(self.kerml.subsetted_features(usage));
        self.check(&mut out, usage, &[sc::USAGE]);
        out
    }
    pub fn redefined_features(&self, usage: ElementId) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.wrap(self.kerml.redefined_features(usage));
        self.check(&mut out, usage, &[sc::USAGE]);
        out
    }
    pub fn all_redefined_features(&self, usage: ElementId) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.wrap(self.kerml.all_redefined_features(usage));
        self.check(&mut out, usage, &[sc::USAGE]);
        out
    }
    /// ConnectorAsUsage structural endpoints, preserving the KerML distinction
    /// between related features and stricter actual binding endpoint obligations.
    pub fn current_connection_related_features(
        &self,
        connection: ElementId,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.wrap(self.kerml.connector_related_features(connection));
        self.check(&mut out, connection, &[sc::CONNECTOR_AS_USAGE]);
        out
    }
    /// Current-graph names compose the KerML evaluator with SysML namingFeature
    /// overrides. Variation typing obligations do not change name completeness.
    pub fn current_names(&self, element: ElementId) -> SysmlQueryResult<EffectiveNames> {
        let mut out = self.wrap(self.kerml.effective_names(element));
        self.check(&mut out, element, &[sc::DEFINITION, sc::USAGE]);
        out
    }

    fn variant_membership<T>(
        &self,
        out: &mut SysmlQueryResult<T>,
        element: ElementId,
        producer_closed: bool,
    ) {
        let relationship = self.kerml.owning_relationship(element);
        if relationship
            .value
            .is_some_and(|id| self.is(id, sc::VARIANT_MEMBERSHIP))
        {
            let owner = self.kerml.owner(element);
            let mut established = false;
            if let Some(owner) = owner.value {
                self.observe(out, FactKey::Element(owner));
                if self.is(owner, sc::DEFINITION)
                    && self.boolean(out, owner, sp::DEFINITION_IS_VARIATION) == Some(true)
                {
                    // checkUsageVariationDefinitionSpecialization is a canonical
                    // specialization obligation. A query cannot supply its type.
                    let ancestors = self.kerml.all_supertypes(element);
                    established = producer_closed && ancestors.value.contains(&owner);
                    out.supporting_queries.push(ancestors);
                }
            }
            out.supporting_queries
                .push(owner.map(|id| id.into_iter().collect()));
            if !established {
                out.pending.insert((element, PendingSysmlRule::Variation));
            }
        }
        out.supporting_queries
            .push(relationship.map(|id| id.into_iter().collect()));
    }

    /// Current-graph deriveElementQualifiedName root boundary and owned-name population.
    /// Explicit full names are supported. Inherited-only name selection and
    /// duplicate sibling names remain Incomplete instead of choosing an endpoint.
    pub fn current_qualified_name(
        &self,
        element: ElementId,
    ) -> SysmlQueryResult<Option<QualifiedNamePath>> {
        self.qualified_name(element, false)
    }

    pub(super) fn qualified_name(
        &self,
        element: ElementId,
        effective: bool,
    ) -> SysmlQueryResult<Option<QualifiedNamePath>> {
        let mut out = self.wrap(self.kerml.owner(element).map(|_| None));
        self.check(&mut out, element, &[kc::ELEMENT]);
        let mut current = Some(element);
        let mut visited = BTreeSet::new();
        let mut segments = Vec::new();
        let mut closed_name_subjects = BTreeSet::new();
        while let Some(id) = current {
            if !visited.insert(id) {
                out.problem(
                    Completeness::Invalid,
                    "SQ_OWNER_CYCLE",
                    id,
                    "Cyclic canonical ownership cannot establish a qualified name",
                );
                return out;
            }
            let membership = self.kerml.owning_relationship(id);
            let owned_member = membership
                .value
                .is_some_and(|m| self.is(m, kc::OWNING_MEMBERSHIP));
            out.supporting_queries
                .push(membership.map(|id| id.into_iter().collect()));
            if !owned_member {
                return out;
            }
            let owner = self.kerml.owner(id);
            let namespace = owner.value;
            out.supporting_queries
                .push(owner.map(|id| id.into_iter().collect()));
            let Some(namespace) = namespace else {
                return out;
            };
            let Some(name) = self.qualified_segment(&mut out, id) else {
                return out;
            };
            if !self.unique_qualified_segment(
                &mut out,
                namespace,
                id,
                &name,
                effective.then_some(&mut closed_name_subjects),
            ) {
                return out;
            }
            segments.push(BTreeSet::from([name]));
            // KerML 1.0 deriveElementQualifiedName: a root Namespace has no
            // qualifiedName; its own name (if any) is excluded from its members'.
            let namespace_owner = self.kerml.owner(namespace);
            let is_root = namespace_owner.value.is_none();
            out.supporting_queries
                .push(namespace_owner.map(|id| id.into_iter().collect()));
            if is_root {
                break;
            }
            current = Some(namespace);
        }
        if out.completeness() != Completeness::Complete {
            return out;
        }
        segments.reverse();
        out.kerml.value = Some(QualifiedNamePath { segments });
        out
    }

    fn qualified_segment<T>(
        &self,
        out: &mut SysmlQueryResult<T>,
        element: ElementId,
    ) -> Option<String> {
        if let Some(name) = self.declared_qualified_segment(out, element) {
            return Some(name);
        }
        // KerML's current naming answer combines effective long and short names.
        // It cannot prove which nonempty inherited/short-name choice is name.
        let names = self.kerml.effective_names(element);
        if !matches!(&names.value, EffectiveNames::Determinate(names) if names.is_empty()) {
            out.problem(Completeness::Incomplete, "SQ_QUALIFIED_NAME_SELECTION", element,
                "The effective full name is not established independently of short-name alternatives");
        }
        out.supporting_names.push(names);
        None
    }

    fn declared_qualified_segment<T>(
        &self,
        out: &mut SysmlQueryResult<T>,
        element: ElementId,
    ) -> Option<String> {
        self.observe(
            out,
            FactKey::Property {
                element,
                property: kp::ELEMENT_DECLARED_NAME,
            },
        );
        if let Ok(PropertyState::Computed(slot)) = self
            .model
            .property_state(element, kp::ELEMENT_DECLARED_NAME)
            && let SlotValue::Scalar(Value::String(name)) = slot.value()
        {
            return Some(name.clone());
        }
        None
    }

    fn unique_qualified_segment<T>(
        &self,
        out: &mut SysmlQueryResult<T>,
        namespace: ElementId,
        element: ElementId,
        name: &str,
        mut closed_name_subjects: Option<&mut BTreeSet<ElementId>>,
    ) -> bool {
        if self
            .kerml
            .context()
            .pending_namespace_scopes
            .contains(&namespace)
            || self
                .kerml
                .context()
                .pending_specialization_scopes
                .contains(&namespace)
        {
            out.problem(
                Completeness::Incomplete,
                "SQ_PENDING_QUALIFIED_NAME_POPULATION",
                namespace,
                "The owned-name population may still change",
            );
            return false;
        }
        let memberships = self.kerml.memberships(namespace);
        let mut matches = BTreeSet::new();
        for &membership in &memberships.value {
            if !self.is(membership, kc::OWNING_MEMBERSHIP) {
                continue;
            }
            let member = self.kerml.member(membership);
            if let Some(target) = member.value {
                if let Some(other_name) = self.declared_qualified_segment(out, target) {
                    if other_name == name {
                        matches.insert(target);
                    }
                } else {
                    let names = self.kerml.effective_names(target);
                    // A complete full/short-name population can exclude a
                    // sibling without selecting which alternative is its full
                    // name. Empty-name and explicit-name paths are unchanged.
                    let excluded = names.completeness == Completeness::Complete
                        && matches!(&names.value, EffectiveNames::Determinate(names)
                            if !names.is_empty() && !names.contains(name));
                    if excluded {
                        if let Some(closed_name_subjects) = &mut closed_name_subjects {
                            self.close_qualified_name_sources(
                                out,
                                target,
                                &names,
                                closed_name_subjects,
                            );
                        }
                        out.supporting_names.push(names);
                    } else {
                        let _ = self.qualified_segment(out, target);
                    }
                }
            }
            out.supporting_queries
                .push(member.map(|id| id.into_iter().collect()));
        }
        out.supporting_queries.push(memberships);
        if matches != BTreeSet::from([element]) {
            out.problem(Completeness::Incomplete, "SQ_QUALIFIED_NAME_COLLISION", element,
                "Duplicate or unavailable owned names require first-member qualified-name selection");
            return false;
        }
        out.completeness() == Completeness::Complete
    }

    fn qualified_name_sources(
        sibling: ElementId,
        names: &QueryResult<EffectiveNames>,
    ) -> BTreeSet<ElementId> {
        let mut sources = BTreeSet::from([sibling]);
        for conclusion in names.explanations.keys() {
            if conclusion.query == agq_kerml_semantics::QueryKind::NamingSource {
                sources.extend([conclusion.subject, conclusion.value]);
            }
        }
        sources
    }

    fn close_qualified_name_sources<T>(
        &self,
        out: &mut SysmlQueryResult<T>,
        sibling: ElementId,
        names: &QueryResult<EffectiveNames>,
        closed_name_subjects: &mut BTreeSet<ElementId>,
    ) {
        use agq_kerml_semantics::{SearchDependency, SemanticClosureRequirement as Closure};
        let sources = Self::qualified_name_sources(sibling, names);
        let mut subjects = sources.clone();
        for source in sources {
            // Packages and other ordinary Elements can be owned siblings too.
            if self.is(source, kc::TYPE) {
                let ancestors = self.kerml.all_supertypes(source);
                subjects.extend(ancestors.value.iter().copied());
                out.supporting_queries.push(ancestors);
            }
        }
        // Naming overrides can inspect another owner's members before choosing
        // a source (or falling back), notably transition inputs and payloads.
        let owners: BTreeSet<_> = names
            .search_dependencies
            .iter()
            .filter_map(|search| match search {
                SearchDependency::OwnedRelationships { owner, .. }
                | SearchDependency::OwnedRelationshipsExcluding { owner, .. }
                | SearchDependency::StructuralFeaturePopulation { owner, .. } => Some(*owner),
                SearchDependency::NamespaceMembers { namespace } => Some(*namespace),
                SearchDependency::SourceRelationships { source, .. } => Some(*source),
                SearchDependency::PropertySet { element, .. } => Some(*element),
                SearchDependency::Incoming { target } => Some(*target),
                _ => None,
            })
            .collect();
        subjects.extend(owners);
        for subject in subjects {
            if closed_name_subjects.insert(subject) {
                self.require_closure(out, subject, &Closure::ALL);
            }
        }
    }

    fn pending_implications<T>(&self, out: &mut SysmlQueryResult<T>, subject: ElementId) {
        if !self.is(subject, sc::DEFINITION) && !self.is(subject, sc::USAGE) {
            return;
        }
        // A validated combined-registry certificate closes the implemented
        // producer boundary for this exact graph and semantic profile. Retain
        // every positive/negative closure search in the composed answer.
        let producer_closed = self.require_closure(
            out,
            subject,
            &agq_kerml_semantics::SemanticClosureRequirement::ALL,
        );
        if !producer_closed {
            let role = if self.is(subject, sc::PART_DEFINITION) {
                Some(StandardSysmlRole::Part)
            } else if self.is(subject, sc::ITEM_DEFINITION) {
                Some(StandardSysmlRole::Item)
            } else if self.is(subject, sc::PART_USAGE) {
                Some(StandardSysmlRole::Parts)
            } else if self.is(subject, sc::ITEM_USAGE) {
                Some(StandardSysmlRole::Items)
            } else {
                None
            };
            let kerml_role = if self.is(subject, sc::ATTRIBUTE_USAGE) {
                Some(StandardRole::DataValues)
            } else if self.is(subject, sc::ATTRIBUTE_DEFINITION) {
                Some(StandardRole::DataValue)
            } else if role.is_none() && self.is(subject, sc::OCCURRENCE_USAGE) {
                Some(StandardRole::Occurrences)
            } else {
                None
            };
            let ancestors = self.kerml.all_supertypes(subject);
            if let Some(role) = role {
                if self
                    .bindings
                    .get(role)
                    .is_none_or(|id| !ancestors.value.contains(&id))
                {
                    out.pending
                        .insert((subject, PendingSysmlRule::StandardGeneralization(role)));
                }
            } else if let Some(role) = kerml_role
                && self
                    .kerml
                    .context()
                    .standard_bindings
                    .as_ref()
                    .is_none_or(|bindings| !ancestors.value.contains(&bindings.get(role)))
            {
                out.pending
                    .insert((subject, PendingSysmlRule::KerMlGeneralization(role)));
            }
            out.supporting_queries.push(ancestors);
        }
        let variation = if self.is(subject, sc::USAGE) {
            sp::USAGE_IS_VARIATION
        } else {
            sp::DEFINITION_IS_VARIATION
        };
        if self.boolean(out, subject, variation) == Some(true)
            && (!producer_closed || self.is(subject, sc::USAGE))
        {
            // Definition-owned variant specialization is implemented and its
            // canonical effects are closed by the certificate. Variation Usage
            // featuring/specialization is still a separate unfinished boundary.
            out.pending.insert((subject, PendingSysmlRule::Variation));
        }
        if self.is(subject, sc::USAGE) {
            self.variant_membership(out, subject, producer_closed);
        }
        let individual = if self.is(subject, sc::OCCURRENCE_USAGE) {
            Some(sp::OCCURRENCE_USAGE_IS_INDIVIDUAL)
        } else if self.is(subject, sc::OCCURRENCE_DEFINITION) {
            Some(sp::OCCURRENCE_DEFINITION_IS_INDIVIDUAL)
        } else {
            None
        };
        if let Some(property) = individual
            && self.boolean(out, subject, property) == Some(true)
        {
            out.pending.insert((subject, PendingSysmlRule::Individual));
        }
        if self.is(subject, sc::OCCURRENCE_USAGE) {
            let property = sp::OCCURRENCE_USAGE_PORTION_KIND;
            self.observe(
                out,
                FactKey::Property {
                    element: subject,
                    property,
                },
            );
            if matches!(
                self.model.property_state(subject, property),
                Ok(PropertyState::Computed(_))
            ) {
                out.pending.insert((subject, PendingSysmlRule::Portion));
            }
        }
        if !producer_closed && self.is(subject, sc::USAGE) {
            // Before closure the scalar producer can alter featuring. Queries
            // that consume the derived value itself must still observe its
            // property state; a closed structural projection never invents false.
            if self
                .boolean(out, subject, sp::USAGE_MAY_TIME_VARY)
                .is_none()
            {
                out.pending.insert((subject, PendingSysmlRule::MayTimeVary));
            }
        }
        if self.context.dependencies.sysml_profile == SysmlBaselineProfile::PUBLISHED
            && self.is(subject, sc::ITEM_USAGE)
            && self.boolean(out, subject, kp::FEATURE_IS_COMPOSITE) == Some(true)
        {
            let owner = self.kerml.owning_type(subject);
            if owner
                .value
                .is_some_and(|id| self.is(id, sc::ITEM_DEFINITION) || self.is(id, sc::ITEM_USAGE))
            {
                out.pending.insert((
                    subject,
                    PendingSysmlRule::CompositeItemSubsettingAuthorityGap,
                ));
            }
            out.supporting_queries
                .push(owner.map(|value| value.into_iter().collect()));
        }
        if !producer_closed
            && (self.is(subject, sc::PORT_USAGE)
                || self.is(subject, sc::CONNECTION_USAGE)
                || self.is(subject, sc::PORT_DEFINITION)
                || self.is(subject, sc::CONNECTION_DEFINITION))
        {
            out.pending
                .insert((subject, PendingSysmlRule::SpecializedSemantics));
        }
    }
}
