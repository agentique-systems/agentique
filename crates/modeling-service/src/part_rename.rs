//! Bounded declared-name rename; identity mapping is derived by the service.
use super::{
    source_identity::{self, Mutation, mapped_range},
    *,
};
use agq_kerml::properties;
use agq_kerml_semantics::{Completeness, MemberAccess};
use agq_kerml_syntax::{
    TextEdit, TokenKind,
    production::{self, NodeIdentity, Production as P},
};
use agq_kernel::{
    ElementId, SyntaxNodeId,
    provenance::{ByteRange, DeclaredOrigin, Origin},
};
use std::collections::BTreeSet;

const POLICY: &str = "agentique-source-identity/part-rename/1";

fn invalid(reason: &str) -> ServiceError {
    ServiceError::Invalid(format!("Part rename identity proof: {reason}"))
}

/// Rename intent. Callers cannot provide text edits, identity maps or checkpoints.
pub struct RenamePart {
    /// Idempotency key retained with the prepared candidate.
    pub operation_id: OperationId,
    /// Existing durable project containing the selected authored part.
    pub project: ProjectId,
    /// Branch whose exact current Validated head is the predecessor.
    pub branch: BranchId,
    /// Explicit revision observed by the operator; durable commit uses CAS.
    pub expected_head: ProjectRevisionId,
    /// Canonical identity retained by the service-proven source token edit.
    pub element: ElementId,
    /// New plain declared name; no source syntax or identity mapping is accepted.
    pub name: String,
    /// Run ordinary validation before returning; false returns a Working candidate.
    pub validate: bool,
}

