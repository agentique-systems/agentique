use crate::{AgentContext, AgentError, AgentPolicy, Authority};
use agq_kerml_syntax::{TextEdit, TokenKind, production};
use agq_kerml_text::ProjectChange;
use agq_kernel::{ElementId, provenance::ByteRange};
use agq_modeling_repository::{CommitReceipt, OperationId};
use agq_modeling_service::{
    ApplyDocumentChanges, ModelingService, PreparedChanges, RevisionSelector,
};
use serde::{Deserialize, Serialize};

/// Typed intent vocabulary. Only CreatePartUsage has a reviewed source mapping in v1.
/// Unsupported operations fail before constructing any candidate.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum ModelCommand {
    CreatePartUsage {
        owner: ElementId,
        name: String,
        definition: Option<ElementId>,
    },
    CreatePortUsage {
        owner: ElementId,
        name: String,
    },
    CreateConnection {
        source: ElementId,
        target: ElementId,
    },
    ChangeDefinition {
        element: ElementId,
        definition: ElementId,
    },
    RenameElement {
        element: ElementId,
        name: String,
    },
    MoveElement {
        element: ElementId,
        owner: ElementId,
    },
    DeleteElement {
        element: ElementId,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourcePreview {
    pub path: String,
    pub before: String,
    pub after: String,
}

/// Candidate retains the exact service operation for durable CAS and retry.
pub struct AgentCandidate {
    pub context: AgentContext,
    pub actor: String,
    pub command: ModelCommand,
    pub source_preview: SourcePreview,
    prepared: PreparedChanges,
}
impl AgentCandidate {
    pub fn prepared(&self) -> &PreparedChanges {
        &self.prepared
    }
    pub fn validate(&mut self, policy: &AgentPolicy) -> Result<(), AgentError> {
        policy.require(Authority::Validate)?;
        // Replace only after successful validation; failure retains the Working candidate.
        let validated = self.prepared.validate()?;
        self.prepared = validated;
        Ok(())
    }
    pub fn commit(
        &self,
        service: &ModelingService,
        policy: &AgentPolicy,
    ) -> Result<CommitReceipt, AgentError> {
        policy.require(Authority::Commit)?;
        if self.prepared.bound_revision().validated().is_none() {
            return Err(AgentError::Invalid(
                "validate this candidate before operator commit".into(),
            ));
        }
        Ok(service.commit_prepared(&self.prepared)?)
    }
}

/// Construct a Working revision from semantic intent and exact authored source.
pub fn propose(
    service: &ModelingService,
    policy: &AgentPolicy,
    context: AgentContext,
    command: ModelCommand,
) -> Result<AgentCandidate, AgentError> {
    policy.require(Authority::Read)?;
    policy.require(Authority::Propose)?;
    let ModelCommand::CreatePartUsage {
        owner,
        name,
        definition,
    } = &command
    else {
        return Err(AgentError::Invalid(
            "this command has no reviewed source mapping in Studio v1".into(),
        ));
    };
    let bound = service.resolve(
        context.project,
        RevisionSelector::Revision(context.revision),
    )?;
    let element = bound.current_element(*owner)?;
    if !matches!(
        element.metaclass_name.as_str(),
        "PartDefinition" | "PartUsage"
    ) {
        return Err(AgentError::Invalid(
            "nested parts require an authored PartDefinition or PartUsage".into(),
        ));
    }
    let origin = element
        .source
        .ok_or_else(|| AgentError::Invalid("owner has no authored source".into()))?;
    let (path, document) = bound
        .revision()
        .documents()
        .find(|(_, d)| d.id() == origin.document)
        .ok_or_else(|| {
            AgentError::Invalid("standard-library and generated records are read-only".into())
        })?;
    if document.revision() != origin.revision {
        return Err(AgentError::Invalid(
            "source provenance belongs to another revision".into(),
        ));
    }
    let syntax = document
        .production_syntax()
        .ok_or_else(|| AgentError::Invalid("lossless source syntax unavailable".into()))?;
    let owner_node = syntax
        .nodes()
        .find(|node| Some(node.id()) == origin.syntax_node)
        .ok_or_else(|| {
            AgentError::Invalid("owner declaration cannot be reconciled to source".into())
        })?;
    let type_name = definition
        .map(|id| {
            let target = bound.current_element(id)?;
            if target.metaclass_name != "PartDefinition" {
                return Err(AgentError::Invalid(
                    "definition must be a PartDefinition".into(),
                ));
            }
            target.declared_qualified_name.ok_or_else(|| {
                AgentError::Invalid("definition has no resolvable qualified name".into())
            })
        })
        .transpose()?;
    let (edit, inserted_declaration) =
        nested_part_edit(syntax, owner_node.range(), name, type_name.as_deref())?;
    let before = document.source().to_string();
    let mut after = before.clone();
    after.replace_range(
        edit.range.start() as usize..edit.range.end() as usize,
        &edit.replacement,
    );
    let source_preview = SourcePreview {
        path: path.into(),
        before,
        after,
    };
    let prepared = service.prepare_changes(ApplyDocumentChanges {
        operation_id: OperationId::new(),
        project: context.project,
        branch: context.branch,
        expected_head: context.revision,
        changes: vec![ProjectChange::Edit {
            document: origin.document,
            edit,
        }],
        validate: false,
    })?;
    if let Some(definition) = definition {
        let candidate = prepared.revision();
        let document = candidate
            .document(origin.document)
            .ok_or_else(|| AgentError::Invalid("candidate lost its authored document".into()))?;
        let syntax = document
            .production_syntax()
            .ok_or_else(|| AgentError::Invalid("candidate source syntax unavailable".into()))?;
        let node = inserted_part_node(syntax, inserted_declaration)?;
        let model = candidate
            .semantic_model()
            .ok_or_else(|| AgentError::Invalid("candidate semantic graph unavailable".into()))?;
        let usages: Vec<_> = model
            .elements()
            .filter(|record| {
                record.metaclass() == agq_sysml::classes::PART_USAGE
                    && candidate
                        .source_for_fact(agq_kernel::provenance::FactKey::Element(record.id()))
                        .is_some_and(|source| {
                            source.document == document.id()
                                && source.revision == document.revision()
                                && source.syntax_node == Some(node)
                        })
            })
            .map(|record| record.id())
            .collect();
        let [usage] = usages.as_slice() else {
            return Err(AgentError::Invalid(
                "inserted part has no unique canonical source identity".into(),
            ));
        };
        verify_selected_definition(model, *usage, *definition)?;
    }
    Ok(AgentCandidate {
        context,
        actor: policy.actor.clone(),
        command,
        source_preview,
        prepared,
    })
}

fn inserted_part_node(
    syntax: &production::Document,
    inserted: ByteRange,
) -> Result<agq_kernel::SyntaxNodeId, AgentError> {
    syntax
        .nodes()
        .filter(|node| {
            node.kind().name() == "PartUsage"
                && node.range().start() <= inserted.start()
                && node.range().end() == inserted.end()
        })
        .min_by_key(|node| node.range().end() - node.range().start())
        .map(|node| node.id())
        .ok_or_else(|| {
            AgentError::Invalid("inserted declaration is not a PartUsage syntax node".into())
        })
}

fn verify_selected_definition(
    model: &agq_kernel::ModelView,
    usage: ElementId,
    selected: ElementId,
) -> Result<(), AgentError> {
    use agq_kerml::{classes, properties};
    use agq_kernel::{provenance::Origin, value::Value};
    let property = |record: &agq_kernel::ElementRecord, id| {
        model
            .registry()
            .resolve_property(record.metaclass(), id)
            .ok()
            .flatten()
            .and_then(|property| model.navigation_slot(record.id(), property.id))
    };
    let targets: std::collections::BTreeSet<_> = model
        .elements()
        .filter(|record| {
            matches!(record.origin(), Origin::Declared(_))
                && model
                    .registry()
                    .is_subtype(record.metaclass(), classes::FEATURE_TYPING)
                    .unwrap_or(false)
                && property(record, properties::SPECIALIZATION_SPECIFIC).is_some_and(|slot| {
                    slot.value()
                        .values()
                        .any(|value| *value == Value::Reference(usage))
                })
        })
        .flat_map(|record| {
            property(record, properties::SPECIALIZATION_GENERAL)
                .into_iter()
                .flat_map(|slot| slot.value().values())
                .filter_map(|value| match value {
                    Value::Reference(id) => Some(*id),
                    _ => None,
                })
        })
        .collect();
    if targets != std::collections::BTreeSet::from([selected]) {
        return Err(AgentError::Invalid("candidate typing does not resolve to the selected definition; its qualified name may be shadowed or unresolved".into()));
    }
    Ok(())
}

fn identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && name.len() <= 120
}

