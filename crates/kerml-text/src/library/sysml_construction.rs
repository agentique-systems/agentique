//! SysML canonical lowering contracts over the shared transaction builder.
//!
//! The explicit production dispatch follows the metaclasses and owned-child
//! actions retained in `standards/grammar/sysml-2.0-source.json` (8.2.2).
//! Concrete family declarations share Definition/Usage body lowering, canonical
//! identity allocation, membership ownership, names, multiplicity, and KerML
//! relationship construction. Dispatch-only productions never allocate a second
//! element. Additional actions below concern only family-specific storage.
use super::*;
use agq_sysml::{classes as s, properties as sp};

pub(super) fn sysml_class(node: Node<'_>) -> Result<Option<MetaclassId>, LibraryLoadError> {
    let name = node.kind().name();
    let class = match name {
        "AnnotatingMember"
        | "PackageMember"
        | "DefinitionMember"
        | "OwnedCrossFeatureMember"
        | "MultiplicityExpressionMember"
        | "EmptyMultiplicityMember"
        | "ConjugatedPortDefinitionMember"
        | "OwnedCrossMultiplicityMember"
        | "OwnedFeatureChainMember"
        | "TransitionSuccessionMember"
        | "PrefixMetadataMember" => c::OWNING_MEMBERSHIP,
        "ElementFilterMember" | "FilterPackageMember" => c::ELEMENT_FILTER_MEMBERSHIP,
        "AliasMember" => c::MEMBERSHIP,
        "VariantUsageMember" | "EnumerationUsageMember" => s::VARIANT_MEMBERSHIP,
        "NonOccurrenceUsageMember"
        | "OccurrenceUsageMember"
        | "StructureUsageMember"
        | "BehaviorUsageMember"
        | "SourceSuccessionMember"
        | "InterfaceNonOccurrenceUsageMember"
        | "InterfaceOccurrenceUsageMember"
        | "FlowPayloadFeatureMember"
        | "FlowFeatureMember"
        | "InitialNodeMember"
        | "ActionNodeMember"
        | "ActionTargetSuccessionMember"
        | "GuardedSuccessionMember"
        | "EntryTransitionMember"
        | "TransitionUsageMember"
        | "TargetTransitionUsageMember"
        | "MetadataBodyUsageMember" => c::FEATURE_MEMBERSHIP,
        "OwnedCrossFeature"
        | "DefaultReferenceUsage"
        | "ReferenceUsage"
        | "SourceEnd"
        | "EmptyFeature"
        | "PayloadParameter"
        | "ConnectorEnd"
        | "FlowFeature"
        | "NodeParameter"
        | "EmptyUsage"
        | "AssignmentTargetParameter"
        | "SubjectUsage"
        | "SatisfactionParameter"
        | "MetadataBodyUsage" => s::REFERENCE_USAGE,
        "OwnedFeatureChain" | "PayloadFeature" | "FeatureChainPrefix" => c::FEATURE,
        "AttributeDefinition" => s::ATTRIBUTE_DEFINITION,
        "AttributeUsage" => s::ATTRIBUTE_USAGE,
        "EnumerationDefinition" => s::ENUMERATION_DEFINITION,
        "EnumeratedValue" | "EnumerationUsage" => s::ENUMERATION_USAGE,
        "OccurrenceDefinition" | "IndividualDefinition" => s::OCCURRENCE_DEFINITION,
        "EmptyMultiplicity" => c::MULTIPLICITY,
        "OccurrenceUsage" | "IndividualUsage" | "PortionUsage" => s::OCCURRENCE_USAGE,
        "EventOccurrenceUsage" | "MessageEvent" => s::EVENT_OCCURRENCE_USAGE,
        "SourceSuccession" | "SuccessionAsUsage" | "TargetSuccession" => s::SUCCESSION_AS_USAGE,
        "SourceEndMember" | "ConnectorEndMember" | "InterfaceEndMember" | "FlowEndMember"
        | "EmptyEndMember" => c::END_FEATURE_MEMBERSHIP,
        "ItemDefinition" => s::ITEM_DEFINITION,
        "ItemUsage" => s::ITEM_USAGE,
        "FlowPayloadFeature" => c::PAYLOAD_FEATURE,
        "PartDefinition" => s::PART_DEFINITION,
        "PartUsage" | "ActorUsage" | "StakeholderUsage" => s::PART_USAGE,
        "PortDefinition" => s::PORT_DEFINITION,
        "ConjugatedPortDefinition" => s::CONJUGATED_PORT_DEFINITION,
        "PortConjugation" => s::PORT_CONJUGATION,
        "ConjugatedPortTyping" => s::CONJUGATED_PORT_TYPING,
        "PortUsage" | "DefaultInterfaceEnd" | "InterfaceEnd" => s::PORT_USAGE,
        "ConnectionDefinition" => s::CONNECTION_DEFINITION,
        "ConnectionUsage" => s::CONNECTION_USAGE,
        "BindingConnectorAsUsage" => s::BINDING_CONNECTOR_AS_USAGE,
        "InterfaceDefinition" => s::INTERFACE_DEFINITION,
        "InterfaceUsage" => s::INTERFACE_USAGE,
        "AllocationDefinition" => s::ALLOCATION_DEFINITION,
        "AllocationUsage" => s::ALLOCATION_USAGE,
        "FlowDefinition" => s::FLOW_DEFINITION,
        "Message" | "FlowUsage" => s::FLOW_USAGE,
        "MessageEventMember"
        | "PayloadParameterMember"
        | "ArgumentMember"
        | "ArgumentExpressionMember"
        | "NodeParameterMember"
        | "EmptyParameterMember"
        | "AssignmentTargetMember"
        | "ExpressionParameterMember"
        | "ActionBodyParameterMember"
        | "IfNodeParameterMember" => c::PARAMETER_MEMBERSHIP,
        "SuccessionFlowUsage" => s::SUCCESSION_FLOW_USAGE,
        "FlowEnd" => c::FLOW_END,
        "FlowEndSubsetting" => c::REFERENCE_SUBSETTING,
        "ActionDefinition" => s::ACTION_DEFINITION,
        "ActionUsage" | "ActionBodyParameter" | "EmptyActionUsage" => s::ACTION_USAGE,
        "PerformActionUsage" | "StatePerformActionUsage" | "TransitionPerformActionUsage" => {
            s::PERFORM_ACTION_USAGE
        }
        "MergeNode" => s::MERGE_NODE,
        "DecisionNode" => s::DECISION_NODE,
        "JoinNode" => s::JOIN_NODE,
        "ForkNode" => s::FORK_NODE,
        "AcceptNode"
        | "StateAcceptActionUsage"
        | "TriggerAction"
        | "TransitionAcceptActionUsage" => s::ACCEPT_ACTION_USAGE,
        "TriggerFeatureValue"
        | "FeatureBinding"
        | "AssignmentTargetBinding"
        | "SatisfactionFeatureValue" => c::FEATURE_VALUE,
        "TriggerExpression" => s::TRIGGER_INVOCATION_EXPRESSION,
        "SendNode" | "StateSendActionUsage" | "TransitionSendActionUsage" => s::SEND_ACTION_USAGE,
        "AssignmentNode" | "StateAssignmentActionUsage" | "TransitionAssignmentActionUsage" => {
            s::ASSIGNMENT_ACTION_USAGE
        }
        "TerminateNode" => s::TERMINATE_ACTION_USAGE,
        "IfNode" => s::IF_ACTION_USAGE,
        "WhileLoopNode" => s::WHILE_LOOP_ACTION_USAGE,
        "ForLoopNode" => s::FOR_LOOP_ACTION_USAGE,
        "GuardedTargetSuccession"
        | "DefaultTargetSuccession"
        | "GuardedSuccession"
        | "TransitionUsage"
        | "TargetTransitionUsage" => s::TRANSITION_USAGE,
        "StateDefinition" => s::STATE_DEFINITION,
        "EntryActionMember" | "DoActionMember" | "ExitActionMember" => {
            s::STATE_SUBACTION_MEMBERSHIP
        }
        "StateUsage" => s::STATE_USAGE,
        "ExhibitStateUsage" => s::EXHIBIT_STATE_USAGE,
        "TriggerActionMember" | "GuardExpressionMember" | "EffectBehaviorMember" => {
            s::TRANSITION_FEATURE_MEMBERSHIP
        }
        "TransitionSuccession" => c::SUCCESSION,
        "CalculationDefinition" => s::CALCULATION_DEFINITION,
        "CalculationUsage" => s::CALCULATION_USAGE,
        "ReturnParameterMember" => c::RETURN_PARAMETER_MEMBERSHIP,
        "ResultExpressionMember" => c::RESULT_EXPRESSION_MEMBERSHIP,
        "ConstraintDefinition" => s::CONSTRAINT_DEFINITION,
        "ConstraintUsage" | "RequirementConstraintUsage" => s::CONSTRAINT_USAGE,
        "AssertConstraintUsage" => s::ASSERT_CONSTRAINT_USAGE,
        "RequirementDefinition" => s::REQUIREMENT_DEFINITION,
        "SubjectMember" | "SatisfactionSubjectMember" => s::SUBJECT_MEMBERSHIP,
        "RequirementConstraintMember" => s::REQUIREMENT_CONSTRAINT_MEMBERSHIP,
        "FramedConcernMember" => s::FRAMED_CONCERN_MEMBERSHIP,
        "FramedConcernUsage" | "ConcernUsage" => s::CONCERN_USAGE,
        "ActorMember" => s::ACTOR_MEMBERSHIP,
        "StakeholderMember" => s::STAKEHOLDER_MEMBERSHIP,
        "RequirementUsage" | "ObjectiveRequirementUsage" | "RequirementVerificationUsage" => {
            s::REQUIREMENT_USAGE
        }
        "SatisfyRequirementUsage" => s::SATISFY_REQUIREMENT_USAGE,
        "SatisfactionReferenceExpression" => c::FEATURE_REFERENCE_EXPRESSION,
        "ConcernDefinition" => s::CONCERN_DEFINITION,
        "CaseDefinition" => s::CASE_DEFINITION,
        "CaseUsage" => s::CASE_USAGE,
        "ObjectiveMember" => s::OBJECTIVE_MEMBERSHIP,
        "AnalysisCaseDefinition" => s::ANALYSIS_CASE_DEFINITION,
        "AnalysisCaseUsage" => s::ANALYSIS_CASE_USAGE,
        "VerificationCaseDefinition" => s::VERIFICATION_CASE_DEFINITION,
        "VerificationCaseUsage" => s::VERIFICATION_CASE_USAGE,
        "RequirementVerificationMember" => s::REQUIREMENT_VERIFICATION_MEMBERSHIP,
        "UseCaseDefinition" => s::USE_CASE_DEFINITION,
        "UseCaseUsage" => s::USE_CASE_USAGE,
        "IncludeUseCaseUsage" => s::INCLUDE_USE_CASE_USAGE,
        "ViewDefinition" => s::VIEW_DEFINITION,
        "ViewRenderingMember" => s::VIEW_RENDERING_MEMBERSHIP,
        "ViewRenderingUsage" | "RenderingUsage" => s::RENDERING_USAGE,
        "ViewUsage" => s::VIEW_USAGE,
        "ViewpointDefinition" => s::VIEWPOINT_DEFINITION,
        "ViewpointUsage" => s::VIEWPOINT_USAGE,
        "RenderingDefinition" => s::RENDERING_DEFINITION,
        "MetadataDefinition" => s::METADATA_DEFINITION,
        "PrefixMetadataUsage" | "MetadataUsage" => s::METADATA_USAGE,
        "ExtendedDefinition" => s::DEFINITION,
        "ExtendedUsage" => s::USAGE,
        // SysML's FeatureTyping dispatches to the concrete relationship child.
        "FeatureTyping"
        | "FeatureMember"
        | "ActionBehaviorMember"
        | "ActionTargetSuccession"
        | "StateActionUsage"
        | "EffectBehaviorUsage"
        | "Definition"
        | "DefinitionDeclaration"
        | "DefinitionBody"
        | "DefinitionBodyItem"
        | "DefinitionElement"
        | "Usage"
        | "UsageDeclaration"
        | "UsageCompletion"
        | "UsageBody"
        | "UsageElement"
        | "NonOccurrenceUsageElement"
        | "OccurrenceUsageElement"
        | "StructureUsageElement"
        | "BehaviorUsageElement"
        | "DefinitionPrefix"
        | "BasicDefinitionPrefix"
        | "RefPrefix"
        | "BasicUsagePrefix"
        | "UsagePrefix"
        | "UnextendedUsagePrefix"
        | "EndUsagePrefix"
        | "OccurrenceDefinitionPrefix"
        | "OccurrenceUsagePrefix"
        | "DefinitionExtensionKeyword"
        | "UsageExtensionKeyword" => return Ok(None),
        "FeatureChainMember" if node.child(P::OwnedFeatureChainMember).is_none() => c::MEMBERSHIP,
        "FeatureChainMember" => return Ok(None),
        _ => {
            if let Some(class) = vocabulary::class(node) {
                return Ok(Some(class));
            }
            if name.ends_with("Definition")
                || name.ends_with("Usage")
                || name.ends_with("Member")
                || name == "ConjugatedPortTyping"
            {
                return Err(LibraryLoadError::Interpretation(format!(
                    "SysML canonical production {name} at {:?}",
                    node.range()
                )));
            }
            return Ok(None);
        }
    };
    Ok(Some(class))
}