impl ModelingService {
    /// Rename one plain authored PartDefinition/PartUsage name through full
    /// reconstruction. References are never rewritten: a changed binding refuses
    /// the candidate. Validation and commit retain their ordinary authority/CAS.
    pub fn prepare_part_rename(
        &self,
        command: RenamePart,
    ) -> Result<PreparedChanges, ServiceError> {
        let branch = self
            .repository
            .get_branch(command.project, command.branch)?;
        if branch.head != command.expected_head {
            return Err(RepositoryError::Conflict {
                expected: command.expected_head,
                actual: branch.head,
            }
            .into());
        }
        let base = self.resolve(
            command.project,
            RevisionSelector::Revision(command.expected_head),
        )?;
        if base.validated().is_none() {
            return Err(invalid("the predecessor must be Validated"));
        }
        let target = base.current_element(command.element)?;
        if !matches!(
            target.metaclass_name.as_str(),
            "PartDefinition" | "PartUsage"
        ) {
            return Err(invalid(
                "only an authored PartDefinition or PartUsage can be renamed",
            ));
        }
        let origin = target
            .source
            .ok_or_else(|| invalid("element has no authored source"))?;
        let document = base
            .revision()
            .document(origin.document)
            .ok_or_else(|| invalid("standard objects are read-only"))?;
        if document.revision() != origin.revision {
            return Err(invalid("source provenance is stale"));
        }
        let syntax = document
            .production_syntax()
            .ok_or_else(|| invalid("lossless source syntax is unavailable"))?;
        let selected = syntax
            .nodes()
            .find(|node| Some(node.id()) == origin.syntax_node)
            .filter(|node| {
                node.range() == origin.range && node.kind().name() == target.metaclass_name
            })
            .ok_or_else(|| invalid("declaration provenance does not match"))?;
        let proof = prove_rename(syntax, selected.id(), &command.name)?;
        let model = base
            .revision()
            .strict_snapshot()
            .ok_or_else(|| invalid("declared graph unavailable"))?
            .model();
        let record = model
            .element(command.element)
            .ok_or_else(|| invalid("selected canonical element is missing"))?;
        if !matches!(
            record.origin(),
            Origin::Declared(DeclaredOrigin::Authored { source: Some(_) })
        ) {
            return Err(invalid("selected part must be a declared authored element"));
        }
        if record
            .slot(properties::ELEMENT_DECLARED_SHORT_NAME)
            .is_some_and(|slot| slot.value().values().next().is_some())
        {
            return Err(invalid("short-name declarations are not supported"));
        }
        let queries = base
            .revision()
            .kerml_queries()
            .map_err(|error| invalid(&format!("name query: {error:?}")))?;
        let owner = queries.owner(command.element);
        if owner.completeness != Completeness::Complete {
            return Err(invalid("owner query is incomplete"));
        }
        let owner = owner
            .value
            .ok_or_else(|| invalid("selected part has no semantic owner"))?;
        let owned = queries.memberships(owner);
        let named = queries.lookup_member(owner, &command.name, MemberAccess::All);
        if owned.completeness != Completeness::Complete
            || named.completeness != Completeness::Complete
        {
            return Err(invalid(
                "direct member names could not be established completely",
            ));
        }
        if named.value.iter().any(|member| {
            member.element != command.element && owned.value.contains(&member.membership)
        }) {
            return Err(invalid("a direct member already has this name"));
        }
        let retained = proof.identities.len();
        let (working, checkpoint) =
            source_identity::reconstruct(base.revision(), &proof.parsed, proof.identities)?;
        source_identity::verify_existing(
            base.revision(),
            &working,
            Mutation::Rename {
                element: command.element,
                before: &proof.before_name,
                after: &command.name,
            },
        )?;
        verify_effective_rename(base.revision(), &working, command.element)?;
        let evidence = serde_json::json!({
            "policy": POLICY,
            "mode": "full-source-reconstruction",
            "base_revision": command.expected_head,
            "element": command.element,
            "owner": owner,
            "before_name": proof.before_name,
            "after_name": command.name,
            "document": origin.document,
            "before_source_revision": origin.revision,
            "after_source_revision": proof.parsed.revision(),
            "before_source_sha256": ContentDigest::of(document.source().as_bytes()),
            "after_source_sha256": ContentDigest::of(proof.parsed.source().as_bytes()),
            "edit_range": proof.edit.range,
            "replacement_sha256": ContentDigest::of(proof.edit.replacement.as_bytes()),
            "retained_syntax_nodes": retained,
            "added_syntax_nodes": 0,
            "removed_syntax_nodes": 0,
            "before_arena_sha256": ContentDigest::of(&serde_json::to_vec(&syntax.identity_checkpoint())?),
            "after_arena_sha256": ContentDigest::of(&serde_json::to_vec(&checkpoint.source.documents.iter().find(|saved| saved.document_id == origin.document).expect("checked source").syntax_nodes)?),
            "retired_identity_reservations_preserved": true,
            "previous_reference_targets_preserved": true,
            "effective_structure_preserved": true,
        });
        let mut candidate = prepare_candidate(&working, command.validate)?;
        candidate.manifest.metadata.name =
            Some(format!("Rename {} to {}", proof.before_name, command.name));
        candidate.manifest.metadata.description = Some(serde_json::to_string(&evidence)?);
        candidate.manifest.metadata.alias.push(POLICY.into());
        candidate.verify()?;
        let validated = if command.validate {
            Some(working.validate()?)
        } else {
            None
        };
        Ok(PreparedChanges {
            request: CommitRevision {
                operation_id: command.operation_id,
                project_id: command.project,
                branch_id: command.branch,
                expected_head: command.expected_head,
                candidate,
            },
            working,
            validated,
        })
    }
}

struct RenameProof {
    parsed: production::Document,
    identities: Vec<NodeIdentity>,
    before_name: String,
    edit: TextEdit,
}

fn plain_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
        && name.len() <= 120
}

