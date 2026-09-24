//! Structural platform queries. These are projections over canonical identities,
//! never execution semantics or substitutes for unimplemented formal validators.
use super::*;
use agq_kerml_semantics::SemanticClosureRequirement as Closure;

/// Metaclass selection over a Definition/Usage feature population. A kind includes
/// its subtypes; for example Part is also Item, and Interface is Connection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsageKind {
    Attribute,
    Item,
    Part,
    Port,
    Connection,
    Interface,
    Occurrence,
    Action,
    State,
    Requirement,
    Constraint,
    Case,
    VerificationCase,
}
impl UsageKind {
    fn metaclass(self) -> MetaclassId {
        match self {
            Self::Attribute => sc::ATTRIBUTE_USAGE,
            Self::Item => sc::ITEM_USAGE,
            Self::Part => sc::PART_USAGE,
            Self::Port => sc::PORT_USAGE,
            Self::Connection => sc::CONNECTION_USAGE,
            Self::Interface => sc::INTERFACE_USAGE,
            Self::Occurrence => sc::OCCURRENCE_USAGE,
            Self::Action => sc::ACTION_USAGE,
            Self::State => sc::STATE_USAGE,
            Self::Requirement => sc::REQUIREMENT_USAGE,
            Self::Constraint => sc::CONSTRAINT_USAGE,
            Self::Case => sc::CASE_USAGE,
            Self::VerificationCase => sc::VERIFICATION_CASE_USAGE,
        }
    }
}

/// The declared role on a StateSubactionMembership, not an execution phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateSubactionKind {
    Entry,
    Do,
    Exit,
}

/// The declared role on a TransitionFeatureMembership. The API selects the
/// canonical membership role without rewriting anomalous formal derivation text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionFeatureKind {
    Trigger,
    Guard,
    Effect,
}

/// Architectural role carried by a canonical feature membership.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequirementCaseRole {
    Subject,
    Actor,
    Objective,
}