fn nested_part_edit(
    syntax: &production::Document,
    owner: ByteRange,
    name: &str,
    definition: Option<&str>,
) -> Result<(TextEdit, ByteRange), AgentError> {
    if !identifier(name) || definition.is_some_and(|name| !name.split("::").all(identifier)) {
        return Err(AgentError::Invalid(
            "use a simple identifier and a qualified definition name".into(),
        ));
    }
    let source = syntax.source();
    let tokens: Vec<_> = syntax
        .tokens()
        .iter()
        .filter(|token| token.range.start() >= owner.start() && token.range.end() <= owner.end())
        .filter(|token| {
            !matches!(
                token.kind,
                TokenKind::Whitespace | TokenKind::Note | TokenKind::Comment
            )
        })
        .collect();
    if tokens.iter().any(|token| syntax.token_text(token) == name) {
        return Err(AgentError::Invalid(
            "name already occurs in the selected declaration; choose a distinct part name".into(),
        ));
    }
    let last = tokens
        .last()
        .ok_or_else(|| AgentError::Invalid("empty source declaration".into()))?;
    let suffix = definition
        .map(|name| format!(" : {name}"))
        .unwrap_or_default();
    let newline = if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let line_start = source[..owner.start() as usize]
        .rfind('\n')
        .map_or(0, |n| n + 1);
    let indent: String = source[line_start..owner.start() as usize]
        .chars()
        .take_while(|c| matches!(c, ' ' | '\t'))
        .collect();
    let declaration = format!("{indent}    part {name}{suffix};");
    let (range, replacement) = match syntax.token_text(last) {
        "}" => (
            ByteRange::new(last.range.start(), last.range.start()).unwrap(),
            format!("{newline}{declaration}{newline}{indent}"),
        ),
        ";" => (
            last.range,
            format!(" {{{newline}{declaration}{newline}{indent}}}"),
        ),
        _ => {
            return Err(AgentError::Invalid(
                "owner must have a complete body or semicolon".into(),
            ));
        }
    };
    // Parser validates keyword/identifier legality; it does not establish semantic acceptance.
    let probe = format!("part def StudioCommandProbe {{ part {name}{suffix}; }}");
    let parsed = production::parse_sysml_with_profile(
        production::SysmlSyntaxProfile::OperationalV3,
        syntax.document(),
        syntax.revision(),
        probe,
        production::Limits::default(),
    )
    .map_err(|error| AgentError::Invalid(error.to_string()))?;
    if !parsed.is_complete() {
        return Err(AgentError::Invalid(
            "part declaration is not valid pinned SysML syntax".into(),
        ));
    }
    let text = format!("part {name}{suffix};");
    let start = range.start() + replacement.find(&text).expect("generated part declaration") as u64;
    let inserted =
        ByteRange::new(start, start + text.len() as u64).expect("generated declaration range");
    Ok((TextEdit { range, replacement }, inserted))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn apply(source: &str, name: &str) -> Result<String, AgentError> {
        let syntax = production::parse_sysml_with_profile(
            production::SysmlSyntaxProfile::OperationalV3,
            agq_kernel::DocumentId::new(),
            agq_kernel::SourceRevisionId::new(),
            source.to_owned(),
            production::Limits::default(),
        )
        .unwrap();
        let (edit, inserted) = nested_part_edit(
            &syntax,
            ByteRange::new(0, source.len() as u64).unwrap(),
            name,
            None,
        )?;
        let next = syntax.edit(&edit, production::Limits::default()).unwrap();
        assert!(next.is_complete());
        assert!(inserted_part_node(&next, inserted).is_ok());
        Ok(next.source().into())
    }
    #[test]
    fn nested_source_command_ignores_comment_braces_and_preserves_bytes() {
        let source = "part def Platform {\r\n    // } not the body end\r\n    part existing;\r\n}";
        let edited = apply(source, "observer").unwrap();
        assert!(edited.contains("// } not the body end\r\n    part existing;"));
        assert!(edited.contains("part observer;\r\n}"));
        assert!(apply(source, "existing").is_err());
        assert!(apply(source, "injected; part bad").is_err());
        assert!(apply(source, "part").is_err());
    }
    #[test]
    fn semicolon_owner_becomes_body() {
        assert!(
            apply("part component;", "child")
                .unwrap()
                .contains("part component {\n    part child;\n}")
        );
    }

    #[test]
    fn command_uses_selected_declaration_range_inside_a_package() {
        let source =
            "package Systems {\n    part def Platform { part inside; }\n    part def Sibling;\n}";
        let syntax = production::parse_sysml_with_profile(
            production::SysmlSyntaxProfile::OperationalV3,
            agq_kernel::DocumentId::new(),
            agq_kernel::SourceRevisionId::new(),
            source,
            production::Limits::default(),
        )
        .unwrap();
        let owner = syntax
            .nodes()
            .find(|node| {
                node.kind().name() == "PartDefinition"
                    && source[node.range().start() as usize..node.range().end() as usize]
                        .contains("Platform")
            })
            .unwrap();
        let (edit, inserted) = nested_part_edit(&syntax, owner.range(), "observer", None).unwrap();
        assert!(
            edit.range.start() >= owner.range().start() && edit.range.end() <= owner.range().end()
        );
        let next = syntax.edit(&edit, production::Limits::default()).unwrap();
        assert!(next.is_complete());
        let node = inserted_part_node(&next, inserted).unwrap();
        let created = next
            .nodes()
            .find(|candidate| candidate.id() == node)
            .unwrap();
        assert_eq!(
            next.source()[created.range().start() as usize..created.range().end() as usize].trim(),
            "part observer;"
        );
        assert!(next.source().ends_with("\n    part def Sibling;\n}"));
        assert_eq!(
            &next.source()[..edit.range.start() as usize],
            &source[..edit.range.start() as usize]
        );
    }

    #[test]
    fn selected_definition_check_rejects_a_valid_but_different_canonical_target() {
        use agq_kerml::{classes, properties};
        use agq_kernel::{
            Snapshot,
            provenance::DeclaredOrigin,
            value::{SlotValue, Value},
        };
        let registry = std::sync::Arc::new(
            agq_sysml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V9).unwrap(),
        );
        let base = Snapshot::new(registry.clone());
        let usage = ElementId::new();
        let selected = ElementId::new();
        let shadow = ElementId::new();
        let typing = ElementId::new();
        let origin = DeclaredOrigin::Authored { source: None };
        let mut changes = base.change_set();
        changes.create(usage, agq_sysml::classes::PART_USAGE, origin.clone());
        changes.create(
            selected,
            agq_sysml::classes::PART_DEFINITION,
            origin.clone(),
        );
        changes.create(shadow, agq_sysml::classes::PART_DEFINITION, origin.clone());
        changes.create(typing, classes::FEATURE_TYPING, origin.clone());
        for (property, target) in [
            (properties::SPECIALIZATION_SPECIFIC, usage),
            (properties::SPECIALIZATION_GENERAL, shadow),
        ] {
            let property = registry
                .resolve_property(classes::FEATURE_TYPING, property)
                .unwrap()
                .unwrap()
                .id;
            changes.set(
                typing,
                property,
                SlotValue::Scalar(Value::Reference(target)),
                origin.clone(),
            );
        }
        let candidate = base.preview(&changes).unwrap();
        assert!(verify_selected_definition(candidate.model(), usage, selected).is_err());
        assert!(verify_selected_definition(candidate.model(), usage, shadow).is_ok());
        assert!(verify_selected_definition(candidate.model(), ElementId::new(), shadow).is_err());
    }
}