/// Accept only `part [def] Name [ : Qualified::Type ] { ... }` / `;`.
/// Bodies remain byte-identical and may contain other supported declarations.
fn name_token(
    before: &production::Document,
    selected: SyntaxNodeId,
) -> Result<(ByteRange, String), ServiceError> {
    let node = before
        .nodes()
        .find(|node| node.id() == selected)
        .filter(|node| matches!(node.kind(), P::PartDefinition | P::PartUsage))
        .ok_or_else(|| invalid("selected syntax is not a Part declaration"))?;
    let tokens: Vec<_> = node
        .tokens()
        .filter(|token| !token.kind.is_trivia() && token.kind != TokenKind::Comment)
        .collect();
    let prefix: &[&str] = if node.kind() == P::PartDefinition {
        &["part", "def"]
    } else {
        &["part"]
    };
    if tokens.len() < prefix.len() + 2
        || !prefix
            .iter()
            .zip(&tokens)
            .all(|(expected, token)| *expected == before.token_text(token))
    {
        return Err(invalid(
            "only a plain part header is supported; modifiers and complex headers require source editing",
        ));
    }
    let token = tokens[prefix.len()];
    let name = before.token_text(token);
    if token.kind != TokenKind::Word
        || !plain_identifier(name)
        || !node
            .names()
            .any(|declared| declared.range == token.range && declared.value == name)
    {
        return Err(invalid(
            "a plain declared name is required; short, quoted and Unicode names are unsupported",
        ));
    }
    let tail = &tokens[prefix.len() + 1..];
    let boundary = tail
        .iter()
        .position(|token| matches!(before.token_text(token), "{" | ";"))
        .ok_or_else(|| invalid("part body boundary is unavailable"))?;
    let header = &tail[..boundary];
    if !header.is_empty()
        && (before.token_text(header[0]) != ":"
            || header.len() < 2
            || header.len() % 2 != 0
            || header[1..].iter().enumerate().any(|(i, token)| {
                if i % 2 == 0 {
                    token.kind != TokenKind::Word || !plain_identifier(before.token_text(token))
                } else {
                    before.token_text(token) != "::"
                }
            }))
    {
        return Err(invalid(
            "only an optional simple qualified type is supported in the part header",
        ));
    }
    Ok((token.range, name.into()))
}

fn prove_rename(
    before: &production::Document,
    selected: SyntaxNodeId,
    name: &str,
) -> Result<RenameProof, ServiceError> {
    if !before.is_complete()
        || before.sysml_profile() != Some(production::SysmlSyntaxProfile::OperationalV3)
    {
        return Err(invalid("complete pinned SysML source is required"));
    }
    if !plain_identifier(name) {
        return Err(invalid(
            "new name must be one ASCII identifier of at most 120 characters",
        ));
    }
    let (range, before_name) = name_token(before, selected)?;
    if before_name == name {
        return Err(invalid("the part already has this name"));
    }
    let edit = TextEdit {
        range,
        replacement: name.into(),
    };
    let after = before
        .edit(&edit, production::Limits::default())
        .map_err(|error| invalid(&error.to_string()))?;
    if !after.is_complete() {
        return Err(invalid(
            "renamed source does not parse completely; reserved words are not names",
        ));
    }
    let old = before.identity_checkpoint();
    let mut next = after.identity_checkpoint();
    if old.len() != next.len() {
        return Err(invalid("rename added or removed syntax productions"));
    }
    let mut index: BTreeMap<_, Vec<usize>> = BTreeMap::new();
    for (i, node) in next.iter().enumerate() {
        index
            .entry((
                node.production.as_str(),
                node.range.start(),
                node.range.end(),
            ))
            .or_default()
            .push(i);
    }
    let mut mapping = Vec::with_capacity(old.len());
    let mut used = BTreeSet::new();
    for node in &old {
        let range = mapped_range(node.range, &edit)?;
        let candidates = index
            .get(&(node.production.as_str(), range.start(), range.end()))
            .ok_or_else(|| invalid("existing production has no matching renamed production"))?;
        let [target] = candidates.as_slice() else {
            return Err(invalid("production mapping is ambiguous"));
        };
        if !used.insert(*target) {
            return Err(invalid("production mapping is not one-to-one"));
        }
        mapping.push(*target);
    }
    for (old_index, &new_index) in mapping.iter().enumerate() {
        let children: Vec<_> = old[old_index]
            .children
            .iter()
            .map(|child| mapping[*child])
            .collect();
        if children != next[new_index].children {
            return Err(invalid("rename changed child ownership or ordering"));
        }
    }
    let selected_index = old
        .iter()
        .position(|node| node.id == selected)
        .ok_or_else(|| invalid("selected production is missing"))?;
    if name_token(&after, next[mapping[selected_index]].id)?.1 != name {
        return Err(invalid(
            "replacement is not the selected declaration's plain name",
        ));
    }
    for (old_index, &new_index) in mapping.iter().enumerate() {
        next[new_index].id = old[old_index].id;
    }
    after
        .clone()
        .restore_identities(&next)
        .map_err(|error| invalid(&error.to_string()))?;
    Ok(RenameProof {
        parsed: after,
        identities: next,
        before_name,
        edit,
    })
}