impl Builder {
    pub(super) fn usage_composite_default(
        &mut self,
        id: ElementId,
        membership: ElementId,
    ) -> Result<(), LibraryLoadError> {
        let class = self.records[&id].class;
        let namespace = self.parents.get(&membership).copied();
        let composite = self.is_class(self.records[&membership].class, c::FEATURE_MEMBERSHIP)
            && !self.is_class(self.records[&membership].class, c::PARAMETER_MEMBERSHIP)
            && !self.is_class(class, s::REFERENCE_USAGE)
            && !self.is_class(class, s::ATTRIBUTE_USAGE)
            && !self.is_class(class, s::EVENT_OCCURRENCE_USAGE)
            && namespace.is_some_and(|owner| {
                let class = self.records[&owner].class;
                self.is_class(class, s::OCCURRENCE_DEFINITION)
                    || self.is_class(class, s::OCCURRENCE_USAGE)
            });
        self.set(id, p::FEATURE_IS_COMPOSITE, Value::Boolean(composite))
    }

    pub(super) fn interpret_sysml(
        &mut self,
        node: Node<'_>,
        id: ElementId,
        job: &mut Job<'_>,
    ) -> Result<(), LibraryLoadError> {
        let name = node.kind().name();
        let class = self.records[&id].class;
        if matches!(
            name,
            "BasicDefinitionPrefix"
                | "RefPrefix"
                | "BasicUsagePrefix"
                | "EndUsagePrefix"
                | "OccurrenceDefinitionPrefix"
                | "OccurrenceUsagePrefix"
        ) {
            // An end prefix can own a separate cross Feature. Its modifiers
            // belong to that Feature, never to the surrounding Usage.
            let cross_ranges: Vec<_> = node
                .descendants()
                .filter(|child| child.kind() == P::OwnedCrossFeatureMember)
                .map(|child| child.range())
                .collect();
            let words: Vec<_> = node
                .tokens()
                .filter(|token| {
                    !token.kind.is_trivia()
                        && !cross_ranges.iter().any(|range| {
                            range.start() <= token.range.start() && token.range.end() <= range.end()
                        })
                })
                .map(|token| node_text(node, token.range))
                .collect();
            for (word, property) in [
                ("abstract", p::TYPE_IS_ABSTRACT),
                ("derived", p::FEATURE_IS_DERIVED),
                ("constant", p::FEATURE_IS_CONSTANT),
                ("end", p::FEATURE_IS_END),
            ] {
                if words.contains(&word) {
                    self.set(id, property, Value::Boolean(true))?;
                }
            }
            if words
                .iter()
                .any(|word| matches!(*word, "ref" | "in" | "out" | "inout" | "end"))
                && self.is_class(class, s::USAGE)
            {
                self.set(id, p::FEATURE_IS_COMPOSITE, Value::Boolean(false))?;
            }
            for (word, base, property) in [
                ("variation", s::DEFINITION, sp::DEFINITION_IS_VARIATION),
                ("variation", s::USAGE, sp::USAGE_IS_VARIATION),
                (
                    "individual",
                    s::OCCURRENCE_DEFINITION,
                    sp::OCCURRENCE_DEFINITION_IS_INDIVIDUAL,
                ),
                (
                    "individual",
                    s::OCCURRENCE_USAGE,
                    sp::OCCURRENCE_USAGE_IS_INDIVIDUAL,
                ),
            ] {
                if words.contains(&word) && self.is_class(class, base) {
                    self.set(id, property, Value::Boolean(true))?;
                }
            }
            if self.is_class(class, s::OCCURRENCE_USAGE) {
                for portion in ["snapshot", "timeslice"] {
                    if words.contains(&portion) {
                        self.set(id, p::FEATURE_IS_PORTION, Value::Boolean(true))?;
                        self.enumeration(id, sp::OCCURRENCE_USAGE_PORTION_KIND, portion)?;
                    }
                }
            }
        }
        match name {
            "PortConjugation" => job.target = Some(sp::PORT_CONJUGATION_ORIGINAL_PORT_DEFINITION),
            "ConjugatedPortTyping" => job.target = Some(sp::CONJUGATED_PORT_TYPING_PORT_DEFINITION),
            "InitialNodeMember" => job.target = Some(p::MEMBERSHIP_MEMBER_ELEMENT),
            "EntryActionMember" | "DoActionMember" | "ExitActionMember" => {
                self.enumeration(
                    id,
                    sp::STATE_SUBACTION_MEMBERSHIP_KIND,
                    match name {
                        "EntryActionMember" => "entry",
                        "DoActionMember" => "do",
                        _ => "exit",
                    },
                )?;
            }
            "TriggerActionMember" | "GuardExpressionMember" | "EffectBehaviorMember" => {
                self.enumeration(
                    id,
                    sp::TRANSITION_FEATURE_MEMBERSHIP_KIND,
                    match name {
                        "TriggerActionMember" => "trigger",
                        "GuardExpressionMember" => "guard",
                        _ => "effect",
                    },
                )?;
            }
            "RequirementKind" => self.enumeration(
                id,
                sp::REQUIREMENT_CONSTRAINT_MEMBERSHIP_KIND,
                if node.text() == "assume" {
                    "assumption"
                } else {
                    "requirement"
                },
            )?,
            "RequirementVerificationMember" => self.enumeration(
                id,
                sp::REQUIREMENT_VERIFICATION_MEMBERSHIP_KIND,
                "requirement",
            )?,
            "DefaultInterfaceEnd" => self.set(id, p::FEATURE_IS_END, Value::Boolean(true))?,
            "StateDefBody" | "StateUsageBody" => {
                let parallel = node
                    .tokens()
                    .find(|token| !token.kind.is_trivia())
                    .is_some_and(|token| node_text(node, token.range) == "parallel");
                self.set(
                    id,
                    if self.is_class(class, s::STATE_DEFINITION) {
                        sp::STATE_DEFINITION_IS_PARALLEL
                    } else {
                        sp::STATE_USAGE_IS_PARALLEL
                    },
                    Value::Boolean(parallel),
                )?;
            }
            "AssertConstraintUsage" | "SatisfyRequirementUsage" => {
                let negated = node
                    .tokens()
                    .filter(|token| !token.kind.is_trivia())
                    .map(|token| node_text(node, token.range))
                    .take_while(|word| !matches!(*word, "constraint" | "satisfy"))
                    .any(|word| word == "not");
                self.set(id, p::INVARIANT_IS_NEGATED, Value::Boolean(negated))?;
            }
            "MetadataDefinition" => {
                let abstract_ = node
                    .tokens()
                    .find(|token| !token.kind.is_trivia())
                    .is_some_and(|token| node_text(node, token.range) == "abstract");
                self.set(id, p::TYPE_IS_ABSTRACT, Value::Boolean(abstract_))?;
            }
            _ => {}
        }
        Ok(())
    }