impl SysmlQueries<'_> {
    /// Observe exact producer requirements. A missing or partial certificate
    /// retains its negative search and makes the composed answer Incomplete.
    pub(super) fn require_closure<T>(
        &self,
        out: &mut SysmlQueryResult<T>,
        subject: ElementId,
        requirements: &[Closure],
    ) -> bool {
        let mut closed = true;
        for &requirement in requirements {
            let witness = self.kerml.producer_closure(subject, requirement);
            closed &= witness.value && witness.completeness == Completeness::Complete;
            out.supporting_queries
                .push(witness.map(|value| if value { vec![subject] } else { vec![] }));
        }
        if !closed {
            out.pending
                .insert((subject, PendingSysmlRule::ProducerClosure));
        }
        closed
    }

    fn close_population(&self, out: &mut SysmlQueryResult<Vec<ElementId>>, owner: ElementId) {
        let ancestors = self.kerml.all_supertypes(owner);
        let subjects: BTreeSet<_> = ancestors
            .value
            .iter()
            .copied()
            .chain(out.value().iter().copied())
            .chain([owner])
            .collect();
        out.supporting_queries.push(ancestors);
        for subject in subjects {
            if self.is(subject, sc::DEFINITION) || self.is(subject, sc::USAGE) {
                self.pending_implications(out, subject);
            } else {
                self.require_closure(
                    out,
                    subject,
                    &[
                        Closure::EffectiveMembership,
                        Closure::EffectiveTyping,
                        Closure::EffectiveFeaturing,
                    ],
                );
            }
        }
    }

    fn select_kind(&self, out: &mut SysmlQueryResult<Vec<ElementId>>, class: MetaclassId) {
        let candidates = std::mem::take(&mut out.kerml.value);
        for candidate in candidates {
            self.observe(out, FactKey::Element(candidate));
            if self.is(candidate, class) {
                out.kerml.value.push(candidate);
            } else {
                out.filtered_targets.insert(candidate);
            }
        }
    }

    /// Direct canonical owned/nested Usage population filtered by metaclass.
    /// This is a current-graph query, matching [`Self::owned_usages`].
    pub fn owned_usages_of_kind(
        &self,
        owner: ElementId,
        kind: UsageKind,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.owned_usages(owner);
        self.select_kind(&mut out, kind.metaclass());
        out
    }

    /// Closed effective population including original inherited identities and
    /// suppressing redefined features. An empty answer still requires closure.
    pub fn effective_usages_of_kind(
        &self,
        owner: ElementId,
        kind: UsageKind,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.effective_usages(owner);
        self.select_kind(&mut out, kind.metaclass());
        out
    }

    /// Current nested Usage population; the receiver must itself be a Usage.
    pub fn nested_usages(&self, usage: ElementId) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.owned_usages(usage);
        self.check(&mut out, usage, &[sc::USAGE]);
        out
    }

    /// Closed nested population including inherited Usage identities.
    pub fn effective_nested_usages(&self, usage: ElementId) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.effective_usages(usage);
        self.check(&mut out, usage, &[sc::USAGE]);
        out
    }

    /// Composite members of one effective population; this is not a recursive
    /// copy/flatten operation or a claim about runtime instance containment.
    pub fn effective_composite_usages(
        &self,
        owner: ElementId,
        kind: UsageKind,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.effective_usages_of_kind(owner, kind);
        let candidates = std::mem::take(&mut out.kerml.value);
        for candidate in candidates {
            if self.boolean(&mut out, candidate, kp::FEATURE_IS_COMPOSITE) == Some(true) {
                out.kerml.value.push(candidate);
            } else {
                out.filtered_targets.insert(candidate);
            }
        }
        out
    }

    /// Effective composite Item children, retaining inherited member identity.
    pub fn effective_subitems(&self, owner: ElementId) -> SysmlQueryResult<Vec<ElementId>> {
        self.effective_composite_usages(owner, UsageKind::Item)
    }
    /// Effective composite Part children, retaining inherited member identity.
    pub fn effective_subparts(&self, owner: ElementId) -> SysmlQueryResult<Vec<ElementId>> {
        self.effective_composite_usages(owner, UsageKind::Part)
    }
    /// Closed effective PortUsage population.
    pub fn effective_ports(&self, owner: ElementId) -> SysmlQueryResult<Vec<ElementId>> {
        self.effective_usages_of_kind(owner, UsageKind::Port)
    }

    /// Closed AttributeUsage typing with the final KerML DataType domain.
    pub fn effective_attribute_definitions(
        &self,
        usage: ElementId,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.current_attribute_definitions(usage);
        self.pending_implications(&mut out, usage);
        out
    }
    /// Closed OccurrenceUsage typing with its profile-specific domain contract.
    pub fn effective_occurrence_definitions(
        &self,
        usage: ElementId,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.current_occurrence_definitions(usage);
        self.pending_implications(&mut out, usage);
        out
    }
    /// Closed ItemUsage typing selecting KerML Structure.
    pub fn effective_item_definitions(&self, usage: ElementId) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.current_item_definitions(usage);
        self.pending_implications(&mut out, usage);
        out
    }
    /// Closed PartUsage typing selecting PartDefinition.
    pub fn effective_part_definitions(&self, usage: ElementId) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.current_part_definitions(usage);
        self.pending_implications(&mut out, usage);
        out
    }
    /// Closed ConnectionUsage AssociationStructure definition population.
    pub fn effective_connection_definitions(
        &self,
        usage: ElementId,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.current_connection_definitions(usage);
        self.pending_implications(&mut out, usage);
        out
    }
    /// Closed PortUsage typing, including conjugated PortDefinitions.
    pub fn effective_port_definitions(&self, usage: ElementId) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.current_port_definitions(usage);
        self.pending_implications(&mut out, usage);
        out
    }

    /// Closed reflexive/transitive specialization population, composing KerML.
    pub fn effective_supertypes(&self, definition: ElementId) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.current_supertypes(definition);
        self.close_population(&mut out, definition);
        out
    }
    /// Closed direct subsetting population; ordinary KerML Features are retained.
    pub fn effective_subsetted_features(
        &self,
        usage: ElementId,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.subsetted_features(usage);
        self.close_population(&mut out, usage);
        out
    }
    /// Closed direct redefinition population, with canonical target identities.
    pub fn effective_redefined_features(
        &self,
        usage: ElementId,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.redefined_features(usage);
        self.close_population(&mut out, usage);
        out
    }

    /// Ordered effective connector ends. Definitions use Association structure;
    /// Usages use Connector structure. No end or endpoint records are copied.
    pub fn effective_connection_ends(
        &self,
        connection: ElementId,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        let answer = if self.is(connection, sc::CONNECTION_DEFINITION) {
            self.kerml
                .association_structure(connection)
                .map(|structure| structure.ends)
        } else {
            self.kerml
                .connector_related_structure(connection)
                .map(|structure| structure.ends)
        };
        let mut out = self.wrap(answer);
        self.check(
            &mut out,
            connection,
            &[sc::CONNECTION_DEFINITION, sc::CONNECTOR_AS_USAGE],
        );
        self.close_population(&mut out, connection);
        out
    }
    /// Closed interface ends, additionally requiring canonical PortUsage ends.
    pub fn effective_interface_ends(
        &self,
        interface: ElementId,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.effective_connection_ends(interface);
        self.check(
            &mut out,
            interface,
            &[sc::INTERFACE_DEFINITION, sc::INTERFACE_USAGE],
        );
        let candidates = out.value().clone();
        for candidate in candidates {
            self.check(&mut out, candidate, &[sc::PORT_USAGE]);
        }
        out
    }
    /// Closed ConnectorAsUsage related-feature population in semantic end order.
    pub fn effective_connection_related_features(
        &self,
        connection: ElementId,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.current_connection_related_features(connection);
        self.close_population(&mut out, connection);
        out
    }

    /// Closed non-result parameters in KerML semantic order. Applicable to
    /// occurrence/action definitions and usages, selecting directed features.
    pub fn effective_parameters(&self, owner: ElementId) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.wrap(self.kerml.structural_parameter_features(owner));
        self.check(
            &mut out,
            owner,
            &[sc::OCCURRENCE_DEFINITION, sc::OCCURRENCE_USAGE],
        );
        self.close_population(&mut out, owner);
        out
    }
    /// Closed result parameter population, separate from ordinary parameters.
    pub fn effective_return_parameters(
        &self,
        owner: ElementId,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.wrap(self.kerml.result_parameters(owner));
        self.check(&mut out, owner, &[sc::DEFINITION, sc::USAGE]);
        self.close_population(&mut out, owner);
        out
    }
    /// Closed composite action children. This query does not execute subactions.
    pub fn effective_subactions(&self, owner: ElementId) -> SysmlQueryResult<Vec<ElementId>> {
        self.effective_composite_usages(owner, UsageKind::Action)
    }

    fn enumeration<T>(
        &self,
        out: &mut SysmlQueryResult<T>,
        subject: ElementId,
        property: PropertyId,
    ) -> Option<String> {
        self.observe(
            out,
            FactKey::Property {
                element: subject,
                property,
            },
        );
        let Ok(PropertyState::Computed(slot)) = self.model.property_state(subject, property) else {
            return None;
        };
        let SlotValue::Scalar(Value::Enumeration(literal)) = slot.value() else {
            return None;
        };
        let agq_kernel::metamodel::ValueKind::Enumeration(domain) =
            self.model.registry().property(property).ok()?.value_kind
        else {
            return None;
        };
        self.model
            .registry()
            .enumeration(domain)
            .ok()?
            .literals
            .get(literal)
            .cloned()
    }

    fn owned_membership_features(
        &self,
        owner: ElementId,
        membership_kind: MetaclassId,
        role: Option<(PropertyId, &str)>,
        member_kind: MetaclassId,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        let memberships = self
            .kerml
            .owned_relationships_of_type(owner, membership_kind);
        let ids = memberships.value.clone();
        let mut out = self.wrap(memberships.map(|_| vec![]));
        self.require_closure(&mut out, owner, &[Closure::EffectiveMembership]);
        for membership in ids {
            self.observe(&mut out, FactKey::Element(membership));
            self.require_closure(&mut out, membership, &[Closure::EffectiveOwnership]);
            if let Some((property, role)) = role
                && self.enumeration(&mut out, membership, property).as_deref() != Some(role)
            {
                continue;
            }
            let member = self.kerml.member(membership);
            if let Some(member) = member.value {
                if self.check(&mut out, member, &[member_kind]) {
                    out.kerml.value.push(member);
                } else {
                    out.rejected_targets.insert(member);
                }
            }
            out.supporting_queries
                .push(member.map(|id| id.into_iter().collect()));
        }
        self.close_population(&mut out, owner);
        out
    }

    /// Owned entry/do/exit action identities in membership order. The pinned
    /// singular-property derivation text is not used to choose one of duplicates.
    pub fn state_actions(
        &self,
        state: ElementId,
        kind: StateSubactionKind,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        let role = match kind {
            StateSubactionKind::Entry => "entry",
            StateSubactionKind::Do => "do",
            StateSubactionKind::Exit => "exit",
        };
        let mut out = self.owned_membership_features(
            state,
            sc::STATE_SUBACTION_MEMBERSHIP,
            Some((sp::STATE_SUBACTION_MEMBERSHIP_KIND, role)),
            sc::ACTION_USAGE,
        );
        self.check(&mut out, state, &[sc::STATE_DEFINITION, sc::STATE_USAGE]);
        out
    }
    /// Owned trigger/guard/effect features in membership order; trigger actions
    /// are AcceptActionUsages, effects are ActionUsages, guards are Expressions.
    pub fn transition_features(
        &self,
        transition: ElementId,
        kind: TransitionFeatureKind,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        let (role, class) = match kind {
            TransitionFeatureKind::Trigger => ("trigger", sc::ACCEPT_ACTION_USAGE),
            TransitionFeatureKind::Guard => ("guard", kc::EXPRESSION),
            TransitionFeatureKind::Effect => ("effect", sc::ACTION_USAGE),
        };
        let mut out = self.owned_membership_features(
            transition,
            sc::TRANSITION_FEATURE_MEMBERSHIP,
            Some((sp::TRANSITION_FEATURE_MEMBERSHIP_KIND, role)),
            class,
        );
        self.check(&mut out, transition, &[sc::TRANSITION_USAGE]);
        out
    }
    /// First effective AcceptAction parameter is its payloadParameter.
    /// This differs from the Systems Library's undirected acceptedMessage
    /// feature, accessible through effective usages and KerML member lookup.
    /// Neither query returns a simulated message value.
    pub fn accept_action_payload_parameter(
        &self,
        action: ElementId,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        let mut out = self.effective_parameters(action);
        self.check(&mut out, action, &[sc::ACCEPT_ACTION_USAGE]);
        out.kerml.value.truncate(1);
        out
    }

    /// Effective requirement/case subject, actor or objective identities selected
    /// by canonical membership class. This plural structural projection does not
    /// repair the anomalous singular objectiveRequirement derivation text.
    pub fn requirement_case_features(
        &self,
        owner: ElementId,
        role: RequirementCaseRole,
    ) -> SysmlQueryResult<Vec<ElementId>> {
        let (membership_class, feature_class) = match role {
            RequirementCaseRole::Subject => (sc::SUBJECT_MEMBERSHIP, sc::REFERENCE_USAGE),
            RequirementCaseRole::Actor => (sc::ACTOR_MEMBERSHIP, sc::PART_USAGE),
            RequirementCaseRole::Objective => (sc::OBJECTIVE_MEMBERSHIP, sc::REQUIREMENT_USAGE),
        };
        let mut out = self.effective_usages(owner);
        self.check(
            &mut out,
            owner,
            &[
                sc::REQUIREMENT_DEFINITION,
                sc::REQUIREMENT_USAGE,
                sc::CASE_DEFINITION,
                sc::CASE_USAGE,
            ],
        );
        let candidates = std::mem::take(&mut out.kerml.value);
        for candidate in candidates {
            let membership = self.kerml.owning_relationship(candidate);
            let selected = membership.value.is_some_and(|id| {
                self.observe(&mut out, FactKey::Element(id));
                self.is(id, membership_class)
            });
            out.supporting_queries
                .push(membership.map(|id| id.into_iter().collect()));
            if selected {
                if self.check(&mut out, candidate, &[feature_class]) {
                    out.kerml.value.push(candidate);
                } else {
                    out.rejected_targets.insert(candidate);
                }
            } else {
                out.filtered_targets.insert(candidate);
            }
        }
        out
    }

    /// Effective names with a closed naming population for the element and all
    /// its type ancestors. Current-graph callers can use [`Self::current_names`].
    pub fn effective_names(&self, element: ElementId) -> SysmlQueryResult<EffectiveNames> {
        let mut out = self.current_names(element);
        let ancestors = self.kerml.all_supertypes(element);
        let ids: BTreeSet<_> = ancestors.value.iter().copied().chain([element]).collect();
        out.supporting_queries.push(ancestors);
        for id in ids {
            self.require_closure(&mut out, id, &[Closure::EffectiveNaming]);
        }
        out
    }
    /// Qualified name with closed naming/ownership and every searched namespace
    /// population. Missing closure remains visible even for a currently known name.
    pub fn effective_qualified_name(
        &self,
        element: ElementId,
    ) -> SysmlQueryResult<Option<QualifiedNamePath>> {
        let mut out = self.qualified_name(element, true);
        let mut current = Some(element);
        let mut visited = BTreeSet::new();
        while let Some(id) = current {
            if !visited.insert(id) {
                break;
            }
            self.require_closure(
                &mut out,
                id,
                &[
                    Closure::EffectiveNaming,
                    Closure::EffectiveOwnership,
                    Closure::EffectiveMembership,
                ],
            );
            let owner = self.kerml.owner(id);
            current = owner.value;
            out.supporting_queries
                .push(owner.map(|id| id.into_iter().collect()));
        }
        out
    }
}