/// Names may affect lookup and implied relationships. Refuse even a completely
/// reconstructed candidate if anything besides the selected name changed value.
/// Source origins, revision labels and evidence are freshly reconstructed normally.
fn verify_effective_rename(
    before: &WorkingProjectRevision,
    after: &WorkingProjectRevision,
    selected: ElementId,
) -> Result<(), ServiceError> {
    let old = before
        .semantic_model()
        .ok_or_else(|| invalid("predecessor effective graph unavailable"))?;
    let next = after
        .semantic_model()
        .ok_or_else(|| invalid("candidate effective graph unavailable"))?;
    if after
        .producer_closure()
        .is_none_or(|closure| !closure.is_fully_closed(next))
        || after
            .producer_status()
            .is_none_or(|status| !status.converged || status.completeness != Completeness::Complete)
    {
        return Err(invalid("complete semantic closure is required"));
    }
    verify_graph_values(old, next, selected)
}

fn verify_graph_values(
    old: &agq_kernel::ModelView,
    next: &agq_kernel::ModelView,
    selected: ElementId,
) -> Result<(), ServiceError> {
    if old.elements().count() != next.elements().count()
        || old.association_occurrences().count() != next.association_occurrences().count()
    {
        return Err(invalid("rename changed effective graph size"));
    }
    for record in old.elements() {
        let replacement = next
            .element(record.id())
            .ok_or_else(|| invalid("rename changed an effective identity"))?;
        let slots: BTreeMap<_, _> = record
            .slots()
            .filter(|(property, _)| {
                record.id() != selected || *property != properties::ELEMENT_DECLARED_NAME
            })
            .map(|(property, slot)| (property, slot.value()))
            .collect();
        let next_slots: BTreeMap<_, _> = replacement
            .slots()
            .filter(|(property, _)| {
                record.id() != selected || *property != properties::ELEMENT_DECLARED_NAME
            })
            .map(|(property, slot)| (property, slot.value()))
            .collect();
        if record.metaclass() != replacement.metaclass() || slots != next_slots {
            return Err(invalid(
                "rename changed effective semantics beyond the selected declared name",
            ));
        }
    }
    let occurrences: BTreeMap<_, _> = next
        .association_occurrences()
        .map(|occurrence| (occurrence.id(), occurrence))
        .collect();
    for occurrence in old.association_occurrences() {
        let next = occurrences
            .get(&occurrence.id())
            .ok_or_else(|| invalid("rename changed an effective relationship identity"))?;
        if occurrence.association() != next.association()
            || occurrence.ends() != next.ends()
            || occurrence.positions() != next.positions()
        {
            return Err(invalid(
                "rename changed an effective relationship or endpoint ordering",
            ));
        }
    }
    // Association-owned derived navigation is query-visible storage outside
    // element slots. Compare its exact key/value population while allowing the
    // ordinary reconstruction to rebuild origins and supporting evidence.
    if !old
        .derived_navigation_results()
        .map(|(key, slot)| (key, slot.value()))
        .eq(next
            .derived_navigation_results()
            .map(|(key, slot)| (key, slot.value())))
    {
        return Err(invalid("rename changed effective derived navigation"));
    }
    // Missing, incomplete and invalid are distinct from an empty computed value.
    // Explanation/search evidence is rebuilt, but failure kind and meaning must
    // remain identical at each exact element/property key.
    if old.computation_failures().count() != next.computation_failures().count()
        || !old
            .computation_failures()
            .zip(next.computation_failures())
            .all(|((key, failure), (next_key, next_failure))| {
                use agq_kernel::derived::ComputationFailure;
                key == next_key
                    && match (failure, next_failure) {
                        (
                            ComputationFailure::Incomplete { reason, .. },
                            ComputationFailure::Incomplete {
                                reason: next_reason,
                                ..
                            },
                        ) => reason == next_reason,
                        (
                            ComputationFailure::Invalid { diagnostic, .. },
                            ComputationFailure::Invalid {
                                diagnostic: next_diagnostic,
                                ..
                            },
                        ) => diagnostic == next_diagnostic,
                        _ => false,
                    }
            })
    {
        return Err(invalid("rename changed effective computation completeness"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(source: &str) -> production::Document {
        production::parse_sysml_with_profile(
            production::SysmlSyntaxProfile::OperationalV3,
            DocumentId::new(),
            SourceRevisionId::new(),
            source,
            production::Limits::default(),
        )
        .unwrap()
    }
    fn selected(syntax: &production::Document) -> SyntaxNodeId {
        syntax
            .nodes()
            .find(|node| matches!(node.kind(), P::PartDefinition | P::PartUsage))
            .unwrap_or_else(|| panic!("{}: {:?}", syntax.source(), syntax.diagnostics()))
            .id()
    }
    #[test]
    fn rename_preserves_complete_identity_arena_and_only_replaces_name() {
        for source in [
            "part def Platform;",
            "package System { part def Platform { part child; } part sibling; }",
            "part sensor : Devices::Sensor;",
            "part // header\n sensor { port observed; }",
        ] {
            let before = parse(source);
            let id = selected(&before);
            let proof = prove_rename(&before, id, "RenamedLonger")
                .unwrap_or_else(|e| panic!("{source}: {e}"));
            let restored = proof.parsed.restore_identities(&proof.identities).unwrap();
            assert_eq!(
                before
                    .nodes()
                    .map(|node| node.id())
                    .collect::<BTreeSet<_>>(),
                restored.nodes().map(|node| node.id()).collect()
            );
            assert_eq!(name_token(&restored, id).unwrap().1, "RenamedLonger");
            assert_eq!(
                &restored.source()[..proof.edit.range.start() as usize],
                &source[..proof.edit.range.start() as usize]
            );
            assert_eq!(
                &restored.source()
                    [proof.edit.range.start() as usize + proof.edit.replacement.len()..],
                &source[proof.edit.range.end() as usize..]
            );
            let second = prove_rename(&restored, id, "P").unwrap();
            let twice = second
                .parsed
                .restore_identities(&second.identities)
                .unwrap();
            assert_eq!(
                before
                    .nodes()
                    .map(|node| node.id())
                    .collect::<BTreeSet<_>>(),
                twice.nodes().map(|node| node.id()).collect()
            );
            assert_eq!(name_token(&twice, id).unwrap().1, "P");
        }
    }
    #[test]
    fn rename_rejects_complex_headers_wrong_identity_and_injected_names() {
        for source in [
            "part def <short> Platform;",
            "part 'quoted name';",
            "abstract part def Platform;",
            "part child : SomeType, OtherType;",
            "part child :> original;",
        ] {
            let before = parse(source);
            assert!(
                prove_rename(&before, selected(&before), "Renamed").is_err(),
                "{source}"
            );
        }
        let before = parse("part def Platform;");
        for name in [
            "Platform",
            "",
            "part",
            "new name",
            "first; part second",
            "<short> Name",
            "Name::Path",
            "Ångström",
        ] {
            assert!(
                prove_rename(&before, selected(&before), name).is_err(),
                "{name}"
            );
        }
        assert!(prove_rename(&before, SyntaxNodeId::new(), "Renamed").is_err());
    }

    #[test]
    fn rename_changes_only_selected_nested_declaration_with_repeated_names() {
        let before =
            parse("part def Platform { part repeated; part container { part repeated; } }");
        let selected = before
            .nodes()
            .filter(|node| node.kind() == P::PartUsage && node.text() == "part repeated;")
            .last()
            .unwrap()
            .id();
        let proof = prove_rename(&before, selected, "nestedRenamed").unwrap();
        let restored = proof.parsed.restore_identities(&proof.identities).unwrap();
        assert_eq!(name_token(&restored, selected).unwrap().1, "nestedRenamed");
        assert_eq!(
            restored.source(),
            "part def Platform { part repeated; part container { part nestedRenamed; } }"
        );
        assert_eq!(
            before
                .nodes()
                .map(|node| node.id())
                .collect::<BTreeSet<_>>(),
            restored.nodes().map(|node| node.id()).collect()
        );
    }

    #[test]
    fn effective_graph_guard_allows_only_selected_declared_name() {
        use agq_kernel::{
            Snapshot,
            value::{SlotValue, Value},
        };
        let base = Snapshot::new(Arc::new(
            agq_kerml::registry_for_profile(agq_kerml::BaselineProfile::OPERATIONAL_V9).unwrap(),
        ));
        let selected = ElementId::new();
        let sibling = ElementId::new();
        let origin = DeclaredOrigin::Authored { source: None };
        let make = |selected_name: &str, sibling_name: &str, extra: bool, abstract_value: bool| {
            let mut changes = base.change_set();
            for (id, name) in [(selected, selected_name), (sibling, sibling_name)] {
                changes.create(id, agq_kerml::classes::TYPE, origin.clone());
                changes.set(
                    id,
                    properties::ELEMENT_DECLARED_NAME,
                    SlotValue::Scalar(Value::String(name.into())),
                    origin.clone(),
                );
            }
            changes.set(
                selected,
                properties::TYPE_IS_ABSTRACT,
                SlotValue::Scalar(Value::Boolean(abstract_value)),
                origin.clone(),
            );
            if extra {
                changes.create(ElementId::new(), agq_kerml::classes::TYPE, origin.clone());
            }
            base.preview(&changes).unwrap()
        };
        let before = make("original", "sibling", false, false);
        let rename = make("renamed", "sibling", false, false);
        assert!(verify_graph_values(before.model(), rename.model(), selected).is_ok());
        for changed in [
            make("renamed", "otherSibling", false, false),
            make("renamed", "sibling", true, false),
            make("renamed", "sibling", false, true),
        ] {
            assert!(verify_graph_values(before.model(), changed.model(), selected).is_err());
        }
        assert!(verify_graph_values(before.model(), rename.model(), sibling).is_err());
    }

    // Non-normative kernel fixture: these association-owned values deliberately
    // have no element record slots or occurrences for the earlier guard to see.
    fn navigation_fixture() -> (agq_kernel::Snapshot, [ElementId; 3], agq_kernel::PropertyId) {
        use agq_kernel::{metamodel::*, *};
        let metamodel = MetamodelId::from_u128(1);
        let class = MetaclassId::from_u128(1);
        let association = AssociationId::from_u128(1);
        let left = PropertyId::from_u128(1);
        let right = PropertyId::from_u128(2);
        let registry = MetamodelRegistry::from_descriptors(DescriptorSet {
            models: vec![MetamodelDescriptor {
                id: metamodel,
                name: "Rename query-state guard fixture".into(),
                version: Version {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                uri: "urn:agentique:test:rename-query-state:1".into(),
            }],
            classes: vec![MetaclassDescriptor {
                id: class,
                name: "FixtureElement".into(),
                package: vec![],
                metamodel,
                direct_supertypes: BTreeSet::new(),
                is_abstract: false,
            }],
            associations: vec![AssociationDescriptor {
                id: association,
                name: "FixtureNavigation".into(),
                package: vec![],
                metamodel,
                member_ends: vec![left, right],
                navigable_owned_ends: BTreeSet::from([left]),
                direct_supertypes: BTreeSet::new(),
                is_abstract: false,
            }],
            properties: [(left, right), (right, left)]
                .into_iter()
                .map(|(id, opposite)| PropertyDescriptor {
                    id,
                    name: format!("end-{id}"),
                    owner: PropertyOwner::Association(association),
                    value_kind: ValueKind::Reference(class),
                    multiplicity: Multiplicity::MANY,
                    ordered: false,
                    unique: true,
                    derived: id == left,
                    composite: false,
                    redefines: BTreeSet::new(),
                    subsets: BTreeSet::new(),
                    derived_union: false,
                    association: Some(association),
                    opposite_ends: BTreeSet::from([opposite]),
                })
                .collect(),
            ..Default::default()
        })
        .unwrap();
        let base = Snapshot::new(Arc::new(registry));
        let ids = [
            ElementId::from_u128(1),
            ElementId::from_u128(2),
            ElementId::from_u128(3),
        ];
        let mut changes = base.change_set();
        for id in ids {
            changes.create(id, class, DeclaredOrigin::Authored { source: None });
        }
        (base.apply(&changes).unwrap(), ids, left)
    }

    #[test]
    fn effective_guard_rejects_navigation_value_and_presence_changes_without_record_changes() {
        use agq_kernel::{RuleId, derived::*, provenance::*, value::*};
        let (base, [subject, first, second], property) = navigation_fixture();
        let make = |targets: &[ElementId], rule| {
            let mut builder = DerivationBuilder::new(base.clone());
            builder.property(
                subject,
                property,
                SlotValue::Set(targets.iter().copied().map(Value::Reference).collect()),
                Explanation {
                    rule: RuleId::from_u128(rule),
                    dependencies: BTreeSet::new(),
                },
            );
            builder.build().unwrap()
        };
        let before = make(&[first], 1);
        let after = make(&[second], 1);
        assert!(
            before
                .model()
                .element(subject)
                .unwrap()
                .slot(property)
                .is_none()
        );
        assert!(
            after
                .model()
                .element(subject)
                .unwrap()
                .slot(property)
                .is_none()
        );
        assert_eq!(before.model().association_occurrences().count(), 0);
        assert_eq!(after.model().association_occurrences().count(), 0);
        assert!(verify_graph_values(before.model(), after.model(), subject).is_err());
        let empty = make(&[], 1);
        assert!(matches!(
            base.model().property_state(subject, property).unwrap(),
            PropertyState::NotComputed
        ));
        assert!(matches!(
            empty.model().property_state(subject, property).unwrap(),
            PropertyState::Computed(_)
        ));
        assert!(verify_graph_values(base.model(), empty.model(), subject).is_err());
        assert!(verify_graph_values(empty.model(), base.model(), subject).is_err());
        let rebuilt_evidence = make(&[first], 2);
        assert!(verify_graph_values(before.model(), rebuilt_evidence.model(), subject).is_ok());
    }

    #[test]
    fn effective_guard_rejects_failure_meaning_changes_but_allows_rebuilt_evidence() {
        use agq_kernel::{RuleId, derived::*, provenance::*};
        let (base, [subject, other, _], property) = navigation_fixture();
        let make = |element, reason, diagnostic: Option<&str>, rule| {
            let mut builder = DerivationBuilder::new(base.clone());
            let explanation = Explanation {
                rule: RuleId::from_u128(rule),
                dependencies: BTreeSet::new(),
            };
            let failure = match diagnostic {
                None => ComputationFailure::Incomplete {
                    reason,
                    explanation,
                    searches: BTreeSet::new(),
                },
                Some(diagnostic) => ComputationFailure::Invalid {
                    diagnostic: diagnostic.into(),
                    explanation,
                    searches: BTreeSet::new(),
                },
            };
            builder.failure(element, property, failure).unwrap();
            builder.build().unwrap()
        };
        let before = make(subject, IncompleteReason::MissingInput, None, 1);
        assert!(
            before
                .model()
                .element(subject)
                .unwrap()
                .slot(property)
                .is_none()
        );
        assert_eq!(before.model().association_occurrences().count(), 0);
        assert!(verify_graph_values(base.model(), before.model(), subject).is_err());
        assert!(verify_graph_values(before.model(), base.model(), subject).is_err());
        for changed in [
            make(subject, IncompleteReason::IncompleteDependency, None, 1),
            make(other, IncompleteReason::MissingInput, None, 1),
            make(
                subject,
                IncompleteReason::MissingInput,
                Some("invalid relationship"),
                1,
            ),
        ] {
            assert!(verify_graph_values(before.model(), changed.model(), subject).is_err());
        }
        let rebuilt_evidence = make(subject, IncompleteReason::MissingInput, None, 2);
        assert!(verify_graph_values(before.model(), rebuilt_evidence.model(), subject).is_ok());
        let invalid = make(
            subject,
            IncompleteReason::MissingInput,
            Some("invalid relationship"),
            1,
        );
        let different = make(
            subject,
            IncompleteReason::MissingInput,
            Some("different failure"),
            1,
        );
        assert!(verify_graph_values(invalid.model(), different.model(), subject).is_err());
    }
}
