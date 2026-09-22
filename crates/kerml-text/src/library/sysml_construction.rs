//! Bounded SysML grammar actions on the shared canonical transaction builder.
use super::*;
use agq_sysml::{classes as s, properties as sp};

pub(super) fn sysml_class(node: Node<'_>) -> Result<Option<MetaclassId>, LibraryLoadError> {
    let name = node.kind().name();
    let class = match name {
        "AttributeDefinition" => s::ATTRIBUTE_DEFINITION,
        "AttributeUsage" => s::ATTRIBUTE_USAGE,
        "ItemDefinition" => s::ITEM_DEFINITION,
        "ItemUsage" => s::ITEM_USAGE,
        "PartDefinition" => s::PART_DEFINITION,
        "PartUsage" => s::PART_USAGE,
        "PortDefinition" => s::PORT_DEFINITION,
        "PortUsage" => s::PORT_USAGE,
        "ConjugatedPortDefinition" => s::CONJUGATED_PORT_DEFINITION,
        "PortConjugation" => s::PORT_CONJUGATION,
        "DefaultReferenceUsage" | "ReferenceUsage" | "OwnedCrossFeature" => s::REFERENCE_USAGE,
        "PackageMember" | "DefinitionMember" | "ConjugatedPortDefinitionMember" => {
            c::OWNING_MEMBERSHIP
        }
        "NonOccurrenceUsageMember"
        | "OccurrenceUsageMember"
        | "StructureUsageMember"
        | "BehaviorUsageMember" => c::FEATURE_MEMBERSHIP,
        "OwnedFeatureChain" => c::FEATURE,
        // SysML's FeatureTyping dispatches to the concrete relationship child.
        "FeatureTyping"
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
            && !self.is_class(class, s::REFERENCE_USAGE)
            && !self.is_class(class, s::ATTRIBUTE_USAGE)
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
            let words: Vec<_> = node
                .tokens()
                .filter(|token| !token.kind.is_trivia())
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
            if words
                .iter()
                .any(|word| matches!(*word, "variation" | "individual" | "snapshot" | "timeslice"))
            {
                return Err(LibraryLoadError::Interpretation(format!(
                    "SysML modifier outside the ordinary Definition/Usage slice: {}",
                    node.text()
                )));
            }
        }
        if name == "PortConjugation" {
            job.target = Some(sp::PORT_CONJUGATION_ORIGINAL_PORT_DEFINITION);
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
                    self.set(id, property, Value::Boolean(false))?;
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
