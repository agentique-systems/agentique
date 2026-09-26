//! Service-verified identity continuity for one appended authored PartUsage.
//! Caller text is input; callers cannot supply a checkpoint or an identity map.
use super::*;
use agq_kerml_semantics::Completeness;
use agq_kerml_syntax::{
    TextEdit, TokenKind,
    production::{self, NodeIdentity, Production as P},
};
use agq_kernel::{
    ElementId, SyntaxNodeId,
    provenance::{ByteRange, FactKey},
};
use std::collections::BTreeSet;

use super::source_identity::{self, Mutation, mapped_range};

const POLICY: &str = "agentique-source-identity/part-insertion/1";

fn invalid(reason: &str) -> ServiceError {
    ServiceError::Invalid(format!("Part insertion identity proof: {reason}"))
}

struct InsertionProof {
    parsed: production::Document,
    identities: Vec<NodeIdentity>,
    added_part: SyntaxNodeId,
    part_name: String,
    retained: usize,
}

impl ModelingService {
    /// Prepare one nested PartUsage while proving continuity of all previous
    /// syntax declarations. Full source reconstruction is intentional; this is
    /// an identity/correctness path, not an incremental-performance claim.
    ///
    /// Only one edit at the selected authored Part's closing brace/semicolon is
    /// permitted. The service derives the identity mapping itself and checks
    /// canonical identities, ownership and previous reference targets afterward.
    pub fn prepare_part_insertion(
        &self,
        command: ApplyDocumentChanges,
        owner: ElementId,
    ) -> Result<PreparedChanges, ServiceError> {
        self.prepare_part_insertion_controlled(command, owner, &CompilationControl::default())
    }