    pub(super) fn sysml_structural_completion(
        &mut self,
        inputs: &[SourceInput<'_>],
    ) -> Result<(), LibraryLoadError> {
        let sysml_documents: std::collections::BTreeSet<_> = inputs
            .iter()
            .filter(|input| input.sysml)
            .map(|input| input.syntax.document())
            .collect();
        let ids: Vec<_> = self
            .order
            .iter()
            .copied()
            .filter(|id| {
                self.records[id]
                    .source
                    .as_ref()
                    .is_some_and(|source| sysml_documents.contains(&source.document))
            })
            .collect();
        for id in ids {
            let class = self.records[&id].class;
            if self.is_class(class, s::ENUMERATION_DEFINITION) {
                // An enum is a variation by construction. The pinned
                // validateDefinitionVariationIsAbstract obligation applies to
                // it as well; its textual production has no abstract modifier.
                self.set(id, p::TYPE_IS_ABSTRACT, Value::Boolean(true))?;
            }
            for (base, property) in [
                (s::DEFINITION, sp::DEFINITION_IS_VARIATION),
                (s::USAGE, sp::USAGE_IS_VARIATION),
                (
                    s::OCCURRENCE_DEFINITION,
                    sp::OCCURRENCE_DEFINITION_IS_INDIVIDUAL,
                ),
                (s::OCCURRENCE_USAGE, sp::OCCURRENCE_USAGE_IS_INDIVIDUAL),
            ] {
                if self.is_class(class, base) {
                    let descriptor = self
                        .base
                        .model()
                        .registry()
                        .resolve_property(class, property)
                        .map_err(agq_kernel::ModelError::from)?
                        .expect("SysML base property");
                    if !descriptor.derived && !self.records[&id].slots.contains_key(&descriptor.id)
                    {
                        // SysML 2.0 EnumerationDefinition::isVariation is a
                        // nonderived redefinition with the explicit default
                        // true (8.3.8.2), also required by its validation rule.
                        // This source-backed construction default is distinct
                        // from the false default on an ordinary Definition.
                        let value = property == sp::DEFINITION_IS_VARIATION
                            && self.is_class(class, s::ENUMERATION_DEFINITION);
                        self.set(id, property, Value::Boolean(value))?;
                    }
                }
            }
            if class == s::PORT_CONJUGATION {
                let conjugated = self.parents.get(&id).copied().ok_or_else(|| {
                    LibraryLoadError::Interpretation(
                        "PortConjugation requires its conjugated definition".into(),
                    )
                })?;
                let membership = self.parents.get(&conjugated).copied().ok_or_else(|| {
                    LibraryLoadError::Interpretation(
                        "ConjugatedPortDefinition requires ownership".into(),
                    )
                })?;
                let original = self.parents.get(&membership).copied().ok_or_else(|| {
                    LibraryLoadError::Interpretation(
                        "ConjugatedPortDefinition requires original port".into(),
                    )
                })?;
                if self.records[&conjugated].class != s::CONJUGATED_PORT_DEFINITION
                    || self.records[&original].class != s::PORT_DEFINITION
                {
                    return Err(LibraryLoadError::Interpretation(
                        "Invalid syntactic port conjugation ownership".into(),
                    ));
                }
                self.set(
                    id,
                    sp::PORT_CONJUGATION_ORIGINAL_PORT_DEFINITION,
                    Value::Reference(original),
                )?;
            }
        }
        Ok(())
    }
}