    /// Prepare the same source-proven nested part with cancellation checkpoints.
    /// Interruption returns no prepared child and never changes durable head.
    pub fn prepare_part_insertion_controlled(
        &self,
        command: ApplyDocumentChanges,
        owner: ElementId,
        control: &CompilationControl,
    ) -> Result<PreparedChanges, ServiceError> {
        control.check()?;
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
        let target = base.current_element(owner)?;
        if !matches!(
            target.metaclass_name.as_str(),
            "PartDefinition" | "PartUsage"
        ) {
            return Err(invalid("owner must be an authored Part"));
        }
        let origin = target
            .source
            .ok_or_else(|| invalid("owner has no authored source"))?;
        let [ProjectChange::Edit { document, edit }] = command.changes.as_slice() else {
            return Err(invalid("exactly one document edit is required"));
        };
        if *document != origin.document {
            return Err(invalid("edit belongs to another document"));
        }
        let source = base
            .revision()
            .document(*document)
            .ok_or_else(|| invalid("standard objects are read-only"))?;
        if source.revision() != origin.revision {
            return Err(invalid("source provenance is stale"));
        }
        let syntax = source
            .production_syntax()
            .ok_or_else(|| invalid("lossless syntax is unavailable"))?;
        let owner_node = syntax
            .nodes()
            .find(|node| Some(node.id()) == origin.syntax_node)
            .filter(|node| {
                node.range() == origin.range && node.kind().name() == target.metaclass_name
            })
            .ok_or_else(|| invalid("owner declaration provenance does not match"))?;
        control.enter(CompilationStage::Parsing)?;
        let proof = prove_insertion(syntax, owner_node.id(), edit)?;
        control.check()?;
        let (working, checkpoint) = source_identity::reconstruct(
            base.revision(),
            &proof.parsed,
            proof.identities,
            control,
        )?;
        verify_continuity(
            base.revision(),
            &working,
            owner,
            *document,
            proof.added_part,
        )?;
        let evidence = serde_json::json!({
            "policy": POLICY,
            "mode": "full-source-reconstruction",
            "base_revision": command.expected_head,
            "owner": owner,
            "document": document,
            "before_source_revision": origin.revision,
            "after_source_revision": proof.parsed.revision(),
            "before_source_sha256": ContentDigest::of(source.source().as_bytes()),
            "after_source_sha256": ContentDigest::of(proof.parsed.source().as_bytes()),
            "edit_range": edit.range,
            "replacement_sha256": ContentDigest::of(edit.replacement.as_bytes()),
            "retained_syntax_nodes": proof.retained,
            "added_part_syntax_node": proof.added_part,
            "before_arena_sha256": ContentDigest::of(&serde_json::to_vec(&syntax.identity_checkpoint())?),
            "after_arena_sha256": ContentDigest::of(&serde_json::to_vec(&checkpoint.source.documents.iter().find(|saved| saved.document_id == *document).expect("checked source").syntax_nodes)?),
            "retired_identity_reservations_preserved": true,
            "previous_reference_targets_preserved": true,
        });
        control.enter(CompilationStage::PreparingReview)?;
        let mut candidate = prepare_candidate(&working, command.validate)?;
        control.check()?;
        let owner_name = target
            .declared_qualified_name
            .as_deref()
            .and_then(|name| name.rsplit("::").next())
            .unwrap_or("selected part");
        candidate.manifest.metadata.name = Some(format!("Add {} to {owner_name}", proof.part_name));
        candidate.manifest.metadata.description = Some(serde_json::to_string(&evidence)?);
        candidate.manifest.metadata.alias.push(POLICY.into());
        candidate.verify()?;
        let validated = if command.validate {
            Some(working.validate()?)
        } else {
            None
        };
        control.check()?;
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

fn prove_insertion(
    before: &production::Document,
    owner: SyntaxNodeId,
    edit: &TextEdit,
) -> Result<InsertionProof, ServiceError> {
    if !before.is_complete()
        || before.sysml_profile() != Some(production::SysmlSyntaxProfile::OperationalV3)
    {
        return Err(invalid("complete pinned SysML source is required"));
    }
    let selected = before
        .nodes()
        .find(|node| node.id() == owner)
        .filter(|node| matches!(node.kind(), P::PartDefinition | P::PartUsage))
        .ok_or_else(|| invalid("selected syntax is not a Part declaration"))?;
    let last = selected
        .tokens()
        .filter(|token| !token.kind.is_trivia() && token.kind != TokenKind::Comment)
        .last()
        .ok_or_else(|| invalid("selected declaration is empty"))?;
    let insertion = match before.token_text(last) {
        "}" if edit.range.start() == last.range.start()
            && edit.range.end() == last.range.start() =>
        {
            true
        }
        ";" if edit.range == last.range => false,
        _ => return Err(invalid("edit must append at the owner's body boundary")),
    };
    let after = before
        .edit(edit, production::Limits::default())
        .map_err(|error| invalid(&error.to_string()))?;
    if !after.is_complete() {
        return Err(invalid("insertion does not parse completely"));
    }
    let old = before.identity_checkpoint();
    let mut next = after.identity_checkpoint();
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
        let mut range = mapped_range(node.range, edit)?;
        // An empty body's production spans its semicolon token. Replacing that
        // token with a braced body can introduce leading/trailing trivia, which
        // the parser does not include in this production's range. Only the
        // exact replaced token gets this adjustment; declaration headers and
        // all other productions still use the ordinary byte-splice mapping.
        if !insertion && node.range == edit.range {
            let leading = edit.replacement.len() - edit.replacement.trim_start().len();
            let trimmed = edit.replacement.trim();
            if !trimmed.starts_with('{') || !trimmed.ends_with('}') {
                return Err(invalid("semicolon must become one braced body"));
            }
            range = ByteRange::new(
                edit.range.start() + leading as u64,
                edit.range.start() + leading as u64 + trimmed.len() as u64,
            )
            .map_err(|_| invalid("replacement body range is invalid"))?;
        }
        let candidates = index
            .get(&(node.production.as_str(), range.start(), range.end()))
            .ok_or_else(|| {
                invalid(&format!(
                    "existing {} at {:?} has no unique matching production at {:?}",
                    node.production, node.range, range
                ))
            })?;
        let [target] = candidates.as_slice() else {
            return Err(invalid("production mapping is ambiguous"));
        };
        if !used.insert(*target) {
            return Err(invalid("production mapping is not one-to-one"));
        }
        mapping.push(*target);
    }
    for (old_index, &new_index) in mapping.iter().enumerate() {
        let old_children: Vec<_> = old[old_index]
            .children
            .iter()
            .map(|child| mapping[*child])
            .collect();
        let new_children: Vec<_> = next[new_index]
            .children
            .iter()
            .filter(|child| used.contains(child))
            .copied()
            .collect();
        if old_children != new_children {
            return Err(invalid("existing child ownership or order changed"));
        }
    }
    let inserted = ByteRange::new(
        edit.range.start(),
        edit.range.start() + edit.replacement.len() as u64,
    )
    .map_err(|_| invalid("insertion range is invalid"))?;
    let mut parts = Vec::new();
    for (i, node) in next.iter().enumerate().filter(|(i, _)| !used.contains(i)) {
        if node.range.start() < inserted.start() || node.range.end() > inserted.end() {
            return Err(invalid("new production escapes the inserted source"));
        }
        if node.production == "PartUsage" {
            parts.push(i);
        }
    }
    let [part] = parts.as_slice() else {
        return Err(invalid("exactly one new PartUsage is required"));
    };
    let added_part = next[*part].id;
    let part = after
        .nodes()
        .find(|node| node.id() == added_part)
        .expect("new part in parsed arena");
    let tokens: Vec<_> = part
        .tokens()
        .filter(|token| !token.kind.is_trivia() && token.kind != TokenKind::Comment)
        .collect();
    // The insertion is one plain named part, optionally typed. No nested body,
    // metadata, relationship declaration or second statement hides in its range.
    let texts: Vec<_> = tokens.iter().map(|token| after.token_text(token)).collect();
    let names: Vec<_> = part.names().collect();
    if texts.first() != Some(&"part")
        || texts.last() != Some(&";")
        || names.is_empty()
        || tokens
            .get(1)
            .is_none_or(|token| token.kind != TokenKind::Word)
        || texts.iter().filter(|&&text| text == ";").count() != 1
        || texts
            .iter()
            .any(|text| matches!(*text, "{" | "}" | "<" | ">"))
    {
        return Err(invalid("only one plain named PartUsage is supported"));
    }
    if texts.len() != 3
        && (texts.get(2) != Some(&":")
            || tokens[3..tokens.len() - 1]
                .iter()
                .enumerate()
                .any(|(i, token)| {
                    if i % 2 == 0 {
                        token.kind != TokenKind::Word
                    } else {
                        after.token_text(token) != "::"
                    }
                })
            || (tokens.len() - 4) % 2 != 1)
    {
        return Err(invalid("part typing must be one simple qualified name"));
    }
    let outside_part = format!(
        "{}{}",
        &after.source()[inserted.start() as usize..part.range().start() as usize],
        &after.source()[part.range().end() as usize..inserted.end() as usize]
    );
    let expected_wrapper = if insertion { "" } else { "{}" };
    if outside_part
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        != expected_wrapper
    {
        return Err(invalid(
            "insertion contains text outside the single part declaration",
        ));
    }
    let part_name = texts[1].to_owned();
    for (old_index, &new_index) in mapping.iter().enumerate() {
        next[new_index].id = old[old_index].id;
    }
    if next
        .iter()
        .map(|node| node.id)
        .collect::<BTreeSet<_>>()
        .len()
        != next.len()
    {
        return Err(invalid("new syntax reuses an existing identity"));
    }
    // Existing shape checker independently rechecks the complete mapped arena.
    after
        .clone()
        .restore_identities(&next)
        .map_err(|error| invalid(&error.to_string()))?;
    Ok(InsertionProof {
        parsed: after,
        identities: next,
        added_part,
        part_name,
        retained: old.len(),
    })
}

fn verify_continuity(
    before: &WorkingProjectRevision,
    after: &WorkingProjectRevision,
    owner: ElementId,
    document: DocumentId,
    added_part: SyntaxNodeId,
) -> Result<(), ServiceError> {
    source_identity::verify_existing(before, after, Mutation::AppendMember { owner })?;
    let next = after
        .strict_snapshot()
        .ok_or_else(|| invalid("candidate declared graph unavailable"))?
        .model();
    let queries = after
        .kerml_queries()
        .map_err(|error| invalid(&format!("candidate query: {error:?}")))?;
    let added: Vec<_> = next
        .elements()
        .filter(|record| {
            next.registry()
                .class(record.metaclass())
                .is_ok_and(|class| class.name == "PartUsage")
                && after
                    .source_for_fact(FactKey::Element(record.id()))
                    .is_some_and(|origin| {
                        origin.document == document && origin.syntax_node == Some(added_part)
                    })
        })
        .map(|record| record.id())
        .collect();
    let [added] = added.as_slice() else {
        return Err(invalid("new source has no unique canonical PartUsage"));
    };
    let added_owner = queries.owner(*added);
    if added_owner.completeness != Completeness::Complete || added_owner.value != Some(owner) {
        return Err(invalid(
            "new PartUsage does not belong to the original selected owner",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> production::Document {
        let syntax = production::parse_sysml_with_profile(
            production::SysmlSyntaxProfile::OperationalV3,
            DocumentId::new(),
            SourceRevisionId::new(),
            source,
            production::Limits::default(),
        )
        .unwrap();
        assert!(syntax.is_complete(), "{source}: {:?}", syntax.diagnostics());
        syntax
    }

    fn edit_for(syntax: &production::Document, owner: SyntaxNodeId, addition: &str) -> TextEdit {
        let node = syntax.nodes().find(|node| node.id() == owner).unwrap();
        let last = node
            .tokens()
            .filter(|t| !t.kind.is_trivia() && t.kind != TokenKind::Comment)
            .last()
            .unwrap();
        match syntax.token_text(last) {
            "}" => TextEdit {
                range: ByteRange::new(last.range.start(), last.range.start()).unwrap(),
                replacement: format!("\n    {addition}\n"),
            },
            ";" => TextEdit {
                range: last.range,
                replacement: format!(" {{\n    {addition}\n}}"),
            },
            _ => panic!("fixture body"),
        }
    }

    #[test]
    fn proof_preserves_every_old_identity_for_braces_and_semicolon_bodies() {
        for source in [
            "part def Platform;",
            "part Platform;",
            "package System { part def Platform { part retained; } part sibling; }",
            "package System { part def Platform {\r\n    // } is a comment\r\n    part 'élan';\r\n} }",
            "package System { part <short> Platform : Definition; }",
        ] {
            let syntax = parse(source);
            let owner = syntax
                .nodes()
                .find(|node| matches!(node.kind(), P::PartDefinition | P::PartUsage))
                .unwrap()
                .id();
            let edit = edit_for(&syntax, owner, "part child;");
            let proof =
                prove_insertion(&syntax, owner, &edit).unwrap_or_else(|e| panic!("{source}: {e}"));
            let reconstructed = proof.parsed.restore_identities(&proof.identities).unwrap();
            let ids: BTreeSet<_> = reconstructed.nodes().map(|node| node.id()).collect();
            assert!(syntax.nodes().all(|node| ids.contains(&node.id())));
            assert!(!syntax.nodes().any(|node| node.id() == proof.added_part));
            assert!(ids.contains(&proof.added_part));
            assert_eq!(proof.retained, syntax.nodes().count());
            assert_eq!(
                &reconstructed.source()[..edit.range.start() as usize],
                &source[..edit.range.start() as usize]
            );
            assert_eq!(
                &reconstructed.source()[edit.range.start() as usize + edit.replacement.len()..],
                &source[edit.range.end() as usize..]
            );
        }
    }

    #[test]
    fn proof_preserves_restored_identity_arena_across_two_insertions() {
        let before = parse("package System { part def Platform; part sibling; }");
        let owner = before
            .nodes()
            .find(|node| node.kind() == P::PartDefinition)
            .unwrap()
            .id();
        let first = prove_insertion(
            &before,
            owner,
            &edit_for(&before, owner, "part firstChild;"),
        )
        .unwrap();
        let first_part = first.added_part;
        let restored = first.parsed.restore_identities(&first.identities).unwrap();
        let second = prove_insertion(
            &restored,
            owner,
            &edit_for(&restored, owner, "part secondChild;"),
        )
        .unwrap();
        let second_part = second.added_part;
        let twice = second
            .parsed
            .restore_identities(&second.identities)
            .unwrap();
        let ids: BTreeSet<_> = twice.nodes().map(|node| node.id()).collect();
        assert!(restored.nodes().all(|node| ids.contains(&node.id())));
        assert!(ids.contains(&owner));
        assert!(ids.contains(&first_part));
        assert!(ids.contains(&second_part));
        assert_ne!(first_part, second_part);
        let owner_node = twice.nodes().find(|node| node.id() == owner).unwrap();
        assert!(owner_node.text().contains("part firstChild;"));
        assert!(owner_node.text().contains("part secondChild;"));
    }

    #[test]
    fn proof_accepts_one_qualified_typing_but_no_hidden_declarations() {
        let syntax = parse("part def Platform { part retained; }");
        let owner = syntax
            .nodes()
            .find(|node| node.kind() == P::PartDefinition)
            .unwrap()
            .id();
        for addition in [
            "part child;",
            "part child : SomeType;",
            "part child : System::SomeType;",
        ] {
            assert!(
                prove_insertion(&syntax, owner, &edit_for(&syntax, owner, addition)).is_ok(),
                "{addition}"
            );
        }
        for addition in [
            "part child; part second;",
            "port child;",
            "part child { part nested; }",
            "part child { port nested; }",
            "part <alias> child;",
            "part child : SomeType, OtherType;",
            "part child; } part escaped {",
        ] {
            assert!(
                prove_insertion(&syntax, owner, &edit_for(&syntax, owner, addition)).is_err(),
                "{addition}"
            );
        }
    }

    #[test]
    fn proof_rejects_header_rename_wrong_owner_and_other_source_changes() {
        let syntax = parse("part def Platform { part retained; }");
        let owner = syntax
            .nodes()
            .find(|node| node.kind() == P::PartDefinition)
            .unwrap()
            .id();
        let child = syntax
            .nodes()
            .find(|node| node.kind() == P::PartUsage)
            .unwrap()
            .id();
        let start = syntax.source().find("Platform").unwrap() as u64;
        let rename = TextEdit {
            range: ByteRange::new(start, start + 8).unwrap(),
            replacement: "Replaced".into(),
        };
        assert!(prove_insertion(&syntax, owner, &rename).is_err());
        let append = edit_for(&syntax, owner, "part child;");
        assert!(prove_insertion(&syntax, child, &append).is_err());
        assert!(prove_insertion(&syntax, SyntaxNodeId::new(), &append).is_err());
        let wipe = TextEdit {
            range: ByteRange::new(0, syntax.source().len() as u64).unwrap(),
            replacement: "part def Platform { part child; }".into(),
        };
        assert!(prove_insertion(&syntax, owner, &wipe).is_err());
    }
}
